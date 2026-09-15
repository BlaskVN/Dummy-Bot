use anyhow::Result;
use poise::serenity_prelude::{GuildId, UserId};
use sqlx::SqlitePool;
use std::sync::Arc;

use super::api::{
    LolApiClient, LolApiError, LolChampionMastery, LolLeagueEntry, LolPlatform, LolQueueType,
    LolRecentMatch, LolSummoner,
};
use crate::valorant::storage::{LinkedRiotAccount, get_guild_visibility, get_linked_account};

#[derive(Debug, Clone, PartialEq)]
pub struct LolProfile {
    pub account: LinkedRiotAccount,
    pub summoner: LolSummoner,
    pub solo_entry: Option<LolLeagueEntry>,
    pub flex_entry: Option<LolLeagueEntry>,
    pub is_self: bool,
    pub is_visible: bool,
}

#[derive(Debug)]
pub enum LolDomainError {
    NotLinked { is_self: bool },
    HiddenOther,
    ApiForbidden,
    ApiNotFound,
    ApiError(anyhow::Error),
    Database(anyhow::Error),
}

impl std::fmt::Display for LolDomainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLinked { is_self } => {
                write!(f, "LoL account not linked (is_self: {is_self})")
            }
            Self::HiddenOther => write!(f, "Target member profile is hidden in this guild"),
            Self::ApiForbidden => write!(f, "LoL API access forbidden"),
            Self::ApiNotFound => write!(f, "LoL summoner not found on platform"),
            Self::ApiError(err) => write!(f, "LoL API error: {err}"),
            Self::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}

impl std::error::Error for LolDomainError {}

pub type LolProfileError = LolDomainError;
pub type LolMatchesError = LolDomainError;
pub type LolMasteryError = LolDomainError;

#[derive(Debug, Clone, PartialEq)]
pub struct LolRecentMatchesData {
    pub account: LinkedRiotAccount,
    pub matches: Vec<LolRecentMatch>,
    pub is_self: bool,
    pub is_visible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LolMasteryData {
    pub account: LinkedRiotAccount,
    pub top_masteries: Vec<LolChampionMastery>,
    pub total_score: i32,
    pub is_self: bool,
    pub is_visible: bool,
}

/// Domain service for League of Legends player profiles, match history, and champion masteries.
#[derive(Clone)]
pub struct LolService {
    pool: SqlitePool,
    api: Arc<dyn LolApiClient>,
}

impl LolService {
    pub fn new(pool: SqlitePool, api: Arc<dyn LolApiClient>) -> Self {
        Self { pool, api }
    }

    /// Resolve the target member's linked Riot account and verify guild privacy settings.
    async fn resolve_target_and_visibility(
        &self,
        guild_id: GuildId,
        requester_id: UserId,
        target_id: UserId,
    ) -> Result<(LinkedRiotAccount, bool), LolDomainError> {
        let is_self = requester_id == target_id;

        let account = match get_linked_account(&self.pool, target_id)
            .await
            .map_err(LolDomainError::Database)?
        {
            Some(acc) => acc,
            None => return Err(LolDomainError::NotLinked { is_self }),
        };

        let is_visible = get_guild_visibility(&self.pool, guild_id, target_id)
            .await
            .map_err(LolDomainError::Database)?;

        if !is_self && !is_visible {
            return Err(LolDomainError::HiddenOther);
        }

        Ok((account, is_visible))
    }

    /// Retrieve a member's League of Legends profile with summoner and ranked queue entries.
    pub async fn get_profile(
        &self,
        guild_id: GuildId,
        requester_id: UserId,
        target_id: UserId,
        platform_override: Option<LolPlatform>,
    ) -> Result<LolProfile, LolProfileError> {
        let is_self = requester_id == target_id;
        let (account, is_visible) = self
            .resolve_target_and_visibility(guild_id, requester_id, target_id)
            .await?;

        let platform =
            platform_override.unwrap_or_else(|| LolPlatform::from_riot_region(account.region));

        let summoner = match self
            .api
            .get_summoner_by_puuid(platform, &account.puuid)
            .await
        {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(
                    err = %err,
                    puuid = %account.puuid,
                    user_id = %target_id,
                    "Failed to load LoL summoner data"
                );
                match err.downcast_ref::<LolApiError>() {
                    Some(LolApiError::Forbidden) => return Err(LolProfileError::ApiForbidden),
                    Some(LolApiError::NotFound) => return Err(LolProfileError::ApiNotFound),
                    _ => return Err(LolProfileError::ApiError(err)),
                }
            }
        };

