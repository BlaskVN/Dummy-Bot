use anyhow::{Result, bail};
use poise::serenity_prelude::{GuildId, ScheduledEventId, ScheduledEventType, UserId};
use sqlx::SqlitePool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ActivityRecord {
    pub scheduled_event_id: String,
    pub host_user_id: Option<String>,
    pub kind: String,
    pub game_key: Option<String>,
    pub capacity: Option<i64>,
    pub state: String,
    pub notification_sent: i64,
}

pub async fn create_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    entity_type: ScheduledEventType,
    host: Option<UserId>,
    game_key: Option<&str>,
    capacity: Option<i64>,
) -> Result<()> {
    if entity_type != ScheduledEventType::Voice {
        bail!("Only VOICE scheduled events are supported");
    }
    if capacity.is_some_and(|capacity| capacity <= 0) {
        bail!("Activity capacity must be positive");
    }
    let kind = if game_key.is_some() {
        "game"
    } else {
        "community"
    };
    if kind == "community" && host.is_none() {
        bail!("Community activities require a host");
    }
    sqlx::query("INSERT INTO community_activity (guild_id, scheduled_event_id, host_user_id, kind, game_key, capacity) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(guild_id.to_string()).bind(event_id.to_string()).bind(host.map(|id| id.to_string()))
        .bind(kind).bind(game_key).bind(capacity).execute(pool).await?;
    Ok(())
}

pub async fn activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
) -> Result<Option<ActivityRecord>> {
    Ok(sqlx::query_as("SELECT scheduled_event_id, host_user_id, kind, game_key, capacity, state, notification_sent FROM community_activity WHERE guild_id = ? AND scheduled_event_id = ?")
        .bind(guild_id.to_string()).bind(event_id.to_string()).fetch_optional(pool).await?)
}

#[cfg(test)]
mod tests {
    use super::{activity, create_activity};
    use crate::database::init_db;
    use poise::serenity_prelude::{GuildId, ScheduledEventId, ScheduledEventType, UserId};

    #[tokio::test]
    async fn validates_and_isolates_voice_activity_extensions() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-activity-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        assert!(
            create_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(10),
                ScheduledEventType::External,
                Some(UserId::new(2)),
                None,
                None
            )
            .await
            .is_err()
        );
        assert!(
            create_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(10),
                ScheduledEventType::Voice,
                Some(UserId::new(2)),
                None,
                Some(0)
            )
            .await
            .is_err()
        );
        create_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(10),
            ScheduledEventType::Voice,
            Some(UserId::new(2)),
            None,
            Some(5),
        )
        .await
        .unwrap();
        assert!(
            create_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(10),
                ScheduledEventType::Voice,
                Some(UserId::new(2)),
                None,
                Some(5)
            )
            .await
            .is_err()
        );
        create_activity(
            &pool,
            GuildId::new(2),
            ScheduledEventId::new(10),
            ScheduledEventType::Voice,
            Some(UserId::new(3)),
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            activity(&pool, GuildId::new(1), ScheduledEventId::new(10))
                .await
                .unwrap()
                .unwrap()
                .capacity,
            Some(5)
        );
        assert_eq!(
            activity(&pool, GuildId::new(2), ScheduledEventId::new(10))
                .await
                .unwrap()
                .unwrap()
                .host_user_id
                .as_deref(),
            Some("3")
        );
        assert!(
            activity(&pool, GuildId::new(1), ScheduledEventId::new(11))
                .await
                .unwrap()
                .is_none()
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
