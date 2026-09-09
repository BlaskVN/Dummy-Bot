use anyhow::Result;
use poise::serenity_prelude::{GuildId, UserId};
use sqlx::SqlitePool;
use std::sync::Arc;

use super::riot_api::{PlayerRankedData, RiotApiClient, RiotApiError, RiotRegion};
use super::storage::{
    LinkedRiotAccount, get_guild_visibility, get_linked_account, list_guild_visible_accounts,
    remove_linked_account, set_guild_visibility, set_linked_account,
};

#[derive(Debug)]
pub enum ValorantLinkError {
    InvalidFormat,
    InvalidRegion,
    AccountNotFound(String),
    ApiError(anyhow::Error),
    Database(anyhow::Error),
}

impl std::fmt::Display for ValorantLinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidFormat => write!(f, "Invalid Riot ID format (must be GameName#TAG)"),
            Self::InvalidRegion => write!(f, "Invalid region specified"),
            Self::AccountNotFound(id) => write!(f, "Riot account not found: {id}"),
            Self::ApiError(err) => write!(f, "Riot API error: {err}"),
            Self::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}

impl std::error::Error for ValorantLinkError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValorantProfile {
    pub account: LinkedRiotAccount,
    pub stats: PlayerRankedData,
    pub is_self: bool,
    pub is_visible: bool,
}

#[derive(Debug)]
pub enum ValorantProfileError {
    NotLinked { is_self: bool },
    HiddenOther,
    ApiForbidden,
    ApiUnranked,
    ApiError(anyhow::Error),
    Database(anyhow::Error),
}

impl std::fmt::Display for ValorantProfileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLinked { is_self } => {
                write!(f, "Account not linked (is_self: {is_self})")
            }
            Self::HiddenOther => write!(f, "User profile is private in this guild"),
            Self::ApiForbidden => write!(f, "Riot API access forbidden"),
            Self::ApiUnranked => write!(f, "Player has no ranked data in this episode/act"),
            Self::ApiError(err) => write!(f, "Failed to load player ranked data: {err}"),
            Self::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}

impl std::error::Error for ValorantProfileError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValorantLeaderboardEntry {
    pub account: LinkedRiotAccount,
    pub stats: PlayerRankedData,
}

#[derive(Debug)]
pub enum ValorantLeaderboardError {
    Empty,
    ApiForbidden,
    ApiError(anyhow::Error),
    Database(anyhow::Error),
}

impl std::fmt::Display for ValorantLeaderboardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "No members have shared their profile in this guild"),
            Self::ApiForbidden => write!(f, "Riot API access forbidden"),
            Self::ApiError(err) => write!(f, "Failed to load leaderboard data: {err}"),
            Self::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}

impl std::error::Error for ValorantLeaderboardError {}

#[derive(Debug)]
pub enum ValorantVisibilityError {
    NotLinked,
    Database(anyhow::Error),
}

impl std::fmt::Display for ValorantVisibilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotLinked => write!(f, "No Riot account is linked to your Discord profile"),
            Self::Database(err) => write!(f, "Database error: {err}"),
        }
    }
}

impl std::error::Error for ValorantVisibilityError {}

impl From<anyhow::Error> for ValorantVisibilityError {
    fn from(err: anyhow::Error) -> Self {
        Self::Database(err)
    }
}

/// Service encapsulating VALORANT account linking, visibility consent, profile retrieval,
/// and guild leaderboard aggregation.
#[derive(Clone)]
pub struct ValorantService {
    pool: SqlitePool,
    riot_api: Arc<dyn RiotApiClient>,
}

impl ValorantService {
    pub fn new(pool: SqlitePool, riot_api: Arc<dyn RiotApiClient>) -> Self {
        Self { pool, riot_api }
    }

