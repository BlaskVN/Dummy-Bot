use anyhow::Result;
use poise::serenity_prelude::UserId;
use sqlx::SqlitePool;

#[derive(Debug, PartialEq, Eq)]
pub enum TrackerUrlError {
    InsecureScheme,
    InvalidHost,
    UserInfoForbidden,
    NonDefaultPort,
    FragmentOrQueryForbidden,
    InvalidProfilePath,
    ParseError(String),
}

impl std::fmt::Display for TrackerUrlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InsecureScheme => write!(f, "URL must use HTTPS protocol"),
            Self::InvalidHost => write!(f, "Host must be tracker.gg"),
            Self::UserInfoForbidden => write!(f, "URL must not contain user credentials"),
            Self::NonDefaultPort => write!(f, "Non-default ports are not permitted"),
            Self::FragmentOrQueryForbidden => {
                write!(f, "URL fragments or queries are not permitted")
            }
            Self::InvalidProfilePath => {
                write!(
                    f,
                    "Path must follow /valorant/profile/riot/<name>%23<tag>[/overview]"
                )
            }
            Self::ParseError(msg) => write!(f, "Failed to parse URL: {msg}"),
        }
    }
}

impl std::error::Error for TrackerUrlError {}

/// Parse and normalize a Tracker Network VALORANT profile URL.
///
/// Enforces:
/// - Must use https://
/// - Host must strictly be "tracker.gg" or "www.tracker.gg"
/// - No user-info credentials (e.g. user:pass@)
/// - No non-default ports
/// - No query strings or fragments
/// - Path must begin with `/valorant/profile/riot/<riot_id>`
/// - Normalizes the URL to `https://tracker.gg/valorant/profile/riot/<riot_id>/overview`
pub fn parse_and_normalize_tracker_url(input: &str) -> Result<String, TrackerUrlError> {
    let input = input.trim();

    // Check fragments or queries upfront
    if input.contains('#') || input.contains('?') {
        return Err(TrackerUrlError::FragmentOrQueryForbidden);
    }

    // Basic scheme check (case-insensitive per RFC 3986)
    if input.len() < 8 || !input[..8].eq_ignore_ascii_case("https://") {
        return Err(TrackerUrlError::InsecureScheme);
    }

    // Strip scheme for parsing host & path
    let without_scheme = &input[8..];
    let (authority, path) = match without_scheme.find('/') {
        Some(idx) => (&without_scheme[..idx], &without_scheme[idx..]),
        None => return Err(TrackerUrlError::InvalidProfilePath),
    };

    // Check credentials
    if authority.contains('@') {
        return Err(TrackerUrlError::UserInfoForbidden);
    }

    // Check port
    if authority.contains(':') {
        return Err(TrackerUrlError::NonDefaultPort);
    }

    // Check host strictly
    let host = authority.to_ascii_lowercase();
    if host != "tracker.gg" && host != "www.tracker.gg" {
        return Err(TrackerUrlError::InvalidHost);
    }

    // Validate path structure: /valorant/profile/riot/<id>[/overview]
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 4 {
        return Err(TrackerUrlError::InvalidProfilePath);
    }

    if !segments[0].eq_ignore_ascii_case("valorant")
        || !segments[1].eq_ignore_ascii_case("profile")
        || !segments[2].eq_ignore_ascii_case("riot")
    {
        return Err(TrackerUrlError::InvalidProfilePath);
    }

    let riot_id = segments[3];
    let Some((name, tag)) = riot_id.split_once("%23") else {
        return Err(TrackerUrlError::InvalidProfilePath);
    };
    if name.is_empty() || tag.is_empty() {
        return Err(TrackerUrlError::InvalidProfilePath);
    }

    // Optional 5th segment must be a known tracker profile tab
    if segments.len() == 5
        && !["overview", "matches", "agents", "weapons"]
            .contains(&segments[4].to_ascii_lowercase().as_str())
    {
        return Err(TrackerUrlError::InvalidProfilePath);
    }
    if segments.len() > 5 {
        return Err(TrackerUrlError::InvalidProfilePath);
    }

    Ok(format!(
        "https://tracker.gg/valorant/profile/riot/{name}%23{tag}/overview"
    ))
}

/// Set a user's global VALORANT tracker profile URL.
pub async fn set_tracker_profile(
    pool: &SqlitePool,
    user_id: UserId,
    raw_url: &str,
) -> Result<String> {
    let normalized = parse_and_normalize_tracker_url(raw_url)?;

    sqlx::query(
        "INSERT INTO valorant_tracker_profile (user_id, url) VALUES (?, ?)
         ON CONFLICT(user_id) DO UPDATE SET url = excluded.url, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(user_id.to_string())
    .bind(&normalized)
    .execute(pool)
    .await?;

    Ok(normalized)
}

/// Retrieve a user's global VALORANT tracker profile URL.
pub async fn get_tracker_profile(pool: &SqlitePool, user_id: UserId) -> Result<Option<String>> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT url FROM valorant_tracker_profile WHERE user_id = ?",
    )
    .bind(user_id.to_string())
    .fetch_optional(pool)
    .await?;

    Ok(result)
}

