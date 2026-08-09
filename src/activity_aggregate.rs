use anyhow::Result;
use poise::serenity_prelude::{GuildId, ScheduledEventId};
use sqlx::SqlitePool;
use std::collections::{HashMap, HashSet};

const QUALIFY_SECONDS: i64 = 30 * 60;

pub async fn finalize_activity(
    pool: &SqlitePool,
    guild_id: GuildId,
    event_id: ScheduledEventId,
    now: i64,
) -> Result<bool> {
    let game_key: Option<String> = sqlx::query_scalar("SELECT COALESCE(game_key, 'community') FROM community_activity WHERE guild_id = ? AND scheduled_event_id = ? AND state IN ('completed', 'canceled', 'deleted')")
        .bind(guild_id.to_string()).bind(event_id.to_string()).fetch_optional(pool).await?;
    match game_key {
        Some(game_key) => finalize_game(pool, guild_id, &game_key, now).await,
        None => Ok(false),
    }
}

pub async fn finalize_pending(pool: &SqlitePool, now: i64) -> Result<()> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT DISTINCT guild_id, COALESCE(game_key, 'community') FROM community_activity WHERE state IN ('completed', 'canceled', 'deleted') AND finalized_at IS NULL ORDER BY guild_id LIMIT 500")
        .fetch_all(pool).await?;
    for (guild, game_key) in rows {
        finalize_game(pool, GuildId::new(guild.parse()?), &game_key, now).await?;
    }
    Ok(())
}

pub async fn add_session_credit(
    pool: &SqlitePool,
    guild_id: GuildId,
    user_id: poise::serenity_prelude::UserId,
    game_key: &str,
    source_key: &str,
) -> Result<bool> {
    let mut transaction = pool.begin().await?;
    let inserted = sqlx::query("INSERT INTO activity_completion (guild_id, source_key, user_id, game_key, play_minutes, session_credit) VALUES (?, ?, ?, ?, 0, 1) ON CONFLICT DO NOTHING")
        .bind(guild_id.to_string()).bind(source_key).bind(user_id.to_string()).bind(game_key)
        .execute(&mut *transaction).await?.rows_affected() == 1;
    if inserted {
        sqlx::query("INSERT INTO activity_member_game_aggregate (guild_id, user_id, game_key, play_minutes, session_credits) VALUES (?, ?, ?, 0, 1) ON CONFLICT(guild_id, user_id, game_key) DO UPDATE SET session_credits = session_credits + 1")
            .bind(guild_id.to_string()).bind(user_id.to_string()).bind(game_key)
            .execute(&mut *transaction).await?;
        sqlx::query("INSERT INTO activity_member_aggregate (guild_id, user_id, play_minutes, session_credits) SELECT ?, ?, COALESCE(SUM(play_minutes), 0), COALESCE(SUM(session_credits), 0) FROM activity_member_game_aggregate WHERE guild_id = ? AND user_id = ? ON CONFLICT(guild_id, user_id) DO UPDATE SET play_minutes = excluded.play_minutes, session_credits = excluded.session_credits")
            .bind(guild_id.to_string()).bind(user_id.to_string()).bind(guild_id.to_string()).bind(user_id.to_string())
            .execute(&mut *transaction).await?;
    }
    transaction.commit().await?;
    Ok(inserted)
}

#[derive(Debug, sqlx::FromRow)]
struct EventRow {
    scheduled_event_id: String,
    kind: String,
}