        let league_entries = match self
            .api
            .get_league_entries(platform, &summoner.summoner_id, &account.puuid)
            .await
        {
            Ok(entries) => entries,
            Err(err) => {
                tracing::warn!(
                    err = %err,
                    summoner_id = %summoner.summoner_id,
                    "Failed to load LoL league entries"
                );
                match err.downcast_ref::<LolApiError>() {
                    Some(LolApiError::Forbidden) => return Err(LolProfileError::ApiForbidden),
                    _ => return Err(LolProfileError::ApiError(err)),
                }
            }
        };

        let mut solo_entry = None;
        let mut flex_entry = None;

        for entry in league_entries {
            match entry.queue_type {
                LolQueueType::SoloDuo => solo_entry = Some(entry),
                LolQueueType::Flex => flex_entry = Some(entry),
                LolQueueType::Other(_) => {}
            }
        }

        Ok(LolProfile {
            account,
            summoner,
            solo_entry,
            flex_entry,
            is_self,
            is_visible,
        })
    }

    /// Retrieve a member's recent League of Legends matches.
    pub async fn get_matches(
        &self,
        guild_id: GuildId,
        requester_id: UserId,
        target_id: UserId,
        count: usize,
        platform_override: Option<LolPlatform>,
    ) -> Result<LolRecentMatchesData, LolMatchesError> {
        let is_self = requester_id == target_id;
        let (account, is_visible) = self
            .resolve_target_and_visibility(guild_id, requester_id, target_id)
            .await?;

        let platform =
            platform_override.unwrap_or_else(|| LolPlatform::from_riot_region(account.region));

        let matches = match self
            .api
            .get_recent_matches(platform, &account.puuid, count)
            .await
        {
            Ok(m) => m,
            Err(err) => {
                tracing::warn!(
                    err = %err,
                    puuid = %account.puuid,
                    "Failed to load recent LoL matches"
                );
                match err.downcast_ref::<LolApiError>() {
                    Some(LolApiError::Forbidden) => return Err(LolMatchesError::ApiForbidden),
                    Some(LolApiError::NotFound) => return Err(LolMatchesError::ApiNotFound),
                    _ => return Err(LolMatchesError::ApiError(err)),
                }
            }
        };

        Ok(LolRecentMatchesData {
            account,
            matches,
            is_self,
            is_visible,
        })
    }

    /// Retrieve a member's top champion masteries and total mastery score.
    pub async fn get_mastery(
        &self,
        guild_id: GuildId,
        requester_id: UserId,
        target_id: UserId,
        count: usize,
        platform_override: Option<LolPlatform>,
    ) -> Result<LolMasteryData, LolMasteryError> {
        let is_self = requester_id == target_id;
        let (account, is_visible) = self
            .resolve_target_and_visibility(guild_id, requester_id, target_id)
            .await?;

        let platform =
            platform_override.unwrap_or_else(|| LolPlatform::from_riot_region(account.region));

        let (masteries_res, score_res) = tokio::join!(
            self.api
                .get_top_champion_masteries(platform, &account.puuid, count),
            self.api.get_total_mastery_score(platform, &account.puuid)
        );

        let top_masteries = match masteries_res {
            Ok(m) => m,
            Err(err) => {
                tracing::warn!(
                    err = %err,
                    puuid = %account.puuid,
                    "Failed to load LoL champion masteries"
                );
                match err.downcast_ref::<LolApiError>() {
                    Some(LolApiError::Forbidden) => return Err(LolMasteryError::ApiForbidden),
                    Some(LolApiError::NotFound) => return Err(LolMasteryError::ApiNotFound),
                    _ => return Err(LolMasteryError::ApiError(err)),
                }
            }
        };

        let total_score = match score_res {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(
                    err = %err,
                    puuid = %account.puuid,
                    "Failed to load LoL total mastery score"
                );
                match err.downcast_ref::<LolApiError>() {
                    Some(LolApiError::Forbidden) => return Err(LolMasteryError::ApiForbidden),
                    Some(LolApiError::NotFound) => return Err(LolMasteryError::ApiNotFound),
                    _ => return Err(LolMasteryError::ApiError(err)),
                }
            }
        };

