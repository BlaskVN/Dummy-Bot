use anyhow::{Result, bail};
use poise::serenity_prelude::{GuildId, UserId};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationAction {
    Warn,
    Kick,
    Ban,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModerationIntent {
    Warn,
    Kick,
    Ban { delete_message_days: Option<u8> },
    Timeout { duration: std::time::Duration },
}

impl ModerationIntent {
    pub fn action(&self) -> ModerationAction {
        match self {
            Self::Warn => ModerationAction::Warn,
            Self::Kick => ModerationAction::Kick,
            Self::Ban { .. } => ModerationAction::Ban,
            Self::Timeout { .. } => ModerationAction::Timeout,
        }
    }

    pub fn action_translation_key(&self) -> crate::i18n::TranslationKey {
        match self {
            Self::Warn => crate::i18n::TranslationKey::ModerationActionWarn,
            Self::Kick => crate::i18n::TranslationKey::ModerationActionKick,
            Self::Ban { .. } => crate::i18n::TranslationKey::ModerationActionBan,
            Self::Timeout { .. } => crate::i18n::TranslationKey::ModerationActionTimeout,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutedCase {
    pub case_number: i64,
    pub summary_text: String,
    pub channel_logged: bool,
}

#[derive(Debug)]
pub enum ModerationExecutionError {
    Denial(crate::permissions::ModerationDenial),
    EmptyReason,
    InvalidEvidence,
    DiscordFailed(anyhow::Error),
    DatabaseFailedAfterDiscordAction { source: anyhow::Error },
    Other(anyhow::Error),
}

impl std::fmt::Display for ModerationExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Denial(d) => write!(f, "Moderation action denied: {:?}", d),
            Self::EmptyReason => write!(f, "Moderation case reason cannot be empty"),
            Self::InvalidEvidence => write!(f, "Invalid evidence URL"),
            Self::DiscordFailed(e) => write!(f, "Discord action failed: {}", e),
            Self::DatabaseFailedAfterDiscordAction { source } => {
                write!(
                    f,
                    "Discord action succeeded, but case recording failed: {}",
                    source
                )
            }
            Self::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ModerationExecutionError {}

impl From<anyhow::Error> for ModerationExecutionError {
    fn from(err: anyhow::Error) -> Self {
        Self::Other(err)
    }
}

pub trait DiscordModerationExecutor: Send + Sync {
    fn execute_action(
        &self,
        guild_id: GuildId,
        target: UserId,
        intent: &ModerationIntent,
        reason: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;

    fn send_moderation_channel_log(
        &self,
        channel_id: poise::serenity_prelude::ChannelId,
        summary: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub struct SerenityDiscordExecutor<'a> {
    pub http: &'a poise::serenity_prelude::Http,
}

impl<'a> SerenityDiscordExecutor<'a> {
    pub fn new(http: &'a poise::serenity_prelude::Http) -> Self {
        Self { http }
    }
}

impl DiscordModerationExecutor for SerenityDiscordExecutor<'_> {
    async fn execute_action(
        &self,
        guild_id: GuildId,
        target: UserId,
        intent: &ModerationIntent,
        reason: &str,
    ) -> anyhow::Result<()> {
        match intent {
            ModerationIntent::Warn => Ok(()),
            ModerationIntent::Kick => {
                guild_id.kick_with_reason(self.http, target, reason).await?;
                Ok(())
            }
            ModerationIntent::Ban {
                delete_message_days,
            } => {
                guild_id
                    .ban_with_reason(self.http, target, delete_message_days.unwrap_or(0), reason)
                    .await?;
                Ok(())
            }
            ModerationIntent::Timeout { duration } => {
                let until = poise::serenity_prelude::Timestamp::from_unix_timestamp(
                    chrono::Utc::now().timestamp() + duration.as_secs() as i64,
                )?;
                let edit = poise::serenity_prelude::EditMember::new()
                    .disable_communication_until_datetime(until);
                guild_id.edit_member(self.http, target, edit).await?;
                Ok(())
            }
        }
    }

    async fn send_moderation_channel_log(
        &self,
        channel_id: poise::serenity_prelude::ChannelId,
        summary: &str,
    ) -> anyhow::Result<()> {
        channel_id
            .send_message(
                self.http,
                poise::serenity_prelude::CreateMessage::new().content(summary),
            )
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ModerationRequest<'a> {
    pub guild_id: GuildId,
    pub target: UserId,
    pub moderator: UserId,
    pub intent: ModerationIntent,
    pub reason: &'a str,
    pub evidence_url: Option<&'a str>,
    pub language: crate::i18n::Language,
    pub denial: Option<crate::permissions::ModerationDenial>,
}

pub async fn execute_moderation_action<E: DiscordModerationExecutor>(
    executor: &E,
    pool: &SqlitePool,
    request: ModerationRequest<'_>,
) -> Result<ExecutedCase, ModerationExecutionError> {
    if let Some(denial) = request.denial {
        return Err(ModerationExecutionError::Denial(denial));
    }
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err(ModerationExecutionError::EmptyReason);
    }
    if let Some(url) = request.evidence_url
        && !valid_evidence_url(url, request.guild_id)
    {
        return Err(ModerationExecutionError::InvalidEvidence);
    }

    if let Err(err) = executor
        .execute_action(request.guild_id, request.target, &request.intent, reason)
        .await
    {
        return Err(ModerationExecutionError::DiscordFailed(err));
    }

    let action = request.intent.action();
    let case_number = match create_case(
        pool,
        request.guild_id,
        action,
        request.target,
        request.moderator,
        reason,
        request.evidence_url,
    )
    .await
    {
        Ok(num) => num,
        Err(err) => {
            tracing::error!(
                guild_id = %request.guild_id,
                target = %request.target,
                moderator = %request.moderator,
                intent = ?request.intent,
                error = %err,
                "Discord action succeeded but moderation case creation failed"
            );
            return Err(ModerationExecutionError::DatabaseFailedAfterDiscordAction { source: err });
        }
    };

    let action_key = request.intent.action_translation_key();
    let summary_text = crate::i18n::tf(
        request.language,
        crate::i18n::TranslationKey::ModerationCaseSummary,
        &[
            &case_number,
            &crate::i18n::t(request.language, action_key),
            &request.target,
            &request.moderator,
            &reason,
        ],
    );

    let mut channel_logged = false;
    if let Ok(Some(channel)) = sqlx::query_scalar::<_, String>(
        "SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?",
    )
    .bind(request.guild_id.to_string())
    .fetch_optional(pool)
    .await
        && let Ok(channel_id) = channel.parse::<u64>()
    {
        let ch = poise::serenity_prelude::ChannelId::new(channel_id);
        if let Err(err) = executor
            .send_moderation_channel_log(ch, &summary_text)
            .await
        {
            tracing::warn!(
                guild_id = %request.guild_id,
                %channel_id,
                error = %err,
                "Failed to deliver moderation case summary to moderation channel"
            );
        } else {
            channel_logged = true;
        }
    }

    Ok(ExecutedCase {
        case_number,
        summary_text,
        channel_logged,
    })
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ModerationCaseRecord {
    pub case_number: i64,
    pub action: String,
    pub target_user_id: String,
    pub moderator_user_id: String,
    pub reason: String,
    pub evidence_url: Option<String>,
    pub status: String,
    pub created_at: String,
    pub void_actor_user_id: Option<String>,
    pub void_reason: Option<String>,
    pub voided_at: Option<String>,
}

impl ModerationAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Warn => "warn",
            Self::Kick => "kick",
            Self::Ban => "ban",
            Self::Timeout => "timeout",
        }
    }
}

pub fn valid_evidence_url(url: &str, guild_id: GuildId) -> bool {
    let Some(path) = url.strip_prefix("https://discord.com/channels/") else {
        return false;
    };
    let parts: Vec<_> = path.split('/').collect();
    parts.len() == 3
        && parts[0].parse::<u64>() == Ok(guild_id.get())
        && parts[1].parse::<u64>().is_ok_and(|id| id > 0)
        && parts[2].parse::<u64>().is_ok_and(|id| id > 0)
}

pub async fn create_case(
    pool: &SqlitePool,
    guild_id: GuildId,
    action: ModerationAction,
    target: UserId,
    moderator: UserId,
    reason: &str,
    evidence_url: Option<&str>,
) -> Result<i64> {
    if reason.trim().is_empty() {
        bail!("Moderation case reason cannot be empty");
    }
    if evidence_url.is_some_and(|url| !valid_evidence_url(url, guild_id)) {
        bail!("Evidence must be a Discord message URL from this guild");
    }

    let mut transaction = pool.begin().await?;
    let case_number: i64 = sqlx::query_scalar(
        "INSERT INTO moderation_case_counter (guild_id, last_number) VALUES (?, 1)\n         ON CONFLICT(guild_id) DO UPDATE SET last_number = last_number + 1\n         RETURNING last_number",
    )
    .bind(guild_id.to_string())
    .fetch_one(&mut *transaction)
    .await?;
    sqlx::query(
        "INSERT INTO moderation_case (guild_id, case_number, action, target_user_id, moderator_user_id, reason, evidence_url) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(guild_id.to_string())
    .bind(case_number)
    .bind(action.as_str())
    .bind(target.to_string())
    .bind(moderator.to_string())
    .bind(reason)
    .bind(evidence_url)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(case_number)
}

pub async fn void_case(
    pool: &SqlitePool,
    guild_id: GuildId,
    case_number: i64,
    actor: UserId,
    reason: &str,
) -> Result<bool> {
    if reason.trim().is_empty() {
        bail!("Void reason cannot be empty");
    }
    Ok(sqlx::query(
        "UPDATE moderation_case SET status = 'voided', void_actor_user_id = ?, void_reason = ?, voided_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND case_number = ? AND status = 'active'",
    )
    .bind(actor.to_string())
    .bind(reason)
    .bind(guild_id.to_string())
    .bind(case_number)
    .execute(pool)
    .await?
    .rows_affected()
        == 1)
}

pub async fn get_case(
    pool: &SqlitePool,
    guild_id: GuildId,
    case_number: i64,
) -> Result<Option<ModerationCaseRecord>> {
    Ok(sqlx::query_as::<_, ModerationCaseRecord>(
        "SELECT case_number, action, target_user_id, moderator_user_id, reason, evidence_url, status, created_at, void_actor_user_id, void_reason, voided_at FROM moderation_case WHERE guild_id = ? AND case_number = ?",
    )
    .bind(guild_id.to_string())
    .bind(case_number)
    .fetch_optional(pool)
    .await?)
}

pub async fn list_cases(
    pool: &SqlitePool,
    guild_id: GuildId,
    target: Option<UserId>,
    offset: i64,
    limit: i64,
) -> Result<Vec<ModerationCaseRecord>> {
    if let Some(target) = target {
        return Ok(sqlx::query_as::<_, ModerationCaseRecord>(
            "SELECT case_number, action, target_user_id, moderator_user_id, reason, evidence_url, status, created_at, void_actor_user_id, void_reason, voided_at FROM moderation_case WHERE guild_id = ? AND target_user_id = ? ORDER BY case_number DESC LIMIT ? OFFSET ?",
        )
        .bind(guild_id.to_string())
        .bind(target.to_string())
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?);
    }
    Ok(sqlx::query_as::<_, ModerationCaseRecord>(
        "SELECT case_number, action, target_user_id, moderator_user_id, reason, evidence_url, status, created_at, void_actor_user_id, void_reason, voided_at FROM moderation_case WHERE guild_id = ? ORDER BY case_number DESC LIMIT ? OFFSET ?",
    )
    .bind(guild_id.to_string())
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?)
}

#[cfg(test)]
mod tests {
    use super::{
        DiscordModerationExecutor, ModerationAction, ModerationExecutionError, ModerationIntent,
        ModerationRequest, create_case, execute_moderation_action, get_case, list_cases,
        valid_evidence_url, void_case,
    };
    use crate::database::init_db;
    use poise::serenity_prelude::{GuildId, UserId};

    #[test]
    fn validates_discord_evidence_for_current_guild() {
        let guild = GuildId::new(1);
        assert!(valid_evidence_url(
            "https://discord.com/channels/1/2/3",
            guild
        ));
        assert!(!valid_evidence_url(
            "https://discord.com/channels/9/2/3",
            guild
        ));
        assert!(!valid_evidence_url(
            "https://example.com/channels/1/2/3",
            guild
        ));
        assert!(!valid_evidence_url(
            "https://discord.com/channels/1/2/3/4",
            guild
        ));
    }

    #[tokio::test]
    async fn numbers_cases_per_guild_and_voids_once() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-case-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        let guild = GuildId::new(1);
        assert_eq!(
            create_case(
                &pool,
                guild,
                ModerationAction::Warn,
                UserId::new(2),
                UserId::new(3),
                "reason",
                Some("https://discord.com/channels/1/2/3")
            )
            .await
            .unwrap(),
            1
        );
        assert!(get_case(&pool, guild, 1).await.unwrap().is_some());
        assert!(get_case(&pool, GuildId::new(9), 2).await.unwrap().is_none());
        assert_eq!(
            create_case(
                &pool,
                guild,
                ModerationAction::Kick,
                UserId::new(4),
                UserId::new(3),
                "other",
                None
            )
            .await
            .unwrap(),
            2
        );
        assert_eq!(
            create_case(
                &pool,
                GuildId::new(9),
                ModerationAction::Ban,
                UserId::new(4),
                UserId::new(3),
                "other",
                None
            )
            .await
            .unwrap(),
            1
        );
        assert!(
            void_case(&pool, guild, 1, UserId::new(5), "entered by mistake")
                .await
                .unwrap()
        );
        assert!(
            !void_case(&pool, guild, 1, UserId::new(6), "second attempt")
                .await
                .unwrap()
        );
        let row: (String, String, String, String) = sqlx::query_as(
            "SELECT action, reason, status, void_actor_user_id FROM moderation_case WHERE guild_id = '1' AND case_number = 1",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            row,
            ("warn".into(), "reason".into(), "voided".into(), "5".into())
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn allocates_unique_numbers_concurrently() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-concurrent-case-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        let mut tasks = tokio::task::JoinSet::new();
        for target in 10..18 {
            let pool = pool.clone();
            tasks.spawn(async move {
                create_case(
                    &pool,
                    GuildId::new(1),
                    ModerationAction::Timeout,
                    UserId::new(target),
                    UserId::new(2),
                    "reason",
                    None,
                )
                .await
                .unwrap()
            });
        }
        let mut numbers = Vec::new();
        while let Some(result) = tasks.join_next().await {
            numbers.push(result.unwrap());
        }
        numbers.sort_unstable();
        assert_eq!(numbers, (1..=8).collect::<Vec<_>>());
        assert_eq!(
            list_cases(&pool, GuildId::new(1), None, 0, 3)
                .await
                .unwrap()
                .into_iter()
                .map(|record| record.case_number)
                .collect::<Vec<_>>(),
            vec![8, 7, 6]
        );
        assert_eq!(
            list_cases(&pool, GuildId::new(1), None, 3, 3)
                .await
                .unwrap()
                .into_iter()
                .map(|record| record.case_number)
                .collect::<Vec<_>>(),
            vec![5, 4, 3]
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn migrates_a_v1_database_without_losing_settings() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-case-migration-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let pool =
            sqlx::SqlitePool::connect(&format!("sqlite:{}/bot.db?mode=rwc", directory.display()))
                .await
                .unwrap();
        sqlx::raw_sql(include_str!("../migrations/0001_initial.sql"))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO guild_language (guild_id, language) VALUES ('1', 'vi')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let language: String =
            sqlx::query_scalar("SELECT language FROM guild_language WHERE guild_id = '1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(language, "vi");
        assert_eq!(
            create_case(
                &pool,
                GuildId::new(1),
                ModerationAction::Warn,
                UserId::new(2),
                UserId::new(3),
                "reason",
                None
            )
            .await
            .unwrap(),
            1
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    type ActionRecord = (GuildId, UserId, ModerationIntent, String);
    type LogRecord = (poise::serenity_prelude::ChannelId, String);

    struct MockDiscordExecutor {
        executed: std::sync::Arc<tokio::sync::Mutex<Vec<ActionRecord>>>,
        logged: std::sync::Arc<tokio::sync::Mutex<Vec<LogRecord>>>,
        fail_action: bool,
    }

    impl MockDiscordExecutor {
        fn new(fail_action: bool) -> Self {
            Self {
                executed: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
                logged: std::sync::Arc::new(tokio::sync::Mutex::new(Vec::new())),
                fail_action,
            }
        }
    }

    impl DiscordModerationExecutor for MockDiscordExecutor {
        async fn execute_action(
            &self,
            guild_id: GuildId,
            target: UserId,
            intent: &ModerationIntent,
            reason: &str,
        ) -> anyhow::Result<()> {
            if self.fail_action {
                anyhow::bail!("Simulated Discord API failure");
            }
            self.executed
                .lock()
                .await
                .push((guild_id, target, intent.clone(), reason.to_string()));
            Ok(())
        }

        async fn send_moderation_channel_log(
            &self,
            channel_id: poise::serenity_prelude::ChannelId,
            summary: &str,
        ) -> anyhow::Result<()> {
            self.logged
                .lock()
                .await
                .push((channel_id, summary.to_string()));
            Ok(())
        }
    }

    #[tokio::test]
    async fn executes_moderation_action_and_logs_to_channel() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-exec-mod-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let pool =
            sqlx::SqlitePool::connect(&format!("sqlite:{}/bot.db?mode=rwc", directory.display()))
                .await
                .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        // Configure moderation channel
        sqlx::query(
            "INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES ('1', '999')",
        )
        .execute(&pool)
        .await
        .unwrap();

        let executor = MockDiscordExecutor::new(false);
        let result = execute_moderation_action(
            &executor,
            &pool,
            ModerationRequest {
                guild_id: GuildId::new(1),
                target: UserId::new(10),
                moderator: UserId::new(20),
                intent: ModerationIntent::Kick,
                reason: "Violated rules",
                evidence_url: Some("https://discord.com/channels/1/2/3"),
                language: crate::i18n::Language::English,
                denial: None,
            },
        )
        .await
        .unwrap();

        assert_eq!(result.case_number, 1);
        assert!(result.channel_logged);
        assert!(result.summary_text.contains("Kick"));
        assert!(result.summary_text.contains("#1"));

        // Verify mock recorded Discord action
        let executed = executor.executed.lock().await;
        assert_eq!(executed.len(), 1);
        assert_eq!(executed[0].0, GuildId::new(1));
        assert_eq!(executed[0].1, UserId::new(10));
        assert_eq!(executed[0].2, ModerationIntent::Kick);
        assert_eq!(executed[0].3, "Violated rules");

        // Verify channel log was sent
        let logged = executor.logged.lock().await;
        assert_eq!(logged.len(), 1);
        assert_eq!(logged[0].0, poise::serenity_prelude::ChannelId::new(999));

        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn rejects_empty_reason_and_invalid_evidence() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-exec-val-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let pool =
            sqlx::SqlitePool::connect(&format!("sqlite:{}/bot.db?mode=rwc", directory.display()))
                .await
                .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();

        let executor = MockDiscordExecutor::new(false);

        // Empty reason
        let err1 = execute_moderation_action(
            &executor,
            &pool,
            ModerationRequest {
                guild_id: GuildId::new(1),
                target: UserId::new(10),
                moderator: UserId::new(20),
                intent: ModerationIntent::Warn,
                reason: "   ",
                evidence_url: None,
                language: crate::i18n::Language::English,
                denial: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err1, ModerationExecutionError::EmptyReason));

        // Invalid evidence URL (wrong guild ID)
        let err2 = execute_moderation_action(
            &executor,
            &pool,
            ModerationRequest {
                guild_id: GuildId::new(1),
                target: UserId::new(10),
                moderator: UserId::new(20),
                intent: ModerationIntent::Warn,
                reason: "Valid reason",
                evidence_url: Some("https://discord.com/channels/999/2/3"),
                language: crate::i18n::Language::English,
                denial: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err2, ModerationExecutionError::InvalidEvidence));

        // Simulated Discord failure
        let failing_executor = MockDiscordExecutor::new(true);
        let err3 = execute_moderation_action(
            &failing_executor,
            &pool,
            ModerationRequest {
                guild_id: GuildId::new(1),
                target: UserId::new(10),
                moderator: UserId::new(20),
                intent: ModerationIntent::Kick,
                reason: "Valid reason",
                evidence_url: None,
                language: crate::i18n::Language::English,
                denial: None,
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(err3, ModerationExecutionError::DiscordFailed(_)));

        // Simulated permission denial
        let err4 = execute_moderation_action(
            &executor,
            &pool,
            ModerationRequest {
                guild_id: GuildId::new(1),
                target: UserId::new(10),
                moderator: UserId::new(20),
                intent: ModerationIntent::Warn,
                reason: "Valid reason",
                evidence_url: None,
                language: crate::i18n::Language::English,
                denial: Some(crate::permissions::ModerationDenial::SelfTarget),
            },
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err4,
            ModerationExecutionError::Denial(crate::permissions::ModerationDenial::SelfTarget)
        ));

        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
