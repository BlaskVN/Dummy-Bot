pub use crate::message_log::health::{current_health, mark_warning_sent, reconcile};
pub use crate::message_log::models::MessageLogHealth;

#[cfg(test)]
mod tests {
    use super::{MessageLogHealth, mark_warning_sent, reconcile};
    use crate::database::init_db;
    use poise::serenity_prelude::GuildId;

    #[tokio::test]
    async fn tracks_healthy_degraded_restart_and_recovery() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-message-health-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        sqlx::query("INSERT INTO message_log_config (guild_id, log_channel_id, enabled) VALUES ('1', '2', 1)").execute(&pool).await.unwrap();
        assert_eq!(
            reconcile(&pool, GuildId::new(1), true).await.unwrap(),
            (MessageLogHealth::Healthy, false)
        );
        assert_eq!(
            reconcile(&pool, GuildId::new(1), false).await.unwrap(),
            (MessageLogHealth::Degraded, true)
        );
        mark_warning_sent(&pool, GuildId::new(1)).await.unwrap();
        assert_eq!(
            reconcile(&pool, GuildId::new(1), false).await.unwrap(),
            (MessageLogHealth::Degraded, false)
        );
        assert_eq!(
            reconcile(&pool, GuildId::new(1), true).await.unwrap(),
            (MessageLogHealth::Healthy, false)
        );
    }
}
