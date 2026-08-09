use anyhow::{Result, bail};
use poise::serenity_prelude::{GuildId, ScheduledEventId, ScheduledEventType, UserId};
use sqlx::SqlitePool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipState {
    Participant,
    Waitlisted,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaveResult {
    pub left: bool,
    pub promoted: Vec<UserId>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ManagedActivityId {
    pub guild_id: String,
    pub scheduled_event_id: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExpiringGameActivity {
    pub guild_id: String,
    pub scheduled_event_id: String,
}

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

pub async fn create_game_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    entity_type: ScheduledEventType,
    game_key: &str,
    expires_at: i64,
) -> Result<()> {
    if entity_type != ScheduledEventType::Voice {
        bail!("Only VOICE scheduled events are supported");
    }
    if game_key.is_empty() || expires_at <= chrono::Utc::now().timestamp() {
        bail!("Game activity requires a key and future expiry");
    }
    sqlx::query("INSERT INTO community_activity (guild_id, scheduled_event_id, kind, game_key, expires_at) VALUES (?, ?, 'game', ?, ?)")
        .bind(guild_id.to_string()).bind(event_id.to_string()).bind(game_key).bind(expires_at)
        .execute(pool).await?;
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

pub async fn active_game_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    game_key: &str,
) -> Result<Option<ScheduledEventId>> {
    let id: Option<String> = sqlx::query_scalar("SELECT scheduled_event_id FROM community_activity WHERE guild_id = ? AND kind = 'game' AND game_key = ? AND state IN ('scheduled', 'active')")
        .bind(guild_id.to_string()).bind(game_key).fetch_optional(pool).await?;
    id.map(|id| Ok(ScheduledEventId::new(id.parse()?)))
        .transpose()
}

pub async fn update_activity_extension(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    capacity: Option<i64>,
    state: Option<&str>,
) -> Result<bool> {
    if capacity.is_some_and(|value| value <= 0) {
        bail!("Activity capacity must be positive");
    }
    if state.is_some_and(|value| {
        !matches!(
            value,
            "scheduled" | "active" | "completed" | "canceled" | "deleted"
        )
    }) {
        bail!("Invalid activity state");
    }
    Ok(sqlx::query("UPDATE community_activity SET capacity = COALESCE(?, capacity), state = COALESCE(?, state), updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ?")
        .bind(capacity).bind(state).bind(guild_id.to_string()).bind(event_id.to_string())
        .execute(pool).await?.rows_affected() == 1)
}

pub async fn join_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    user_id: UserId,
    is_bot: bool,
) -> Result<MembershipState> {
    if is_bot {
        bail!("Bot accounts cannot join activities");
    }
    let mut transaction = pool.begin().await?;
    let inserted = sqlx::query(
        "INSERT INTO community_activity_member (guild_id, scheduled_event_id, user_id, state)\n         SELECT a.guild_id, a.scheduled_event_id, ?,\n           CASE WHEN a.capacity IS NULL OR (SELECT COUNT(*) FROM community_activity_member m WHERE m.guild_id = a.guild_id AND m.scheduled_event_id = a.scheduled_event_id AND m.state = 'participant') < a.capacity\n             THEN 'participant' ELSE 'waitlisted' END\n         FROM community_activity a\n         WHERE a.guild_id = ? AND a.scheduled_event_id = ? AND a.state IN ('scheduled', 'active')\n         ON CONFLICT(guild_id, scheduled_event_id, user_id) DO NOTHING",
    )
    .bind(user_id.to_string())
    .bind(guild_id.to_string())
    .bind(event_id.to_string())
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    let activity_state: Option<String> = sqlx::query_scalar(
        "SELECT state FROM community_activity WHERE guild_id = ? AND scheduled_event_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(event_id.to_string())
    .fetch_optional(&mut *transaction)
    .await?;
    if !matches!(activity_state.as_deref(), Some("scheduled" | "active")) {
        transaction.commit().await?;
        return Ok(MembershipState::Closed);
    }
    let state: String = sqlx::query_scalar("SELECT state FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ?")
        .bind(guild_id.to_string()).bind(event_id.to_string()).bind(user_id.to_string())
        .fetch_one(&mut *transaction).await?;
    transaction.commit().await?;
    debug_assert!(inserted <= 1);
    Ok(if state == "participant" {
        MembershipState::Participant
    } else {
        MembershipState::Waitlisted
    })
}

pub async fn leave_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    user_id: UserId,
) -> Result<LeaveResult> {
    let mut transaction = pool.begin().await?;
    let old_state: Option<String> = sqlx::query_scalar("DELETE FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ? RETURNING state")
        .bind(guild_id.to_string()).bind(event_id.to_string()).bind(user_id.to_string())
        .fetch_optional(&mut *transaction).await?;
    let promoted = if old_state.as_deref() == Some("participant") {
        promote_available(&mut transaction, guild_id, event_id).await?
    } else {
        Vec::new()
    };
    transaction.commit().await?;
    Ok(LeaveResult {
        left: old_state.is_some(),
        promoted,
    })
}

pub async fn set_activity_capacity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    capacity: Option<i64>,
) -> Result<Vec<UserId>> {
    if capacity.is_some_and(|value| value <= 0) {
        bail!("Activity capacity must be positive");
    }
    let mut transaction = pool.begin().await?;
    let changed = sqlx::query("UPDATE community_activity SET capacity = ?, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ?")
        .bind(capacity).bind(guild_id.to_string()).bind(event_id.to_string())
        .execute(&mut *transaction).await?.rows_affected();
    if changed == 0 {
        bail!("Activity not found");
    }
    let promoted = promote_available(&mut transaction, guild_id, event_id).await?;
    transaction.commit().await?;
    Ok(promoted)
}

