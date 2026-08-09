use anyhow::Result;
use poise::serenity_prelude::{GuildId, ScheduledEventId, UserId};
use sqlx::SqlitePool;
use std::collections::HashSet;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttendanceRecord {
    pub user_id: String,
    pub accumulated_seconds: i64,
    pub active_started_at: Option<i64>,
}

pub async fn reconcile_attendance(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    eligible: &[UserId],
    now: i64,
) -> Result<()> {
    let eligible = eligible
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>();
    let mut transaction = pool.begin().await?;
    let active: Vec<(String, i64)> = sqlx::query_as("SELECT user_id, active_started_at FROM activity_attendance WHERE guild_id = ? AND scheduled_event_id = ? AND active_started_at IS NOT NULL")
        .bind(guild_id.to_string()).bind(event_id.to_string()).fetch_all(&mut *transaction).await?;
    for (user, started) in active {
        if !eligible.contains(&user) {
            insert_interval(&mut transaction, guild_id, event_id, &user, started, now).await?;
            sqlx::query("UPDATE activity_attendance SET accumulated_seconds = accumulated_seconds + MAX(0, ? - ?), active_started_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ? AND active_started_at = ?")
                .bind(now).bind(started).bind(guild_id.to_string()).bind(event_id.to_string())
                .bind(&user).bind(started).execute(&mut *transaction).await?;
        }
    }
    for user in eligible {
        let opted_out: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM activity_opt_out WHERE guild_id = ? AND user_id = ?)",
        )
        .bind(guild_id.to_string())
        .bind(&user)
        .fetch_one(&mut *transaction)
        .await?;
        if opted_out {
            continue;
        }
        sqlx::query("INSERT INTO activity_attendance (guild_id, scheduled_event_id, user_id, active_started_at) SELECT ?, ?, ?, ? WHERE EXISTS(SELECT 1 FROM community_activity WHERE guild_id = ? AND scheduled_event_id = ? AND state IN ('scheduled', 'active')) ON CONFLICT(guild_id, scheduled_event_id, user_id) DO UPDATE SET active_started_at = COALESCE(activity_attendance.active_started_at, excluded.active_started_at), updated_at = CURRENT_TIMESTAMP")
            .bind(guild_id.to_string()).bind(event_id.to_string()).bind(&user).bind(now)
            .bind(guild_id.to_string()).bind(event_id.to_string()).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn pause_session(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    now: i64,
) -> Result<()> {
    let mut transaction = pool.begin().await?;
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT user_id, active_started_at FROM activity_attendance WHERE guild_id = ? AND scheduled_event_id = ? AND active_started_at IS NOT NULL")
        .bind(guild_id.to_string()).bind(event_id.to_string()).fetch_all(&mut *transaction).await?;
    for (user, started) in rows {
        insert_interval(&mut transaction, guild_id, event_id, &user, started, now).await?;
        sqlx::query("UPDATE activity_attendance SET accumulated_seconds = accumulated_seconds + MAX(0, ? - ?), active_started_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ? AND active_started_at = ?")
            .bind(now).bind(started).bind(guild_id.to_string()).bind(event_id.to_string())
            .bind(user).bind(started).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

pub async fn pause_member(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: UserId,
    now: i64,
) -> Result<()> {
    let mut transaction = pool.begin().await?;
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT scheduled_event_id, active_started_at FROM activity_attendance WHERE guild_id = ? AND user_id = ? AND active_started_at IS NOT NULL")
        .bind(guild_id.to_string()).bind(user_id.to_string()).fetch_all(&mut *transaction).await?;
    for (event, started) in rows {
        let event_id = ScheduledEventId::new(event.parse()?);
        insert_interval(
            &mut transaction,
            guild_id,
            event_id,
            &user_id.to_string(),
            started,
            now,
        )
        .await?;
        sqlx::query("UPDATE activity_attendance SET accumulated_seconds = accumulated_seconds + MAX(0, ? - ?), active_started_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ? AND active_started_at = ?")
            .bind(now).bind(started).bind(guild_id.to_string()).bind(event_id.to_string())
            .bind(user_id.to_string()).bind(started).execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(())
}

async fn insert_interval(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    user_id: &str,
    started: i64,
    ended: i64,
) -> Result<()> {
    if ended > started {
        sqlx::query("INSERT INTO activity_attendance_interval (guild_id, scheduled_event_id, user_id, started_at, ended_at) VALUES (?, ?, ?, ?, ?) ON CONFLICT DO NOTHING")
            .bind(guild_id.to_string()).bind(event_id.to_string()).bind(user_id)
            .bind(started).bind(ended).execute(&mut **transaction).await?;
    }
    Ok(())
}

pub async fn clear_stale_active_starts(pool: &SqlitePool) -> Result<u64> {
    Ok(sqlx::query("UPDATE activity_attendance SET active_started_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE active_started_at IS NOT NULL")
        .execute(pool).await?.rows_affected())
}

pub async fn attendance_records(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
) -> Result<Vec<AttendanceRecord>> {
    Ok(sqlx::query_as("SELECT user_id, accumulated_seconds, active_started_at FROM activity_attendance WHERE guild_id = ? AND scheduled_event_id = ? ORDER BY user_id")
        .bind(guild_id.to_string()).bind(event_id.to_string()).fetch_all(pool).await?)
}

#[cfg(test)]
mod tests {
    use super::{
        attendance_records, clear_stale_active_starts, pause_session, reconcile_attendance,
    };
    use crate::community::create_activity;
    use crate::database::init_db;
    use poise::serenity_prelude::{GuildId, ScheduledEventId, ScheduledEventType, UserId};

    #[tokio::test]
    async fn accumulates_only_transition_time_and_drops_offline_time() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-attendance-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        create_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(2),
            ScheduledEventType::Voice,
            Some(UserId::new(9)),
            None,
            None,
        )
        .await
        .unwrap();
        reconcile_attendance(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(2),
            &[UserId::new(3), UserId::new(4)],
            100,
        )
        .await
        .unwrap();
        reconcile_attendance(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(2),
            &[UserId::new(3), UserId::new(4)],
            110,
        )
        .await
        .unwrap();
        reconcile_attendance(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(2),
            &[UserId::new(4)],
            130,
        )
        .await
        .unwrap();
        let rows = attendance_records(&pool, GuildId::new(1), ScheduledEventId::new(2))
            .await
            .unwrap();
        assert_eq!(rows[0].accumulated_seconds, 30);
        assert_eq!(rows[0].active_started_at, None);
        assert_eq!(rows[1].active_started_at, Some(100));
        assert_eq!(clear_stale_active_starts(&pool).await.unwrap(), 1);
        let rows = attendance_records(&pool, GuildId::new(1), ScheduledEventId::new(2))
            .await
            .unwrap();
        assert_eq!(rows[1].accumulated_seconds, 0);
        assert_eq!(rows[1].active_started_at, None);
        reconcile_attendance(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(2),
            &[UserId::new(4)],
            200,
        )
        .await
        .unwrap();
        pause_session(&pool, GuildId::new(1), ScheduledEventId::new(2), 210)
            .await
            .unwrap();
        sqlx::query("INSERT INTO activity_opt_out (guild_id, user_id) VALUES ('1', '5')")
            .execute(&pool)
            .await
            .unwrap();
        reconcile_attendance(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(2),
            &[UserId::new(5)],
            220,
        )
        .await
        .unwrap();
        let rows = attendance_records(&pool, GuildId::new(1), ScheduledEventId::new(2))
            .await
            .unwrap();
        assert_eq!(rows[1].accumulated_seconds, 10);
        assert_eq!(rows.len(), 2);
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