#[derive(Debug, sqlx::FromRow)]
struct AttendanceRow {
    scheduled_event_id: String,
    user_id: String,
    accumulated_seconds: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct IntervalRow {
    scheduled_event_id: String,
    user_id: String,
    started_at: i64,
    ended_at: i64,
}

pub async fn finalize_game(
    pool: &SqlitePool,
    guild_id: GuildId,
    game_key: &str,
    now: i64,
) -> Result<bool> {
    let mut transaction = pool.begin().await?;
    // ponytail: serialize rare finalization per Guild with SQLite's write lock.
    sqlx::query("UPDATE community_activity SET updated_at = updated_at WHERE guild_id = ? AND COALESCE(game_key, 'community') = ?")
        .bind(guild_id.to_string()).bind(game_key).execute(&mut *transaction).await?;
    let open: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_activity WHERE guild_id = ? AND COALESCE(game_key, 'community') = ? AND state IN ('scheduled', 'active')")
        .bind(guild_id.to_string()).bind(game_key).fetch_one(&mut *transaction).await?;
    if open > 0 {
        transaction.commit().await?;
        return Ok(false);
    }
    let events: Vec<EventRow> = sqlx::query_as("SELECT scheduled_event_id, kind FROM community_activity WHERE guild_id = ? AND COALESCE(game_key, 'community') = ? AND state IN ('completed', 'canceled', 'deleted') AND finalized_at IS NULL ORDER BY CASE kind WHEN 'community' THEN 0 ELSE 1 END, scheduled_event_id")
        .bind(guild_id.to_string()).bind(game_key).fetch_all(&mut *transaction).await?;
    if events.is_empty() {
        transaction.commit().await?;
        return Ok(false);
    }
    let attendance: Vec<AttendanceRow> = sqlx::query_as("SELECT a.scheduled_event_id, a.user_id, a.accumulated_seconds FROM activity_attendance a JOIN community_activity c ON c.guild_id = a.guild_id AND c.scheduled_event_id = a.scheduled_event_id WHERE a.guild_id = ? AND COALESCE(c.game_key, 'community') = ? AND c.finalized_at IS NULL")
        .bind(guild_id.to_string()).bind(game_key).fetch_all(&mut *transaction).await?;
    let intervals: Vec<IntervalRow> = sqlx::query_as("SELECT i.scheduled_event_id, i.user_id, i.started_at, i.ended_at FROM activity_attendance_interval i JOIN community_activity c ON c.guild_id = i.guild_id AND c.scheduled_event_id = i.scheduled_event_id WHERE i.guild_id = ? AND COALESCE(c.game_key, 'community') = ? AND c.finalized_at IS NULL ORDER BY i.started_at, i.ended_at")
        .bind(guild_id.to_string()).bind(game_key).fetch_all(&mut *transaction).await?;
    let event_kind = events
        .iter()
        .map(|event| (event.scheduled_event_id.clone(), event.kind.as_str()))
        .collect::<HashMap<_, _>>();
    let users = attendance
        .iter()
        .map(|row| row.user_id.clone())
        .collect::<HashSet<_>>();
    for user in users {
        let qualifying = attendance
            .iter()
            .filter(|row| row.user_id == user && row.accumulated_seconds >= QUALIFY_SECONDS)
            .map(|row| row.scheduled_event_id.clone())
            .collect::<HashSet<_>>();
        let mut ordered = events
            .iter()
            .filter(|event| qualifying.contains(&event.scheduled_event_id))
            .collect::<Vec<_>>();
        ordered.sort_by_key(|event| (event.kind != "community", &event.scheduled_event_id));
        let mut covered = Vec::new();
        let mut assigned = HashMap::new();
        for event in ordered {
            let event_intervals = intervals
                .iter()
                .filter(|interval| {
                    interval.user_id == user
                        && interval.scheduled_event_id == event.scheduled_event_id
                })
                .map(|interval| (interval.started_at, interval.ended_at))
                .collect::<Vec<_>>();
            assigned.insert(
                event.scheduled_event_id.clone(),
                assign_uncovered(&mut covered, &event_intervals),
            );
        }
        for row in attendance.iter().filter(|row| row.user_id == user) {
            let minutes = assigned.get(&row.scheduled_event_id).copied().unwrap_or(0) / 60;
            let mut credit = i64::from(qualifying.contains(&row.scheduled_event_id));
            if credit == 1 && event_kind.get(&row.scheduled_event_id) == Some(&"game") {
                let overlaps_scheduled = intervals
                    .iter()
                    .filter(|interval| {
                        interval.user_id == user
                            && interval.scheduled_event_id == row.scheduled_event_id
                    })
                    .any(|ad_hoc| {
                        intervals.iter().any(|scheduled| {
                            scheduled.user_id == user
                                && event_kind.get(&scheduled.scheduled_event_id)
                                    == Some(&"community")
                                && qualifying.contains(&scheduled.scheduled_event_id)
                                && overlaps(
                                    ad_hoc.started_at,
                                    ad_hoc.ended_at,
                                    scheduled.started_at,
                                    scheduled.ended_at,
                                )
                        })
                    });
                if overlaps_scheduled {
                    credit = 0;
                }
            }
            let source = format!("activity:{}", row.scheduled_event_id);
            let inserted = sqlx::query("INSERT INTO activity_completion (guild_id, source_key, user_id, game_key, play_minutes, session_credit) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT DO NOTHING")
                .bind(guild_id.to_string()).bind(source).bind(&user).bind(game_key)
                .bind(minutes).bind(credit).execute(&mut *transaction).await?.rows_affected();
            if inserted == 1 {
                sqlx::query("INSERT INTO activity_member_game_aggregate (guild_id, user_id, game_key, play_minutes, session_credits) VALUES (?, ?, ?, ?, ?) ON CONFLICT(guild_id, user_id, game_key) DO UPDATE SET play_minutes = play_minutes + excluded.play_minutes, session_credits = session_credits + excluded.session_credits")
                    .bind(guild_id.to_string()).bind(&user).bind(game_key).bind(minutes).bind(credit)
                    .execute(&mut *transaction).await?;
            }
        }
        sqlx::query("INSERT INTO activity_member_aggregate (guild_id, user_id, play_minutes, session_credits) SELECT ?, ?, COALESCE(SUM(play_minutes), 0), COALESCE(SUM(session_credits), 0) FROM activity_member_game_aggregate WHERE guild_id = ? AND user_id = ? ON CONFLICT(guild_id, user_id) DO UPDATE SET play_minutes = excluded.play_minutes, session_credits = excluded.session_credits")
            .bind(guild_id.to_string()).bind(&user).bind(guild_id.to_string()).bind(&user)
            .execute(&mut *transaction).await?;
    }
    for event in &events {
        sqlx::query("UPDATE community_activity SET finalized_at = ? WHERE guild_id = ? AND scheduled_event_id = ? AND finalized_at IS NULL")
            .bind(now).bind(guild_id.to_string()).bind(&event.scheduled_event_id).execute(&mut *transaction).await?;
        sqlx::query("DELETE FROM activity_attendance_interval WHERE guild_id = ? AND scheduled_event_id = ?")
            .bind(guild_id.to_string()).bind(&event.scheduled_event_id).execute(&mut *transaction).await?;
        sqlx::query(
            "DELETE FROM activity_attendance WHERE guild_id = ? AND scheduled_event_id = ?",
        )
        .bind(guild_id.to_string())
        .bind(&event.scheduled_event_id)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;
    Ok(true)
}

fn overlaps(a_start: i64, a_end: i64, b_start: i64, b_end: i64) -> bool {
    a_start < b_end && b_start < a_end
}

fn assign_uncovered(covered: &mut Vec<(i64, i64)>, intervals: &[(i64, i64)]) -> i64 {
    let mut seconds = 0;
    for &(start, end) in intervals {
        let mut cursor = start;
        covered.sort_unstable();
        for &(covered_start, covered_end) in covered.iter() {
            if covered_end <= cursor || covered_start >= end {
                continue;
            }
            seconds += (covered_start.min(end) - cursor).max(0);
            cursor = cursor.max(covered_end);
            if cursor >= end {
                break;
            }
        }
        seconds += (end - cursor).max(0);
        covered.push((start, end));
        merge_intervals(covered);
    }
    seconds
}

fn merge_intervals(intervals: &mut Vec<(i64, i64)>) {
    intervals.sort_unstable();
    let mut merged: Vec<(i64, i64)> = Vec::new();
    for &(start, end) in intervals.iter() {
        if let Some(last) = merged.last_mut()
            && start <= last.1
        {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }
    *intervals = merged;
}

pub fn activity_level(total_minutes: i64) -> u64 {
    let minutes = total_minutes.max(0) as u128;
    let mut low = 0_u128;
    let mut high = 1_u128;
    while high * (high + 1) * 30 <= minutes {
        high *= 2;
    }
    while low + 1 < high {
        let middle = (low + high) / 2;
        if middle * (middle + 1) * 30 <= minutes {
            low = middle;
        } else {
            high = middle;
        }
    }
    low as u64
}

#[cfg(test)]
mod tests {
    use super::{
        activity_level, add_session_credit, assign_uncovered, finalize_activity, overlaps,
    };
    use crate::community::{create_activity, create_game_activity, update_activity_extension};
    use crate::database::init_db;
    use poise::serenity_prelude::{GuildId, ScheduledEventId, ScheduledEventType, UserId};

    #[test]
    fn assigns_overlap_once_and_scheduled_first() {
        let mut covered = Vec::new();
        assert_eq!(assign_uncovered(&mut covered, &[(0, 2_000)]), 2_000);
        assert_eq!(assign_uncovered(&mut covered, &[(1_000, 3_000)]), 1_000);
        assert!(overlaps(0, 2_000, 1_000, 3_000));
        assert!(!overlaps(0, 1_000, 1_000, 2_000));
    }

    #[test]
    fn derives_integer_level_boundaries() {
        assert_eq!(activity_level(59), 0);
        assert_eq!(activity_level(60), 1);
        assert_eq!(activity_level(179), 1);
        assert_eq!(activity_level(180), 2);
        assert_eq!(activity_level(359), 2);
        assert_eq!(activity_level(360), 3);
    }

    #[tokio::test]
    async fn qualifies_at_thirty_minutes_and_resolves_overlap_in_reverse_order() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-aggregate-test-{}-{}",
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
        let scheduled = ScheduledEventId::new(10);
        let ad_hoc = ScheduledEventId::new(11);
        create_activity(
            &pool,
            guild,
            scheduled,
            ScheduledEventType::Voice,
            Some(UserId::new(9)),
            Some("minecraft"),
            None,
        )
        .await
        .unwrap();
        create_game_activity(
            &pool,
            guild,
            ad_hoc,
            ScheduledEventType::Voice,
            "minecraft",
            chrono::Utc::now().timestamp() + 10_000,
        )
        .await
        .unwrap();
        for (event, user, seconds, start, end) in [
            (10, 3, 2_000, 0, 2_000),
            (11, 3, 2_000, 0, 2_000),
            (10, 4, 1_799, 0, 1_799),
            (10, 5, 1_859, 0, 1_859),
            (10, 6, 1_800, 0, 1_800),
            (11, 6, 1_800, 900, 2_700),
        ] {
            sqlx::query("INSERT INTO activity_attendance (guild_id, scheduled_event_id, user_id, accumulated_seconds) VALUES ('1', ?, ?, ?)")
                .bind(event.to_string()).bind(user.to_string()).bind(seconds).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO activity_attendance_interval (guild_id, scheduled_event_id, user_id, started_at, ended_at) VALUES ('1', ?, ?, ?, ?)")
                .bind(event.to_string()).bind(user.to_string()).bind(start).bind(end).execute(&pool).await.unwrap();
        }
        update_activity_extension(&pool, guild, ad_hoc, None, Some("completed"))
            .await
            .unwrap();
        assert!(
            !finalize_activity(&pool, guild, ad_hoc, 3_000)
                .await
                .unwrap()
        );
        update_activity_extension(&pool, guild, scheduled, None, Some("completed"))
            .await
            .unwrap();
        assert!(
            finalize_activity(&pool, guild, scheduled, 3_000)
                .await
                .unwrap()
        );
        assert!(
            !finalize_activity(&pool, guild, scheduled, 3_000)
                .await
                .unwrap()
        );
        let totals: Vec<(String, i64, i64)> = sqlx::query_as("SELECT user_id, play_minutes, session_credits FROM activity_member_aggregate WHERE guild_id = '1' ORDER BY user_id")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(
            totals,
            vec![
                ("3".into(), 33, 1),
                ("4".into(), 0, 0),
                ("5".into(), 30, 1),
                ("6".into(), 45, 1)
            ]
        );
        let raw: i64 = sqlx::query_scalar("SELECT (SELECT COUNT(*) FROM activity_attendance) + (SELECT COUNT(*) FROM activity_attendance_interval)")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(raw, 0);
        let mismatches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM activity_member_aggregate t WHERE t.play_minutes != (SELECT COALESCE(SUM(g.play_minutes), 0) FROM activity_member_game_aggregate g WHERE g.guild_id = t.guild_id AND g.user_id = t.user_id)")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(mismatches, 0);
        assert!(
            add_session_credit(&pool, guild, UserId::new(3), "word-puzzle", "puzzle:1")
                .await
                .unwrap()
        );
        assert!(
            !add_session_credit(&pool, guild, UserId::new(3), "word-puzzle", "puzzle:1")
                .await
                .unwrap()
        );
        let total: (i64, i64) = sqlx::query_as("SELECT play_minutes, session_credits FROM activity_member_aggregate WHERE guild_id = '1' AND user_id = '3'")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(total, (33, 2));
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