async fn promote_available(
    transaction: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    guild_id: GuildId,
    event_id: ScheduledEventId,
) -> Result<Vec<UserId>> {
    let capacity: Option<i64> = sqlx::query_scalar(
        "SELECT capacity FROM community_activity WHERE guild_id = ? AND scheduled_event_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(event_id.to_string())
    .fetch_one(&mut **transaction)
    .await?;
    let participant_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ? AND state = 'participant'")
        .bind(guild_id.to_string()).bind(event_id.to_string())
        .fetch_one(&mut **transaction).await?;
    let limit = capacity.map_or(i64::MAX, |value| (value - participant_count).max(0));
    if limit == 0 {
        return Ok(Vec::new());
    }
    let promoted: Vec<String> = sqlx::query_scalar(
        "UPDATE community_activity_member SET state = 'participant', promoted_at = CURRENT_TIMESTAMP, promotion_notification = 'pending'\n         WHERE sequence IN (SELECT sequence FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ? AND state = 'waitlisted' ORDER BY sequence LIMIT ?)\n         RETURNING user_id",
    )
    .bind(guild_id.to_string())
    .bind(event_id.to_string())
    .bind(limit)
    .fetch_all(&mut **transaction)
    .await?;
    promoted
        .into_iter()
        .map(|id| Ok(UserId::new(id.parse()?)))
        .collect()
}

pub async fn claim_promotion_notification(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    user_id: UserId,
) -> Result<bool> {
    Ok(sqlx::query("UPDATE community_activity_member SET promotion_notification = 'sending' WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ? AND promotion_notification = 'pending'")
        .bind(guild_id.to_string()).bind(event_id.to_string()).bind(user_id.to_string())
        .execute(pool).await?.rows_affected() == 1)
}

pub async fn finish_promotion_notification(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    user_id: UserId,
    delivered: bool,
) -> Result<()> {
    sqlx::query("UPDATE community_activity_member SET promotion_notification = ? WHERE guild_id = ? AND scheduled_event_id = ? AND user_id = ? AND promotion_notification = 'sending'")
        .bind(if delivered { "delivered" } else { "failed" }).bind(guild_id.to_string())
        .bind(event_id.to_string()).bind(user_id.to_string()).execute(pool).await?;
    Ok(())
}

pub async fn mirror_activity_state(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    state: &str,
) -> Result<bool> {
    update_activity_extension(pool, guild_id, event_id, None, Some(state)).await
}

pub async fn nonterminal_activities(
    pool: &SqlitePool,
    limit: i64,
) -> Result<Vec<ManagedActivityId>> {
    Ok(sqlx::query_as("SELECT guild_id, scheduled_event_id FROM community_activity WHERE state IN ('scheduled', 'active') ORDER BY updated_at LIMIT ?")
        .bind(limit).fetch_all(pool).await?)
}

pub async fn claim_deleted_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
) -> Result<Option<Vec<UserId>>> {
    let mut transaction = pool.begin().await?;
    let claimed = sqlx::query("UPDATE community_activity SET state = 'deleted', notification_sent = 1, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ? AND notification_sent = 0")
        .bind(guild_id.to_string()).bind(event_id.to_string())
        .execute(&mut *transaction).await?.rows_affected();
    if claimed == 0 {
        let managed: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM community_activity WHERE guild_id = ? AND scheduled_event_id = ?)")
            .bind(guild_id.to_string()).bind(event_id.to_string())
            .fetch_one(&mut *transaction).await?;
        transaction.commit().await?;
        return Ok(managed.then(Vec::new));
    }
    let users: Vec<String> = sqlx::query_scalar("SELECT user_id FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ? ORDER BY sequence")
        .bind(guild_id.to_string()).bind(event_id.to_string())
        .fetch_all(&mut *transaction).await?;
    sqlx::query(
        "DELETE FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(event_id.to_string())
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Some(
        users
            .into_iter()
            .map(|id| Ok(UserId::new(id.parse()?)))
            .collect::<Result<Vec<_>>>()?,
    ))
}