    /// Link a Riot account to a Discord user after validating formatting, region, and Riot API existence.
    pub async fn link_account(
        &self,
        user_id: UserId,
        riot_id: &str,
        region: Option<&str>,
    ) -> Result<LinkedRiotAccount, ValorantLinkError> {
        let trimmed = riot_id.trim();
        let Some((game_name, tag_line)) = trimmed.split_once('#') else {
            return Err(ValorantLinkError::InvalidFormat);
        };

        let game_name = game_name.trim();
        let tag_line = tag_line.trim();
        if game_name.is_empty() || tag_line.is_empty() {
            return Err(ValorantLinkError::InvalidFormat);
        }

        let riot_region = match region {
            Some(r) => RiotRegion::try_parse(r).ok_or(ValorantLinkError::InvalidRegion)?,
            None => RiotRegion::Ap,
        };

        let verified = match self
            .riot_api
            .get_account_by_riot_id(riot_region, game_name, tag_line)
            .await
        {
            Ok(acc) => acc,
            Err(err) => {
                if let Some(RiotApiError::NotFound) = err.downcast_ref::<RiotApiError>() {
                    let full_id = format!("{game_name}#{tag_line}");
                    return Err(ValorantLinkError::AccountNotFound(full_id));
                }

                tracing::warn!(
                    %err,
                    %game_name,
                    %tag_line,
                    "Failed to verify Riot account with Riot API"
                );
                return Err(ValorantLinkError::ApiError(err));
            }
        };

        let linked = set_linked_account(
            &self.pool,
            user_id,
            &verified.puuid,
            &verified.game_name,
            &verified.tag_line,
            riot_region,
        )
        .await
        .map_err(ValorantLinkError::Database)?;

        Ok(linked)
    }

    /// Unlink a user's global Riot account and revoke visibility across all guilds.
    pub async fn unlink_account(&self, user_id: UserId) -> Result<bool, anyhow::Error> {
        remove_linked_account(&self.pool, user_id).await
    }

    /// Retrieve a user's VALORANT profile and ranked stats, enforcing guild visibility permissions.
    pub async fn get_profile(
        &self,
        guild_id: GuildId,
        requester_id: UserId,
        target_id: UserId,
    ) -> Result<ValorantProfile, ValorantProfileError> {
        let is_self = requester_id == target_id;

        let account = match get_linked_account(&self.pool, target_id)
            .await
            .map_err(ValorantProfileError::Database)?
        {
            Some(acc) => acc,
            None => return Err(ValorantProfileError::NotLinked { is_self }),
        };

        let is_visible = get_guild_visibility(&self.pool, guild_id, target_id)
            .await
            .map_err(ValorantProfileError::Database)?;

        if !is_self && !is_visible {
            return Err(ValorantProfileError::HiddenOther);
        }

        let stats = match self
            .riot_api
            .get_player_ranked(account.region, &account.puuid)
            .await
        {
            Ok(s) => s,
            Err(err) => {
                tracing::warn!(
                    err = %err,
                    puuid = %account.puuid,
                    user_id = %target_id,
                    "Failed to load player ranked data"
                );
                match err.downcast_ref::<RiotApiError>() {
                    Some(RiotApiError::Forbidden) => return Err(ValorantProfileError::ApiForbidden),
                    Some(RiotApiError::NotFound) => return Err(ValorantProfileError::ApiUnranked),
                    _ => return Err(ValorantProfileError::ApiError(err)),
                }
            }
        };

        Ok(ValorantProfile {
            account,
            stats,
            is_self,
            is_visible,
        })
    }

    /// Retrieve and rank all visible members in a guild by tier descending, ranked rating descending, and wins descending.
    pub async fn get_guild_leaderboard(
        &self,
        guild_id: GuildId,
    ) -> Result<Vec<ValorantLeaderboardEntry>, ValorantLeaderboardError> {
        let visible_accounts = list_guild_visible_accounts(&self.pool, guild_id)
            .await
            .map_err(ValorantLeaderboardError::Database)?;

        if visible_accounts.is_empty() {
            return Err(ValorantLeaderboardError::Empty);
        }

        let mut join_set = tokio::task::JoinSet::new();
        for acc in visible_accounts {
            let api = Arc::clone(&self.riot_api);
            join_set.spawn(async move {
                let res = api.get_player_ranked(acc.region, &acc.puuid).await;
                (acc, res)
            });
        }

        let mut ranked_entries = Vec::new();
        let mut had_forbidden = false;
        while let Some(res) = join_set.join_next().await {
            if let Ok((acc, res)) = res {
                match res {
                    Ok(stats) => {
                        ranked_entries.push(ValorantLeaderboardEntry { account: acc, stats })
                    }
                    Err(err) => {
                        if let Some(RiotApiError::Forbidden) = err.downcast_ref::<RiotApiError>() {
                            had_forbidden = true;
                        }
                        tracing::warn!(
                            %err,
                            user_id = %acc.user_id,
                            "Failed to load player ranked data for leaderboard"
                        );
                    }
                }
            }
        }

        if ranked_entries.is_empty() {
            if had_forbidden {
                return Err(ValorantLeaderboardError::ApiForbidden);
            } else {
                return Err(ValorantLeaderboardError::ApiError(anyhow::anyhow!(
                    "No leaderboard entries could be loaded"
                )));
            }
        }

        ranked_entries.sort_by(|a, b| {
            b.stats
                .tier
                .cmp(&a.stats.tier)
                .then_with(|| b.stats.ranked_rating.cmp(&a.stats.ranked_rating))
                .then_with(|| b.stats.number_of_wins.cmp(&a.stats.number_of_wins))
        });

        Ok(ranked_entries)
    }

