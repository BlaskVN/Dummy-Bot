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
        ModerationAction, create_case, get_case, list_cases, valid_evidence_url, void_case,
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
        sqlx::query("INSERT INTO guild_config (guild_id, prefix) VALUES ('1', '!')")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let prefix: String =
            sqlx::query_scalar("SELECT prefix FROM guild_config WHERE guild_id = '1'")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(prefix, "!");
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
}