pub async fn next_game_expiry(pool: &SqlitePool) -> Result<Option<i64>> {
    Ok(sqlx::query_scalar("SELECT MIN(expires_at) FROM community_activity WHERE kind = 'game' AND state IN ('scheduled', 'active') AND expires_at IS NOT NULL")
        .fetch_one(pool).await?)
}

pub async fn due_game_activities(
    pool: &SqlitePool,
    now: i64,
    limit: i64,
) -> Result<Vec<ExpiringGameActivity>> {
    Ok(sqlx::query_as("SELECT guild_id, scheduled_event_id FROM community_activity WHERE kind = 'game' AND state IN ('scheduled', 'active') AND expires_at <= ? ORDER BY expires_at LIMIT ?")
        .bind(now).bind(limit).fetch_all(pool).await?)
}

pub async fn finish_game_expiry(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    state: &str,
) -> Result<bool> {
    if !matches!(state, "canceled" | "deleted") {
        bail!("Invalid expiry state");
    }
    let mut transaction = pool.begin().await?;
    let changed = sqlx::query("UPDATE community_activity SET state = ?, expires_at = NULL, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ? AND scheduled_event_id = ? AND kind = 'game' AND state IN ('scheduled', 'active')")
        .bind(state).bind(guild_id.to_string()).bind(event_id.to_string())
        .execute(&mut *transaction).await?.rows_affected() == 1;
    if changed {
        sqlx::query(
            "DELETE FROM community_activity_member WHERE guild_id = ? AND scheduled_event_id = ?",
        )
        .bind(guild_id.to_string())
        .bind(event_id.to_string())
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::{
        MembershipState, active_game_activity, activity, claim_deleted_activity,
        claim_promotion_notification, create_activity, create_game_activity, due_game_activities,
        finish_game_expiry, finish_promotion_notification, join_activity, leave_activity,
        mirror_activity_state, next_game_expiry, nonterminal_activities, set_activity_capacity,
        update_activity_extension,
    };
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
        create_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(15),
            ScheduledEventType::Voice,
            None,
            Some("minecraft"),
            None,
        )
        .await
        .unwrap();
        assert_eq!(
            active_game_activity(&pool, GuildId::new(1), "minecraft")
                .await
                .unwrap(),
            Some(ScheduledEventId::new(15))
        );
        assert!(
            create_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(16),
                ScheduledEventType::Voice,
                None,
                Some("minecraft"),
                None,
            )
            .await
            .is_err()
        );

        create_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(20),
            ScheduledEventType::Voice,
            Some(UserId::new(2)),
            None,
            Some(1),
        )
        .await
        .unwrap();
        assert_eq!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(3),
                false,
            )
            .await
            .unwrap(),
            MembershipState::Participant
        );
        assert_eq!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(4),
                false,
            )
            .await
            .unwrap(),
            MembershipState::Waitlisted
        );
        assert_eq!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(6),
                false,
            )
            .await
            .unwrap(),
            MembershipState::Waitlisted
        );
        assert_eq!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(4),
                false,
            )
            .await
            .unwrap(),
            MembershipState::Waitlisted
        );
        assert!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(5),
                true,
            )
            .await
            .is_err()
        );
        let leave = leave_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(20),
            UserId::new(3),
        )
        .await
        .unwrap();
        assert_eq!(leave.promoted, vec![UserId::new(4)]);
        assert!(
            claim_promotion_notification(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(4),
            )
            .await
            .unwrap()
        );
        assert!(
            !claim_promotion_notification(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(20),
                UserId::new(4),
            )
            .await
            .unwrap()
        );
        finish_promotion_notification(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(20),
            UserId::new(4),
            false,
        )
        .await
        .unwrap();
        let notification: String = sqlx::query_scalar("SELECT promotion_notification FROM community_activity_member WHERE guild_id = '1' AND scheduled_event_id = '20' AND user_id = '4'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(notification, "failed");
        let promoted =
            set_activity_capacity(&pool, GuildId::new(1), ScheduledEventId::new(20), None)
                .await
                .unwrap();
        assert_eq!(promoted, vec![UserId::new(6)]);

        create_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(30),
            ScheduledEventType::Voice,
            Some(UserId::new(2)),
            None,
            Some(1),
        )
        .await
        .unwrap();
        let (first, second) = tokio::join!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(30),
                UserId::new(7),
                false,
            ),
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(30),
                UserId::new(8),
                false,
            )
        );
        let states = [first.unwrap(), second.unwrap()];
        assert_eq!(
            states
                .iter()
                .filter(|state| **state == MembershipState::Participant)
                .count(),
            1
        );
        assert_eq!(
            states
                .iter()
                .filter(|state| **state == MembershipState::Waitlisted)
                .count(),
            1
        );

        create_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(40),
            ScheduledEventType::Voice,
            Some(UserId::new(2)),
            None,
            Some(2),
        )
        .await
        .unwrap();
        for user in 10..=13 {
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(40),
                UserId::new(user),
                false,
            )
            .await
            .unwrap();
        }
        let (first, second) = tokio::join!(
            leave_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(40),
                UserId::new(10),
            ),
            leave_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(40),
                UserId::new(11),
            )
        );
        let mut promoted = first
            .unwrap()
            .promoted
            .into_iter()
            .chain(second.unwrap().promoted)
            .map(UserId::get)
            .collect::<Vec<_>>();
        promoted.sort_unstable();
        assert_eq!(promoted, vec![12, 13]);
        let participants: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_activity_member WHERE guild_id = '1' AND scheduled_event_id = '40' AND state = 'participant'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(participants, 2);
        assert!(
            !leave_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(40),
                UserId::new(10),
            )
            .await
            .unwrap()
            .left
        );
        update_activity_extension(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(30),
            None,
            Some("completed"),
        )
        .await
        .unwrap();
        assert_eq!(
            join_activity(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(30),
                UserId::new(9),
                false,
            )
            .await
            .unwrap(),
            MembershipState::Closed
        );
        assert!(
            mirror_activity_state(&pool, GuildId::new(1), ScheduledEventId::new(20), "active",)
                .await
                .unwrap()
        );
        assert!(
            nonterminal_activities(&pool, 100)
                .await
                .unwrap()
                .iter()
                .any(|row| row.scheduled_event_id == "20")
        );
        assert_eq!(
            claim_deleted_activity(&pool, GuildId::new(1), ScheduledEventId::new(40))
                .await
                .unwrap()
                .unwrap(),
            vec![UserId::new(12), UserId::new(13)]
        );
        assert!(
            claim_deleted_activity(&pool, GuildId::new(1), ScheduledEventId::new(40))
                .await
                .unwrap()
                .unwrap()
                .is_empty()
        );
        assert!(
            claim_deleted_activity(&pool, GuildId::new(1), ScheduledEventId::new(99))
                .await
                .unwrap()
                .is_none()
        );
        let remaining: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_activity_member WHERE guild_id = '1' AND scheduled_event_id = '40'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(remaining, 0);
        let expiry = chrono::Utc::now().timestamp() + 100;
        create_game_activity(
            &pool,
            GuildId::new(1),
            ScheduledEventId::new(80),
            ScheduledEventType::Voice,
            "factorio",
            expiry,
        )
        .await
        .unwrap();
        assert_eq!(next_game_expiry(&pool).await.unwrap(), Some(expiry));
        assert!(
            due_game_activities(&pool, expiry - 1, 10)
                .await
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            due_game_activities(&pool, expiry, 10).await.unwrap().len(),
            1
        );
        assert!(
            finish_game_expiry(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(80),
                "canceled",
            )
            .await
            .unwrap()
        );
        assert!(
            !finish_game_expiry(
                &pool,
                GuildId::new(1),
                ScheduledEventId::new(80),
                "canceled",
            )
            .await
            .unwrap()
        );
        assert_eq!(next_game_expiry(&pool).await.unwrap(), None);
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