    /// Enable Guild Profile Visibility for a user. Fails with `NotLinked` if the user has no linked account.
    pub async fn enable_visibility(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<(), ValorantVisibilityError> {
        let account = get_linked_account(&self.pool, user_id).await?;
        if account.is_none() {
            return Err(ValorantVisibilityError::NotLinked);
        }

        set_guild_visibility(&self.pool, guild_id, user_id, true).await?;
        Ok(())
    }

    /// Disable Guild Profile Visibility for a user.
    pub async fn disable_visibility(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<(), anyhow::Error> {
        set_guild_visibility(&self.pool, guild_id, user_id, false).await
    }

    /// Check if a user has enabled Guild Profile Visibility in a guild.
    pub async fn get_visibility_status(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<bool, anyhow::Error> {
        get_guild_visibility(&self.pool, guild_id, user_id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::valorant::riot_api::{BoxFuture, CompetitiveTier, LeaderboardResponse, PlatformStatus, RiotAccount};
    use std::collections::HashMap;
    use std::sync::Mutex;

    async fn setup_test_pool() -> (SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-valorant-service-test-{}-{}",
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

    struct MockRiotClient {
        accounts: Mutex<HashMap<(String, String), RiotAccount>>,
        ranked_data: Mutex<HashMap<String, Result<PlayerRankedData, RiotApiError>>>,
    }

    impl MockRiotClient {
        fn new() -> Self {
            Self {
                accounts: Mutex::new(HashMap::new()),
                ranked_data: Mutex::new(HashMap::new()),
            }
        }

        fn add_account(&self, game_name: &str, tag_line: &str, puuid: &str) {
            self.accounts.lock().unwrap().insert(
                (game_name.to_lowercase(), tag_line.to_lowercase()),
                RiotAccount {
                    puuid: puuid.to_string(),
                    game_name: game_name.to_string(),
                    tag_line: tag_line.to_string(),
                },
            );
        }

        fn set_ranked(&self, puuid: &str, stats: Result<PlayerRankedData, RiotApiError>) {
            self.ranked_data
                .lock()
                .unwrap()
                .insert(puuid.to_string(), stats);
        }
    }

    impl RiotApiClient for MockRiotClient {
        fn get_platform_status<'a>(
            &'a self,
            _region: RiotRegion,
        ) -> BoxFuture<'a, Result<PlatformStatus>> {
            Box::pin(async move {
                Ok(PlatformStatus {
                    id: "VALORANT".to_string(),
                    name: "VALORANT".to_string(),
                    locales: vec![],
                    maintenances: vec![],
                    incidents: vec![],
                })
            })
        }

        fn get_leaderboard<'a>(
            &'a self,
            _region: RiotRegion,
            _act_id: &'a str,
            _size: u32,
            _start_index: u32,
        ) -> BoxFuture<'a, Result<LeaderboardResponse>> {
            Box::pin(async move {
                Ok(LeaderboardResponse {
                    act_id: "act".to_string(),
                    total_players: 0,
                    players: vec![],
                })
            })
        }

        fn get_player_ranked<'a>(
            &'a self,
            _region: RiotRegion,
            puuid: &'a str,
        ) -> BoxFuture<'a, Result<PlayerRankedData>> {
            Box::pin(async move {
                let map = self.ranked_data.lock().unwrap();
                match map.get(puuid) {
                    Some(Ok(data)) => Ok(data.clone()),
                    Some(Err(RiotApiError::NotFound)) => Err(anyhow::Error::new(RiotApiError::NotFound)),
                    Some(Err(RiotApiError::Forbidden)) => Err(anyhow::Error::new(RiotApiError::Forbidden)),
                    Some(Err(e)) => Err(anyhow::anyhow!("{e}")),
                    None => Ok(super::super::riot_api::MockRiotApiClient::generate_mock_player_ranked(
                        puuid, None, None,
                    )),
                }
            })
        }

        fn get_account_by_riot_id<'a>(
            &'a self,
            _region: RiotRegion,
            game_name: &'a str,
            tag_line: &'a str,
        ) -> BoxFuture<'a, Result<RiotAccount>> {
            Box::pin(async move {
                let map = self.accounts.lock().unwrap();
                match map.get(&(game_name.to_lowercase(), tag_line.to_lowercase())) {
                    Some(acc) => Ok(acc.clone()),
                    None => Err(anyhow::Error::new(RiotApiError::NotFound)),
                }
            })
        }
    }

    #[tokio::test]
    async fn link_account_format_and_region_validation() {
        let (pool, _dir) = setup_test_pool().await;
        let mock = Arc::new(MockRiotClient::new());
        let service = ValorantService::new(pool, mock.clone());
        let user = UserId::new(100);

        // Missing hash separator
        let err = service.link_account(user, "InvalidID", None).await.unwrap_err();
        assert!(matches!(err, ValorantLinkError::InvalidFormat));

        // Empty game name
        let err = service.link_account(user, "#TAG", None).await.unwrap_err();
        assert!(matches!(err, ValorantLinkError::InvalidFormat));

        // Empty tag line
        let err = service.link_account(user, "Name#", None).await.unwrap_err();
        assert!(matches!(err, ValorantLinkError::InvalidFormat));

        // Invalid region
        let err = service
            .link_account(user, "Name#TAG", Some("invalid_region"))
            .await
            .unwrap_err();
        assert!(matches!(err, ValorantLinkError::InvalidRegion));

        // Account not found in Riot API
        let err = service
            .link_account(user, "TenZ#0001", Some("na"))
            .await
            .unwrap_err();
        match err {
            ValorantLinkError::AccountNotFound(id) => assert_eq!(id, "TenZ#0001"),
            other => panic!("Expected AccountNotFound, got {:?}", other),
        }

        // Successful link
        mock.add_account("TenZ", "0001", "tenz-puuid");
        let linked = service
            .link_account(user, "TenZ#0001", Some("na"))
            .await
            .unwrap();
        assert_eq!(linked.game_name, "TenZ");
        assert_eq!(linked.tag_line, "0001");
        assert_eq!(linked.region, RiotRegion::Na);
        assert_eq!(linked.puuid, "tenz-puuid");

        // Unlink
        let unlinked = service.unlink_account(user).await.unwrap();
        assert!(unlinked);
        let unlinked_again = service.unlink_account(user).await.unwrap();
        assert!(!unlinked_again);
    }

    #[tokio::test]
    async fn profile_self_vs_other_visibility_privacy_enforcement() {
        let (pool, _dir) = setup_test_pool().await;
        let mock = Arc::new(MockRiotClient::new());
        let service = ValorantService::new(pool, mock.clone());

        let guild = GuildId::new(500);
        let requester = UserId::new(101);
        let target = UserId::new(102);

        // Neither linked
        let err = service.get_profile(guild, requester, requester).await.unwrap_err();
        assert!(matches!(err, ValorantProfileError::NotLinked { is_self: true }));

        let err = service.get_profile(guild, requester, target).await.unwrap_err();
        assert!(matches!(err, ValorantProfileError::NotLinked { is_self: false }));

        // Link target
        mock.add_account("Jett", "0001", "jett-puuid");
        service
            .link_account(target, "Jett#0001", Some("ap"))
            .await
            .unwrap();

        // Target has visibility disabled by default: requester sees HiddenOther
        let err = service.get_profile(guild, requester, target).await.unwrap_err();
        assert!(matches!(err, ValorantProfileError::HiddenOther));

        // Target viewing self sees profile even if visibility is false
        let self_prof = service.get_profile(guild, target, target).await.unwrap();
        assert!(self_prof.is_self);
        assert!(!self_prof.is_visible);
        assert_eq!(self_prof.account.game_name, "Jett");

        // Target enables visibility
        service.enable_visibility(guild, target).await.unwrap();
        assert!(service.get_visibility_status(guild, target).await.unwrap());

        // Now requester can view target's profile
        let other_prof = service.get_profile(guild, requester, target).await.unwrap();
        assert!(!other_prof.is_self);
        assert!(other_prof.is_visible);
        assert_eq!(other_prof.account.game_name, "Jett");

        // Target disables visibility
        service.disable_visibility(guild, target).await.unwrap();
        assert!(!service.get_visibility_status(guild, target).await.unwrap());

        let err = service.get_profile(guild, requester, target).await.unwrap_err();
        assert!(matches!(err, ValorantProfileError::HiddenOther));
    }

    #[tokio::test]
    async fn leaderboard_sorting_tier_rr_wins() {
        let (pool, _dir) = setup_test_pool().await;
        let mock = Arc::new(MockRiotClient::new());
        let service = ValorantService::new(pool, mock.clone());
        let guild = GuildId::new(700);

        // Create 4 players with explicit stats:
        // p1: Ascendant1, 50 RR, 10 wins
        // p2: Diamond3, 90 RR, 100 wins
        // p3: Diamond3, 90 RR, 20 wins
        // p4: Diamond3, 50 RR, 200 wins
        let players = [
            (UserId::new(1), "P1", "1", "puuid-1", CompetitiveTier::Ascendant1, 50, 10),
            (UserId::new(2), "P2", "2", "puuid-2", CompetitiveTier::Diamond3, 90, 100),
            (UserId::new(3), "P3", "3", "puuid-3", CompetitiveTier::Diamond3, 90, 20),
            (UserId::new(4), "P4", "4", "puuid-4", CompetitiveTier::Diamond3, 50, 200),
        ];

        for (u, name, tag, puuid, tier, rr, wins) in players {
            mock.add_account(name, tag, puuid);
            service
                .link_account(u, &format!("{name}#{tag}"), Some("ap"))
                .await
                .unwrap();
            service.enable_visibility(guild, u).await.unwrap();
            mock.set_ranked(
                puuid,
                Ok(PlayerRankedData {
                    puuid: puuid.to_string(),
                    game_name: name.to_string(),
                    tag_line: tag.to_string(),
                    tier,
                    ranked_rating: rr,
                    number_of_wins: wins,
                }),
            );
        }

        let lb = service.get_guild_leaderboard(guild).await.unwrap();
        assert_eq!(lb.len(), 4);
        // Expected order:
        // 1st: P1 (Ascendant1)
        // 2nd: P2 (Diamond3, 90 RR, 100 wins)
        // 3rd: P3 (Diamond3, 90 RR, 20 wins)
        // 4th: P4 (Diamond3, 50 RR, 200 wins)
        assert_eq!(lb[0].account.game_name, "P1");
        assert_eq!(lb[1].account.game_name, "P2");
        assert_eq!(lb[2].account.game_name, "P3");
        assert_eq!(lb[3].account.game_name, "P4");
    }

    #[tokio::test]
    async fn leaderboard_empty_and_error_handling() {
        let (pool, _dir) = setup_test_pool().await;
        let mock = Arc::new(MockRiotClient::new());
        let service = ValorantService::new(pool, mock.clone());
        let guild = GuildId::new(800);

        // Empty leaderboard
        let err = service.get_guild_leaderboard(guild).await.unwrap_err();
        assert!(matches!(err, ValorantLeaderboardError::Empty));

        // Add player with forbidden error
        let user = UserId::new(10);
        mock.add_account("Secret", "0001", "secret-puuid");
        service
            .link_account(user, "Secret#0001", Some("ap"))
            .await
            .unwrap();
        service.enable_visibility(guild, user).await.unwrap();
        mock.set_ranked("secret-puuid", Err(RiotApiError::Forbidden));

        let err = service.get_guild_leaderboard(guild).await.unwrap_err();
        assert!(matches!(err, ValorantLeaderboardError::ApiForbidden));
    }

    #[tokio::test]
    async fn visibility_enabling_without_linked_account_fails() {
        let (pool, _dir) = setup_test_pool().await;
        let mock = Arc::new(MockRiotClient::new());
        let service = ValorantService::new(pool, mock);
        let guild = GuildId::new(900);
        let user = UserId::new(999);

        let err = service.enable_visibility(guild, user).await.unwrap_err();
        assert!(matches!(err, ValorantVisibilityError::NotLinked));
    }
}