/// Remove a user's global VALORANT tracker profile URL.
pub async fn remove_tracker_profile(pool: &SqlitePool, user_id: UserId) -> Result<bool> {
    let rows = sqlx::query("DELETE FROM valorant_tracker_profile WHERE user_id = ?")
        .bind(user_id.to_string())
        .execute(pool)
        .await?
        .rows_affected();

    Ok(rows > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_pool() -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-tracker-test-{}-{}",
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
    fn accepts_valid_tracker_urls() {
        let valid_inputs = [
            (
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/overview",
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/overview",
            ),
            (
                "https://tracker.gg/valorant/profile/riot/Player%23NA1",
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/overview",
            ),
            (
                "https://www.tracker.gg/valorant/profile/riot/TenZ%230001/overview",
                "https://tracker.gg/valorant/profile/riot/TenZ%230001/overview",
            ),
            (
                "https://tracker.gg/valorant/profile/riot/Space%20Name%23TAG/matches",
                "https://tracker.gg/valorant/profile/riot/Space%20Name%23TAG/overview",
            ),
            (
                "HTTPS://tracker.gg/valorant/profile/riot/Player%23NA1/overview",
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/overview",
            ),
        ];

        for (input, expected) in valid_inputs {
            let result = parse_and_normalize_tracker_url(input);
            assert_eq!(result.unwrap(), expected, "failed on input: {input}");
        }
    }

    #[test]
    fn rejects_invalid_and_malicious_urls() {
        let invalid_cases = [
            (
                "http://tracker.gg/valorant/profile/riot/Player%23NA1/overview",
                TrackerUrlError::InsecureScheme,
            ),
            (
                "https://attacker.com/valorant/profile/riot/Player%23NA1/overview",
                TrackerUrlError::InvalidHost,
            ),
            (
                "https://tracker.gg.attacker.com/valorant/profile/riot/Player%23NA1/overview",
                TrackerUrlError::InvalidHost,
            ),
            (
                "https://evil.tracker.gg/valorant/profile/riot/Player%23NA1/overview",
                TrackerUrlError::InvalidHost,
            ),
            (
                "https://tracker.gg:8080/valorant/profile/riot/Player%23NA1/overview",
                TrackerUrlError::NonDefaultPort,
            ),
            (
                "https://user:pass@tracker.gg/valorant/profile/riot/Player%23NA1/overview",
                TrackerUrlError::UserInfoForbidden,
            ),
            (
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/overview?token=secret",
                TrackerUrlError::FragmentOrQueryForbidden,
            ),
            (
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/overview#fragment",
                TrackerUrlError::FragmentOrQueryForbidden,
            ),
            (
                "https://tracker.gg/csgo/profile/steam/12345/overview",
                TrackerUrlError::InvalidProfilePath,
            ),
            (
                "https://tracker.gg/valorant/",
                TrackerUrlError::InvalidProfilePath,
            ),
            (
                "https://tracker.gg/valorant/profile/riot/PlayerWithoutTag/overview",
                TrackerUrlError::InvalidProfilePath,
            ),
            (
                "https://tracker.gg/valorant/profile/riot/Player%23/overview",
                TrackerUrlError::InvalidProfilePath,
            ),
            (
                "https://tracker.gg/valorant/profile/riot/%23Tag/overview",
                TrackerUrlError::InvalidProfilePath,
            ),
            (
                "https://tracker.gg/valorant/profile/riot/Player%23NA1/invalidtab",
                TrackerUrlError::InvalidProfilePath,
            ),
        ];

        for (input, expected_err) in invalid_cases {
            let result = parse_and_normalize_tracker_url(input);
            assert_eq!(
                result.unwrap_err(),
                expected_err,
                "failed on input: {input}"
            );
        }
    }

    #[tokio::test]
    async fn tracker_profile_crud_and_global_isolation() {
        let (pool, dir) = setup_test_pool().await;
        let user1 = UserId::new(12345);
        let user2 = UserId::new(67890);

        // Initially none
        assert!(get_tracker_profile(&pool, user1).await.unwrap().is_none());

        // Set user 1
        let url1 = "https://tracker.gg/valorant/profile/riot/Player1%23NA1";
        let saved1 = set_tracker_profile(&pool, user1, url1).await.unwrap();
        assert_eq!(
            saved1,
            "https://tracker.gg/valorant/profile/riot/Player1%23NA1/overview"
        );

        // Set user 2
        let url2 = "https://tracker.gg/valorant/profile/riot/Player2%23EUW";
        let saved2 = set_tracker_profile(&pool, user2, url2).await.unwrap();
        assert_eq!(
            saved2,
            "https://tracker.gg/valorant/profile/riot/Player2%23EUW/overview"
        );

        // Verify independent reads
        assert_eq!(
            get_tracker_profile(&pool, user1).await.unwrap(),
            Some(saved1)
        );
        assert_eq!(
            get_tracker_profile(&pool, user2).await.unwrap(),
            Some(saved2)
        );

        // Update user 1 replaces prior link
        let updated_url1 = "https://tracker.gg/valorant/profile/riot/Player1Updated%23NA1/overview";
        set_tracker_profile(&pool, user1, updated_url1)
            .await
            .unwrap();
        assert_eq!(
            get_tracker_profile(&pool, user1).await.unwrap(),
            Some(updated_url1.to_string())
        );

        // Remove user 1
        assert!(remove_tracker_profile(&pool, user1).await.unwrap());
        assert!(get_tracker_profile(&pool, user1).await.unwrap().is_none());

        // User 2 remains untouched
        assert_eq!(
            get_tracker_profile(&pool, user2).await.unwrap(),
            Some("https://tracker.gg/valorant/profile/riot/Player2%23EUW/overview".to_string())
        );

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
