use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

use crate::i18n::Language;
use crate::word_puzzle_store::SummaryEntry;

/// Outbox port trait for delivering word puzzle summary messages.
#[async_trait::async_trait]
pub trait WordPuzzleOutbox: Send + Sync {
    async fn deliver_summary(
        &self,
        guild_id: serenity::all::GuildId,
        channel_id: serenity::all::ChannelId,
        answer: &str,
        rows: &[SummaryEntry],
    ) -> Result<(), anyhow::Error>;
}

/// Reconciles expired word puzzle sessions, awards pending credits,
/// claims finished sessions, and delivers summaries through the outbox port.
pub async fn reconcile_and_deliver<O: WordPuzzleOutbox>(
    pool: &SqlitePool,
    outbox: &O,
) -> Result<(), anyhow::Error> {
    let now = chrono::Utc::now().timestamp();
    crate::word_puzzle_store::reconcile_expired(pool, now, 100).await?;
    crate::word_puzzle_store::award_pending_credits(pool, now, 500).await?;
    let sessions = crate::word_puzzle_store::claim_finished(pool, now, 20).await?;

    for session in sessions {
        let delivery_result = async {
            let guild_id = serenity::all::GuildId::new(session.guild_id.parse::<u64>()?);
            let channel_id =
                serenity::all::ChannelId::new(session.result_channel_id.parse::<u64>()?);
            let rows = crate::word_puzzle_store::summary(pool, session.id).await?;
            outbox
                .deliver_summary(guild_id, channel_id, &session.answer, &rows)
                .await
        }
        .await;

        match delivery_result {
            Ok(()) => {
                if let Err(error) =
                    crate::word_puzzle_store::cleanup_delivered(pool, session.id).await
                {
                    tracing::error!(
                        session_id = session.id,
                        %error,
                        "Could not clean delivered Word Puzzle"
                    );
                }
            }
            Err(error) => {
                tracing::error!(
                    session_id = session.id,
                    %error,
                    "Could not deliver Word Puzzle summary"
                );
                if let Err(rel_err) =
                    crate::word_puzzle_store::release_summary_claim(pool, session.id).await
                {
                    tracing::error!(
                        session_id = session.id,
                        %rel_err,
                        "Could not release Word Puzzle summary claim"
                    );
                }
            }
        }
    }

    Ok(())
}

/// Discord adapter implementation of `WordPuzzleOutbox`.
pub struct DiscordOutbox<'a> {
    pub ctx: &'a serenity::all::Context,
    pub data: &'a crate::state::Data,
}

impl<'a> DiscordOutbox<'a> {
    pub fn new(ctx: &'a serenity::all::Context, data: &'a crate::state::Data) -> Self {
        Self { ctx, data }
    }
}

#[async_trait::async_trait]
impl WordPuzzleOutbox for DiscordOutbox<'_> {
    async fn deliver_summary(
        &self,
        guild_id: serenity::all::GuildId,
        channel_id: serenity::all::ChannelId,
        answer: &str,
        rows: &[SummaryEntry],
    ) -> Result<(), anyhow::Error> {
        let language = self.data.language(guild_id).await;
        let summary = format_summary(language, answer, rows);
        channel_id
            .send_message(
                self.ctx,
                serenity::all::CreateMessage::new()
                    .embed(crate::ui::panel(
                        self.data,
                        crate::ui::Tone::Primary,
                        summary,
                    ))
                    .allowed_mentions(serenity::all::CreateAllowedMentions::new()),
            )
            .await?;
        Ok(())
    }
}

/// Convenience function that delivers finished word puzzle summaries using Discord.
pub async fn reconcile_and_deliver_discord(
    ctx: &serenity::all::Context,
    data: &crate::state::Data,
) {
    let outbox = DiscordOutbox::new(ctx, data);
    if let Err(error) = reconcile_and_deliver(&data.db_pool, &outbox).await {
        tracing::error!(%error, "Could not reconcile and deliver Word Puzzles");
    }
}