        Ok(LolMasteryData {
            account,
            top_masteries,
            total_score,
            is_self,
            is_visible,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lol::api::{BoxFuture, MockLolApiClient};
    use crate::valorant::riot_api::RiotRegion;
    use crate::valorant::storage::{set_guild_visibility, set_linked_account};

    async fn setup_test_pool() -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-lol-service-test-{}-{}",
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

    struct FailingLolClient {
        forbidden: bool,
        not_found: bool,
    }

    impl LolApiClient for FailingLolClient {
        fn get_summoner_by_puuid<'a>(
            &'a self,
            _platform: LolPlatform,
            _puuid: &'a str,
        ) -> BoxFuture<'a, Result<LolSummoner>> {
            Box::pin(async move {
                if self.forbidden {
                    Err(anyhow::Error::new(LolApiError::Forbidden))
                } else if self.not_found {
                    Err(anyhow::Error::new(LolApiError::NotFound))
                } else {
                    Err(anyhow::anyhow!("generic failure"))
                }
            })
        }

        fn get_league_entries<'a>(
            &'a self,
            _platform: LolPlatform,
            _summoner_id: &'a str,
            _puuid: &'a str,
        ) -> BoxFuture<'a, Result<Vec<LolLeagueEntry>>> {
            Box::pin(async move { Err(anyhow::Error::new(LolApiError::Forbidden)) })
        }

        fn get_recent_matches<'a>(
            &'a self,
            _platform: LolPlatform,
            _puuid: &'a str,
            _count: usize,
        ) -> BoxFuture<'a, Result<Vec<LolRecentMatch>>> {
            Box::pin(async move { Err(anyhow::Error::new(LolApiError::Forbidden)) })
        }

        fn get_top_champion_masteries<'a>(
            &'a self,
            _platform: LolPlatform,
            _puuid: &'a str,
            _count: usize,
        ) -> BoxFuture<'a, Result<Vec<LolChampionMastery>>> {
            Box::pin(async move { Err(anyhow::Error::new(LolApiError::Forbidden)) })
        }

        fn get_total_mastery_score<'a>(
            &'a self,
            _platform: LolPlatform,
            _puuid: &'a str,
        ) -> BoxFuture<'a, Result<i32>> {
            Box::pin(async move { Err(anyhow::Error::new(LolApiError::Forbidden)) })
        }
    }

    #[tokio::test]
    async fn profile_self_vs_other_visibility_and_unlinked() {
        let (pool, dir) = setup_test_pool().await;
        let mock_client = Arc::new(MockLolApiClient);
        let service = LolService::new(pool.clone(), mock_client);

        let guild_id = GuildId::new(100);
        let requester_id = UserId::new(1);
        let target_id = UserId::new(2);

        // Neither linked
        let err = service
            .get_profile(guild_id, requester_id, requester_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolProfileError::NotLinked { is_self: true }));

        let err = service
            .get_profile(guild_id, requester_id, target_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolProfileError::NotLinked { is_self: false }));

        // Link target account (default visibility is false)
        set_linked_account(
            &pool,
            target_id,
            "puuid-target-lol",
            "Faker",
            "KR1",
            RiotRegion::Kr,
        )
        .await
        .unwrap();

        // Requester viewing target -> HiddenOther
        let err = service
            .get_profile(guild_id, requester_id, target_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolProfileError::HiddenOther));

        // Target viewing self -> allowed even though is_visible is false
        let self_profile = service
            .get_profile(guild_id, target_id, target_id, None)
            .await
            .unwrap();
        assert!(self_profile.is_self);
        assert!(!self_profile.is_visible);
        assert_eq!(self_profile.account.game_name, "Faker");
        assert!(self_profile.solo_entry.is_some());
        assert!(self_profile.flex_entry.is_some());

        // Target enables guild visibility
        set_guild_visibility(&pool, guild_id, target_id, true)
            .await
            .unwrap();

        // Requester viewing target now succeeds
        let other_profile = service
            .get_profile(guild_id, requester_id, target_id, None)
            .await
            .unwrap();
        assert!(!other_profile.is_self);
        assert!(other_profile.is_visible);
        assert_eq!(other_profile.account.game_name, "Faker");

        // Custom platform override
        let jp_profile = service
            .get_profile(guild_id, requester_id, target_id, Some(LolPlatform::Jp1))
            .await
            .unwrap();
        assert_eq!(jp_profile.account.game_name, "Faker");

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn matches_self_vs_other_and_privacy() {
        let (pool, dir) = setup_test_pool().await;
        let mock_client = Arc::new(MockLolApiClient);
        let service = LolService::new(pool.clone(), mock_client);

        let guild_id = GuildId::new(200);
        let requester_id = UserId::new(10);
        let target_id = UserId::new(20);

        // Not linked
        let err = service
            .get_matches(guild_id, requester_id, requester_id, 5, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolMatchesError::NotLinked { is_self: true }));

        // Link target
        set_linked_account(
            &pool,
            target_id,
            "puuid-target-20",
            "Gumayusi",
            "T1",
            RiotRegion::Kr,
        )
        .await
        .unwrap();

        // Hidden to other
        let err = service
            .get_matches(guild_id, requester_id, target_id, 5, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolMatchesError::HiddenOther));

        // Visible to self
        let self_matches = service
            .get_matches(guild_id, target_id, target_id, 3, None)
            .await
            .unwrap();
        assert_eq!(self_matches.matches.len(), 3);
        assert!(self_matches.is_self);

        // Enable visibility
        set_guild_visibility(&pool, guild_id, target_id, true)
            .await
            .unwrap();
        let other_matches = service
            .get_matches(guild_id, requester_id, target_id, 2, None)
            .await
            .unwrap();
        assert_eq!(other_matches.matches.len(), 2);
        assert!(!other_matches.is_self);
        assert!(other_matches.is_visible);

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn mastery_self_vs_other_and_privacy() {
        let (pool, dir) = setup_test_pool().await;
        let mock_client = Arc::new(MockLolApiClient);
        let service = LolService::new(pool.clone(), mock_client);

        let guild_id = GuildId::new(300);
        let requester_id = UserId::new(30);
        let target_id = UserId::new(40);

        // Not linked
        let err = service
            .get_mastery(guild_id, requester_id, requester_id, 5, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolMasteryError::NotLinked { is_self: true }));

        // Link target
        set_linked_account(
            &pool,
            target_id,
            "puuid-target-40",
            "Keria",
            "T1",
            RiotRegion::Kr,
        )
        .await
        .unwrap();

        // Hidden to other
        let err = service
            .get_mastery(guild_id, requester_id, target_id, 5, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolMasteryError::HiddenOther));

        // Visible to self
        let self_mastery = service
            .get_mastery(guild_id, target_id, target_id, 3, None)
            .await
            .unwrap();
        assert_eq!(self_mastery.top_masteries.len(), 3);
        assert_eq!(self_mastery.total_score, 145);
        assert!(self_mastery.is_self);

        // Enable visibility
        set_guild_visibility(&pool, guild_id, target_id, true)
            .await
            .unwrap();
        let other_mastery = service
            .get_mastery(guild_id, requester_id, target_id, 5, None)
            .await
            .unwrap();
        assert_eq!(other_mastery.top_masteries.len(), 5);
        assert!(!other_mastery.is_self);
        assert!(other_mastery.is_visible);

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn api_error_propagation() {
        let (pool, dir) = setup_test_pool().await;
        let user_id = UserId::new(50);
        let guild_id = GuildId::new(400);

        set_linked_account(
            &pool,
            user_id,
            "puuid-user-50",
            "Zeus",
            "HLE",
            RiotRegion::Kr,
        )
        .await
        .unwrap();

        // 1. Forbidden error
        let forbidden_client = Arc::new(FailingLolClient {
            forbidden: true,
            not_found: false,
        });
        let service = LolService::new(pool.clone(), forbidden_client);

        let err = service
            .get_profile(guild_id, user_id, user_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolProfileError::ApiForbidden));

        let err = service
            .get_matches(guild_id, user_id, user_id, 5, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolMatchesError::ApiForbidden));

        let err = service
            .get_mastery(guild_id, user_id, user_id, 5, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolMasteryError::ApiForbidden));

        // 2. NotFound error
        let not_found_client = Arc::new(FailingLolClient {
            forbidden: false,
            not_found: true,
        });
        let service = LolService::new(pool.clone(), not_found_client);

        let err = service
            .get_profile(guild_id, user_id, user_id, None)
            .await
            .unwrap_err();
        assert!(matches!(err, LolProfileError::ApiNotFound));

        pool.close().await;
        let _ = tokio::fs::remove_dir_all(&dir).await;
    }
}
