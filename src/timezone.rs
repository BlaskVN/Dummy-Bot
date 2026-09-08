use anyhow::{Result, bail};
use chrono::{DateTime, Duration, LocalResult, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use poise::serenity_prelude::GuildId;
use sqlx::SqlitePool;
use std::str::FromStr;

pub fn parse(name: &str) -> Option<Tz> {
    Tz::from_str(name).ok()
}

pub fn next_five_am(now: DateTime<Utc>, timezone: Tz) -> Option<DateTime<Utc>> {
    let local_now = now.with_timezone(&timezone);
    let mut date = local_now.date_naive();
    if local_now.time() >= NaiveTime::from_hms_opt(5, 0, 0).expect("valid time") {
        date += Duration::days(1);
    }

    let local = date.and_hms_opt(5, 0, 0).expect("valid time");
    match timezone.from_local_datetime(&local) {
        LocalResult::Single(value) | LocalResult::Ambiguous(value, _) => {
            Some(value.with_timezone(&Utc))
        }
        LocalResult::None => {
            let mut candidate = local;
            for _ in 0..(48 * 60) {
                candidate += Duration::minutes(1);
                match timezone.from_local_datetime(&candidate) {
                    LocalResult::Single(value) | LocalResult::Ambiguous(value, _) => {
                        return Some(value.with_timezone(&Utc));
                    }
                    LocalResult::None => {}
                }
            }
            None
        }
    }
}

/// Retrieve the parsed IANA time zone for a guild.
pub async fn get_timezone(pool: &SqlitePool, guild_id: GuildId) -> Result<Option<Tz>> {
    let name = get_timezone_name(pool, guild_id).await?;
    Ok(name.as_deref().and_then(parse))
}

/// Retrieve the configured IANA time zone name string for a guild.
pub async fn get_timezone_name(pool: &SqlitePool, guild_id: GuildId) -> Result<Option<String>> {
    let result = sqlx::query_scalar::<_, Option<String>>(
        "SELECT iana_name FROM guild_timezone WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(result.flatten())
}

/// Validate and persist a guild's IANA time zone.
pub async fn set_timezone(pool: &SqlitePool, guild_id: GuildId, iana_name: &str) -> Result<()> {
    if parse(iana_name).is_none() {
        bail!("Invalid IANA time zone: {iana_name}");
    }

    sqlx::query(
        "INSERT INTO guild_timezone (guild_id, iana_name) VALUES (?, ?)
         ON CONFLICT(guild_id) DO UPDATE SET iana_name = excluded.iana_name, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .bind(iana_name)
    .execute(pool)
    .await?;

    Ok(())
}

/// Reset a guild's configured time zone to default.
pub async fn clear_timezone(pool: &SqlitePool, guild_id: GuildId) -> Result<()> {
    sqlx::query(
        "INSERT INTO guild_timezone (guild_id, iana_name) VALUES (?, NULL)
         ON CONFLICT(guild_id) DO UPDATE SET iana_name = NULL, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .execute(pool)
    .await?;

    Ok(())
}

/// Calculate the next 05:00 session expiry for a guild in its configured time zone.
pub async fn next_session_expiry(
    pool: &SqlitePool,
    guild_id: GuildId,
    now: DateTime<Utc>,
) -> Result<Option<DateTime<Utc>>> {
    let Some(tz) = get_timezone(pool, guild_id).await? else {
        return Ok(None);
    };

    Ok(next_five_am(now, tz))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    async fn setup_test_pool() -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-tz-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let url = format!("sqlite:{}/bot.db?mode=rwc", directory.display());
        let pool = crate::database::init_db(&url, &directory).await.unwrap();
        (pool, directory)
    }

    #[test]
    fn accepts_iana_names_but_not_offsets() {
        assert!(parse("Asia/Bangkok").is_some());
        assert!(parse("America/New_York").is_some());
        assert!(parse("UTC+7").is_none());
        assert!(parse("Not/AZone").is_none());
    }

    #[test]
    fn next_boundary_crosses_dst() {
        let now = Utc.with_ymd_and_hms(2025, 3, 9, 6, 0, 0).unwrap();
        let next = next_five_am(now, parse("America/New_York").unwrap()).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2025, 3, 9, 9, 0, 0).unwrap());
    }

    #[test]
    fn next_boundary_handles_a_skipped_day() {
        let now = Utc.with_ymd_and_hms(2011, 12, 29, 18, 0, 0).unwrap();
        let next = next_five_am(now, parse("Pacific/Apia").unwrap()).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2011, 12, 30, 10, 0, 0).unwrap());
    }

    #[tokio::test]
    async fn set_timezone_rejects_invalid_iana_without_persisting() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(100);

        let result = set_timezone(&pool, guild_id, "UTC+7").await;
        assert!(result.is_err());

        let configured = get_timezone_name(&pool, guild_id).await.unwrap();
        assert!(configured.is_none());

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn set_and_get_timezone_roundtrip() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(200);

        set_timezone(&pool, guild_id, "Asia/Bangkok").await.unwrap();

        let name = get_timezone_name(&pool, guild_id).await.unwrap();
        assert_eq!(name.as_deref(), Some("Asia/Bangkok"));

        let tz = get_timezone(&pool, guild_id).await.unwrap();
        assert_eq!(tz, Some(chrono_tz::Asia::Bangkok));

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn clear_timezone_resets_configuration() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(300);

        set_timezone(&pool, guild_id, "America/New_York")
            .await
            .unwrap();
        clear_timezone(&pool, guild_id).await.unwrap();

        let name = get_timezone_name(&pool, guild_id).await.unwrap();
        assert!(name.is_none());

        let tz = get_timezone(&pool, guild_id).await.unwrap();
        assert!(tz.is_none());

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn next_session_expiry_calculates_boundary_for_guild() {
        let (pool, dir) = setup_test_pool().await;
        let guild_id = GuildId::new(400);

        // Before configuration, returns None
        let now = Utc.with_ymd_and_hms(2025, 3, 9, 6, 0, 0).unwrap();
        let unconfigured = next_session_expiry(&pool, guild_id, now).await.unwrap();
        assert!(unconfigured.is_none());

        // Once configured, calculates boundary
        set_timezone(&pool, guild_id, "America/New_York")
            .await
            .unwrap();
        let expiry = next_session_expiry(&pool, guild_id, now).await.unwrap();
        assert_eq!(
            expiry,
            Some(Utc.with_ymd_and_hms(2025, 3, 9, 9, 0, 0).unwrap())
        );

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