/// Formats the summary text in the target language.
pub fn format_summary(language: Language, answer: &str, rows: &[SummaryEntry]) -> String {
    let rows_str = rows
        .iter()
        .map(|row| {
            let template = match (language, row.status.as_str()) {
                (Language::English, "won") => "<@{}> — solved in {} attempt(s)",
                (Language::English, _) => "<@{}> — unsolved after {} attempt(s)",
                (Language::Vietnamese, "won") => "<@{}> — giải được trong {} lượt",
                (Language::Vietnamese, _) => "<@{}> — chưa giải được sau {} lượt",
                (Language::Japanese, "won") => "<@{}> — {}回で正解",
                (Language::Japanese, _) => "<@{}> — {}回で未正解",
            };
            template
                .replacen("{}", &row.user_id, 1)
                .replacen("{}", &row.attempts.to_string(), 1)
        })
        .collect::<Vec<_>>()
        .join("\n");

    let (title, answer_fmt) = match language {
        Language::English => ("**Word Puzzle complete**", "Answer: **{}**"),
        Language::Vietnamese => ("**Câu đố chữ đã hoàn tất**", "Đáp án: **{}**"),
        Language::Japanese => ("**ワードパズル終了**", "答え：**{}**"),
    };

    format!(
        "{title}\n{}\n{rows_str}",
        answer_fmt.replacen("{}", &answer.to_uppercase(), 1)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::init_db;
    use crate::word_puzzle_store::{
        create_session, finish_now, session, start,
    };
    use poise::serenity_prelude::{ChannelId, GuildId, UserId};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Mutex;

    #[derive(Debug, Clone)]
    struct RecordedDelivery {
        guild_id: u64,
        channel_id: u64,
        answer: String,
        #[allow(dead_code)]
        rows: Vec<SummaryEntry>,
    }

    struct MockOutbox {
        calls: Mutex<Vec<RecordedDelivery>>,
        should_fail: AtomicBool,
    }

    impl MockOutbox {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                should_fail: AtomicBool::new(false),
            }
        }

        fn recorded_calls(&self) -> Vec<RecordedDelivery> {
            self.calls.lock().unwrap().clone()
        }
    }

    #[async_trait::async_trait]
    impl WordPuzzleOutbox for MockOutbox {
        async fn deliver_summary(
            &self,
            guild_id: serenity::all::GuildId,
            channel_id: serenity::all::ChannelId,
            answer: &str,
            rows: &[SummaryEntry],
        ) -> Result<(), anyhow::Error> {
            if self.should_fail.load(Ordering::SeqCst) {
                anyhow::bail!("Simulated delivery error");
            }
            self.calls.lock().unwrap().push(RecordedDelivery {
                guild_id: guild_id.get(),
                channel_id: channel_id.get(),
                answer: answer.to_owned(),
                rows: rows.to_vec(),
            });
            Ok(())
        }
    }

    async fn test_pool(name: &str) -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-word-puzzle-engine-{name}-{}-{}",
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
        (pool, directory)
    }

    #[tokio::test]
    async fn delivery_success_cleans_up_claimed_session() {
        let (pool, directory) = test_pool("success").await;
        let outbox = MockOutbox::new();

        let s = create_session(
            &pool,
            GuildId::new(10),
            UserId::new(20),
            ChannelId::new(30),
            100,
        )
        .await
        .unwrap();
        start(&pool, s.id, UserId::new(20), 105, 5).await.unwrap();
        assert!(finish_now(&pool, s.id, UserId::new(20), 110).await.unwrap());

        reconcile_and_deliver(&pool, &outbox).await.unwrap();

        let calls = outbox.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].guild_id, 10);
        assert_eq!(calls[0].channel_id, 30);
        assert_eq!(calls[0].answer, s.answer);

        // Session should be cleaned up after successful delivery
        assert!(session(&pool, s.id).await.unwrap().is_none());

        pool.close().await;
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn delivery_failure_releases_claim_and_allows_retry() {
        let (pool, directory) = test_pool("failure-retry").await;
        let outbox = MockOutbox::new();
        outbox.should_fail.store(true, Ordering::SeqCst);

        let s = create_session(
            &pool,
            GuildId::new(11),
            UserId::new(21),
            ChannelId::new(31),
            100,
        )
        .await
        .unwrap();
        start(&pool, s.id, UserId::new(21), 105, 5).await.unwrap();
        assert!(finish_now(&pool, s.id, UserId::new(21), 110).await.unwrap());

        // Attempt delivery which fails
        reconcile_and_deliver(&pool, &outbox).await.unwrap();

        // Session should still exist and claim should be released
        let existing = session(&pool, s.id).await.unwrap();
        assert!(existing.is_some());
        let claimed_at: Option<i64> = sqlx::query_scalar(
            "SELECT summary_claimed_at FROM word_puzzle_session WHERE id = ?",
        )
        .bind(s.id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(claimed_at.is_none(), "Claim should have been released");

        // Now allow outbox to succeed and retry
        outbox.should_fail.store(false, Ordering::SeqCst);
        reconcile_and_deliver(&pool, &outbox).await.unwrap();

        let calls = outbox.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].guild_id, 11);
        assert_eq!(calls[0].channel_id, 31);

        // Session cleaned up after retry succeeded
        assert!(session(&pool, s.id).await.unwrap().is_none());

        pool.close().await;
        let _ = std::fs::remove_dir_all(directory);
    }

    #[tokio::test]
    async fn expired_session_reconciliation_and_credit_awarding_triggered() {
        let (pool, directory) = test_pool("reconcile-and-award").await;
        let outbox = MockOutbox::new();

        // Setup 1: Expired session (started at 100, duration 10 seconds -> deadline 110, now is current time > 110)
        let s = create_session(
            &pool,
            GuildId::new(12),
            UserId::new(22),
            ChannelId::new(32),
            100,
        )
        .await
        .unwrap();
        start(&pool, s.id, UserId::new(22), 105, 10).await.unwrap();

        // Setup 2: Pending credit in word_puzzle_completion
        sqlx::query(
            "INSERT INTO word_puzzle_completion (session_id, guild_id, user_id, completed_at) VALUES (?, ?, ?, ?)",
        )
        .bind(999)
        .bind("12")
        .bind("22")
        .bind(100)
        .execute(&pool)
        .await
        .unwrap();

        // Run engine reconcile and deliver
        reconcile_and_deliver(&pool, &outbox).await.unwrap();

        // 1. Expired session should have been reconciled (expired -> finished) and delivered
        let calls = outbox.recorded_calls();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].guild_id, 12);
        assert_eq!(calls[0].channel_id, 32);
        assert!(session(&pool, s.id).await.unwrap().is_none());

        // 2. Pending credit should have been processed (credit_processed_at set)
        let unprocessed_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM word_puzzle_completion WHERE credit_processed_at IS NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(unprocessed_count, 0);

        pool.close().await;
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn format_summary_renders_all_supported_languages() {
        let entries = vec![
            SummaryEntry {
                user_id: "101".to_owned(),
                status: "won".to_owned(),
                attempts: 3,
            },
            SummaryEntry {
                user_id: "102".to_owned(),
                status: "lost".to_owned(),
                attempts: 6,
            },
        ];

        let en = format_summary(Language::English, "apple", &entries);
        assert!(en.contains("**Word Puzzle complete**"));
        assert!(en.contains("Answer: **APPLE**"));
        assert!(en.contains("<@101> — solved in 3 attempt(s)"));
        assert!(en.contains("<@102> — unsolved after 6 attempt(s)"));

        let vi = format_summary(Language::Vietnamese, "apple", &entries);
        assert!(vi.contains("**Câu đố chữ đã hoàn tất**"));
        assert!(vi.contains("Đáp án: **APPLE**"));
        assert!(vi.contains("<@101> — giải được trong 3 lượt"));
        assert!(vi.contains("<@102> — chưa giải được sau 6 lượt"));

        let ja = format_summary(Language::Japanese, "apple", &entries);
        assert!(ja.contains("**ワードパズル終了**"));
        assert!(ja.contains("答え：**APPLE**"));
        assert!(ja.contains("<@101> — 3回で正解"));
        assert!(ja.contains("<@102> — 6回で未正解"));
    }
}
