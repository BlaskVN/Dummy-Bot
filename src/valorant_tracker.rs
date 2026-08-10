use anyhow::{Result, bail};
use poise::serenity_prelude::UserId;
use sqlx::SqlitePool;

pub fn normalize_tracker_url(input: &str) -> Result<String> {
    let mut url = reqwest::Url::parse(input)?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.host_str() != Some("tracker.gg")
        || url.port().is_some()
        || url.fragment().is_some()
    {
        bail!("Not a supported Tracker profile URL");
    }
    let segments: Vec<_> = url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("Profile URL has no path"))?
        .collect();
    if segments.len() != 5
        || segments[..3] != ["valorant", "profile", "riot"]
        || segments[4] != "overview"
    {
        bail!("Not a Tracker VALORANT Riot profile path");
    }
    let riot_id = segments[3];
    if !valid_percent_encoding(riot_id)
        || riot_id.to_ascii_lowercase().contains("%2f")
        || riot_id.to_ascii_lowercase().contains("%5c")
    {
        bail!("Invalid encoded Riot ID");
    }
    let lowercase = riot_id.to_ascii_lowercase();
    let separators: Vec<_> = lowercase.match_indices("%23").collect();
    if separators.len() != 1 || separators[0].0 == 0 || separators[0].0 + 3 == riot_id.len() {
        bail!("Riot ID must contain an encoded name and tag");
    }
    url.set_query(None);
    Ok(url.to_string())
}

pub async fn set_profile(pool: &SqlitePool, user_id: UserId, input: &str) -> Result<String> {
    let url = normalize_tracker_url(input)?;
    sqlx::query("INSERT INTO valorant_tracker_profile (user_id, url) VALUES (?, ?) ON CONFLICT(user_id) DO UPDATE SET url = excluded.url, updated_at = CURRENT_TIMESTAMP")
        .bind(user_id.to_string()).bind(&url).execute(pool).await?;
    Ok(url)
}

pub async fn profile(pool: &SqlitePool, user_id: UserId) -> Result<Option<String>> {
    Ok(
        sqlx::query_scalar("SELECT url FROM valorant_tracker_profile WHERE user_id = ?")
            .bind(user_id.to_string())
            .fetch_optional(pool)
            .await?,
    )
}

pub async fn remove_profile(pool: &SqlitePool, user_id: UserId) -> Result<bool> {
    Ok(
        sqlx::query("DELETE FROM valorant_tracker_profile WHERE user_id = ?")
            .bind(user_id.to_string())
            .execute(pool)
            .await?
            .rows_affected()
            == 1,
    )
}

fn valid_percent_encoding(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            if index + 2 >= bytes.len()
                || !bytes[index + 1].is_ascii_hexdigit()
                || !bytes[index + 2].is_ascii_hexdigit()
            {
                return false;
            }
            index += 3;
        } else {
            index += 1;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{normalize_tracker_url, profile, remove_profile, set_profile};
    use crate::database::{delete_guild_data, init_db};
    use poise::serenity_prelude::{GuildId, UserId};

    #[test]
    fn accepts_only_normalized_tracker_riot_profile_urls() {
        let valid = [
            (
                "https://tracker.gg/valorant/profile/riot/Name%23NA1/overview",
                "https://tracker.gg/valorant/profile/riot/Name%23NA1/overview",
            ),
            (
                "https://tracker.gg:443/valorant/profile/riot/clairo%20fan%2380808/overview?platform=pc",
                "https://tracker.gg/valorant/profile/riot/clairo%20fan%2380808/overview",
            ),
            (
                "https://tracker.gg/valorant/profile/riot/%D1%84%D1%83%D1%85%23TAG/overview",
                "https://tracker.gg/valorant/profile/riot/%D1%84%D1%83%D1%85%23TAG/overview",
            ),
        ];
        for (input, expected) in valid {
            assert_eq!(normalize_tracker_url(input).unwrap(), expected);
        }
        for invalid in [
            "http://tracker.gg/valorant/profile/riot/Name%23NA1/overview",
            "https://user@tracker.gg/valorant/profile/riot/Name%23NA1/overview",
            "https://tracker.gg:444/valorant/profile/riot/Name%23NA1/overview",
            "https://tracker.gg/valorant/profile/riot/Name%23NA1/overview#stats",
            "https://tracker.gg.evil.test/valorant/profile/riot/Name%23NA1/overview",
            "https://evil.tracker.gg/valorant/profile/riot/Name%23NA1/overview",
            "https://tracker.gg/valorant/profile/riot/Name%23NA1/matches",
            "https://tracker.gg/valorant/profile/riot/Name%23NA1/overview/extra",
            "https://tracker.gg/valorant/profile/riot/Name/overview",
            "https://tracker.gg/valorant/profile/riot/%23NA1/overview",
            "https://tracker.gg/valorant/profile/riot/Name%23/overview",
            "https://tracker.gg/valorant/profile/riot/Name%23NA1%23X/overview",
            "https://tracker.gg/valorant/profile/riot/Name%2FMore%23NA1/overview",
            "https://tracker.gg/valorant/profile/riot/Name#NA1/overview",
        ] {
            assert!(
                normalize_tracker_url(invalid).is_err(),
                "accepted {invalid}"
            );
        }
    }

    #[tokio::test]
    async fn replaces_one_global_profile_and_survives_guild_cleanup() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-tracker-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        let user = UserId::new(2);
        set_profile(
            &pool,
            user,
            "https://tracker.gg/valorant/profile/riot/First%23ONE/overview",
        )
        .await
        .unwrap();
        let replacement = set_profile(
            &pool,
            user,
            "https://tracker.gg/valorant/profile/riot/Second%23TWO/overview",
        )
        .await
        .unwrap();
        assert_eq!(
            profile(&pool, user).await.unwrap().as_deref(),
            Some(replacement.as_str())
        );
        delete_guild_data(&pool, GuildId::new(1)).await.unwrap();
        assert_eq!(
            profile(&pool, user).await.unwrap().as_deref(),
            Some(replacement.as_str())
        );
        assert!(remove_profile(&pool, user).await.unwrap());
        assert!(!remove_profile(&pool, user).await.unwrap());
        assert!(profile(&pool, user).await.unwrap().is_none());
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
