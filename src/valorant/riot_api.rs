use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Supported Riot Games VALORANT API routing regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, poise::ChoiceParameter)]
pub enum RiotRegion {
    #[name = "ap (Asia-Pacific)"]
    Ap,
    #[name = "br (Brazil)"]
    Br,
    #[name = "eu (Europe)"]
    Eu,
    #[name = "kr (Korea)"]
    Kr,
    #[name = "latam (Latin America)"]
    Latam,
    #[name = "na (North America)"]
    Na,
}

impl RiotRegion {
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ap" | "asia" => Some(Self::Ap),
            "br" | "brazil" => Some(Self::Br),
            "eu" | "europe" | "eun" | "euw" => Some(Self::Eu),
            "kr" | "korea" => Some(Self::Kr),
            "latam" | "la1" => Some(Self::Latam),
            "na" | "north_america" => Some(Self::Na),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ap => "ap",
            Self::Br => "br",
            Self::Eu => "eu",
            Self::Kr => "kr",
            Self::Latam => "latam",
            Self::Na => "na",
        }
    }

    pub fn api_endpoint(&self) -> &'static str {
        match self {
            Self::Ap => "https://ap.api.riotgames.com",
            Self::Br => "https://br.api.riotgames.com",
            Self::Eu => "https://eu.api.riotgames.com",
            Self::Kr => "https://kr.api.riotgames.com",
            Self::Latam => "https://latam.api.riotgames.com",
            Self::Na => "https://na.api.riotgames.com",
        }
    }

    pub fn account_cluster_endpoint(&self) -> &'static str {
        match self {
            Self::Ap | Self::Kr => "https://asia.api.riotgames.com",
            Self::Br | Self::Latam | Self::Na => "https://americas.api.riotgames.com",
            Self::Eu => "https://europe.api.riotgames.com",
        }
    }
}

/// Official Riot Account details from Account-V1.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RiotAccount {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocalizedContent {
    pub locale: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusIncident {
    pub id: i64,
    pub maintenance_status: Option<String>,
    pub incident_severity: Option<String>,
    pub titles: Vec<LocalizedContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStatus {
    pub id: String,
    pub name: String,
    pub locales: Vec<String>,
    pub maintenances: Vec<StatusIncident>,
    pub incidents: Vec<StatusIncident>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardPlayer {
    pub leaderboard_rank: u64,
    pub ranked_rating: u64,
    pub number_of_wins: u64,
    pub game_name: Option<String>,
    pub tag_line: Option<String>,
    pub puuid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LeaderboardResponse {
    pub act_id: String,
    pub total_players: u64,
    pub players: Vec<LeaderboardPlayer>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[repr(u8)]
pub enum CompetitiveTier {
    Unranked = 0,
    Iron1 = 1,
    Iron2 = 2,
    Iron3 = 3,
    Bronze1 = 4,
    Bronze2 = 5,
    Bronze3 = 6,
    Silver1 = 7,
    Silver2 = 8,
    Silver3 = 9,
    Gold1 = 10,
    Gold2 = 11,
    Gold3 = 12,
    Platinum1 = 13,
    Platinum2 = 14,
    Platinum3 = 15,
    Diamond1 = 16,
    Diamond2 = 17,
    Diamond3 = 18,
    Ascendant1 = 19,
    Ascendant2 = 20,
    Ascendant3 = 21,
    Immortal1 = 22,
    Immortal2 = 23,
    Immortal3 = 24,
    Radiant = 25,
}

impl CompetitiveTier {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Unranked => "Unranked",
            Self::Iron1 => "Iron 1",
            Self::Iron2 => "Iron 2",
            Self::Iron3 => "Iron 3",
            Self::Bronze1 => "Bronze 1",
            Self::Bronze2 => "Bronze 2",
            Self::Bronze3 => "Bronze 3",
            Self::Silver1 => "Silver 1",
            Self::Silver2 => "Silver 2",
            Self::Silver3 => "Silver 3",
            Self::Gold1 => "Gold 1",
            Self::Gold2 => "Gold 2",
            Self::Gold3 => "Gold 3",
            Self::Platinum1 => "Platinum 1",
            Self::Platinum2 => "Platinum 2",
            Self::Platinum3 => "Platinum 3",
            Self::Diamond1 => "Diamond 1",
            Self::Diamond2 => "Diamond 2",
            Self::Diamond3 => "Diamond 3",
            Self::Ascendant1 => "Ascendant 1",
            Self::Ascendant2 => "Ascendant 2",
            Self::Ascendant3 => "Ascendant 3",
            Self::Immortal1 => "Immortal 1",
            Self::Immortal2 => "Immortal 2",
            Self::Immortal3 => "Immortal 3",
            Self::Radiant => "Radiant",
        }
    }
}

impl std::fmt::Display for CompetitiveTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerRankedData {
    pub puuid: String,
    pub game_name: String,
    pub tag_line: String,
    pub tier: CompetitiveTier,
    pub ranked_rating: u32,
    pub number_of_wins: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RecentMatchSummary {
    pub match_id: String,
    pub map_name: String,
    pub game_mode: String,
    pub game_start_millis: i64,
    pub character: String,
    pub rounds_won: u32,
    pub rounds_lost: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub won: bool,
}

/// Seam for official Riot Games Developer API interactions.
///
/// Note (ADR-0004): Automated VALORANT player data is strictly handled through Riot's
/// approved APIs (such as VAL-STATUS-V1 and VAL-RANKED-V1), separate from unverified user
/// navigation Tracker Profile Links, ensuring full compliance with Riot Developer Terms.
pub trait RiotApiClient: Send + Sync {
    fn get_platform_status<'a>(
        &'a self,
        region: RiotRegion,
    ) -> BoxFuture<'a, Result<PlatformStatus>>;

    fn get_leaderboard<'a>(
        &'a self,
        region: RiotRegion,
        act_id: &'a str,
        size: u32,
        start_index: u32,
    ) -> BoxFuture<'a, Result<LeaderboardResponse>>;

    fn get_player_ranked<'a>(
        &'a self,
        region: RiotRegion,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<PlayerRankedData>>;

    fn get_account_by_riot_id<'a>(
        &'a self,
        region: RiotRegion,
        game_name: &'a str,
        tag_line: &'a str,
    ) -> BoxFuture<'a, Result<RiotAccount>>;

    fn get_recent_matches<'a>(
        &'a self,
        region: RiotRegion,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<RecentMatchSummary>>>;
}

/// Structured domain errors for Riot Games API interactions.
#[derive(Debug)]
pub enum RiotApiError {
    NotFound,
    Forbidden,
    Unauthorized,
    Api {
        status: reqwest::StatusCode,
        message: String,
    },
    Transport(reqwest::Error),
}

impl std::fmt::Display for RiotApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Riot account or data not found (404)"),
            Self::Forbidden => write!(f, "Riot API access forbidden (403)"),
            Self::Unauthorized => write!(f, "Riot API unauthorized or key expired (401)"),
            Self::Api { status, message } => write!(f, "Riot API error {status}: {message}"),
            Self::Transport(err) => write!(f, "Network error communicating with Riot API: {err}"),
        }
    }
}

impl std::error::Error for RiotApiError {}

/// Production HTTP Riot API client using reqwest with X-Riot-Token.
pub struct HttpRiotApiClient {
    api_key: String,
    client: reqwest::Client,
}

impl HttpRiotApiClient {
    pub fn new(api_key: String, client: reqwest::Client) -> Self {
        Self { api_key, client }
    }

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T> {
        let resp = self
            .client
            .get(url)
            .header("X-Riot-Token", &self.api_key)
            .header("User-Agent", "Dummy-Bot/3.4 (DiscordBot)")
            .send()
            .await
            .map_err(RiotApiError::Transport)
            .with_context(|| format!("Failed to send GET request to {url}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let message = resp.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(RiotApiError::NotFound.into());
            }
            if status == reqwest::StatusCode::FORBIDDEN {
                return Err(RiotApiError::Forbidden.into());
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(RiotApiError::Unauthorized.into());
            }
            return Err(RiotApiError::Api { status, message }.into());
        }

        let text = resp
            .text()
            .await
            .with_context(|| format!("Failed to read response body from {url}"))?;

        let data = serde_json::from_str::<T>(&text)
            .with_context(|| format!("Failed to decode JSON response from {url}"))?;

        Ok(data)
    }
}

impl RiotApiClient for HttpRiotApiClient {
    fn get_platform_status<'a>(
        &'a self,
        region: RiotRegion,
    ) -> BoxFuture<'a, Result<PlatformStatus>> {
        Box::pin(async move {
            let base = region.api_endpoint();
            let url = format!("{base}/val/status/v1/platform-data");
            self.get_json(&url).await
        })
    }

    fn get_leaderboard<'a>(
        &'a self,
        region: RiotRegion,
        act_id: &'a str,
        size: u32,
        start_index: u32,
    ) -> BoxFuture<'a, Result<LeaderboardResponse>> {
        Box::pin(async move {
            let base = region.api_endpoint();
            let url = format!(
                "{base}/val/ranked/v1/leaderboards/by-act/{act_id}?size={size}&startIndex={start_index}"
            );
            self.get_json(&url).await
        })
    }

    fn get_player_ranked<'a>(
        &'a self,
        region: RiotRegion,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<PlayerRankedData>> {
        Box::pin(async move {
            let base = region.api_endpoint();
            let url = format!("{base}/val/ranked/v1/players/{puuid}");
            self.get_json(&url).await
        })
    }

    fn get_account_by_riot_id<'a>(
        &'a self,
        region: RiotRegion,
        game_name: &'a str,
        tag_line: &'a str,
    ) -> BoxFuture<'a, Result<RiotAccount>> {
        Box::pin(async move {
            let base = region.account_cluster_endpoint();
            let mut url = reqwest::Url::parse(base)
                .with_context(|| format!("Invalid account cluster endpoint URL: {base}"))?;
            url.path_segments_mut()
                .map_err(|_| anyhow::anyhow!("Cannot format path segments on {base}"))?
                .extend(&[
                    "riot",
                    "account",
                    "v1",
                    "accounts",
                    "by-riot-id",
                    game_name,
                    tag_line,
                ]);
            self.get_json(url.as_str()).await
        })
    }

    fn get_recent_matches<'a>(
        &'a self,
        region: RiotRegion,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<RecentMatchSummary>>> {
        Box::pin(async move {
            let base = region.api_endpoint();
            let url = format!("{base}/val/match/v1/matchlists/by-puuid/{puuid}");
            #[derive(Deserialize)]
            #[serde(untagged)]
            enum MatchListPayload {
                Direct(Vec<RecentMatchSummary>),
                Wrapped {
                    #[serde(alias = "matches", alias = "history")]
                    history: Vec<RecentMatchSummary>,
                },
            }

            let payload: MatchListPayload = self.get_json(&url).await?;
            let mut matches = match payload {
                MatchListPayload::Direct(m) => m,
                MatchListPayload::Wrapped { history } => history,
            };
            matches.truncate(count);
            Ok(matches)
        })
    }
}

/// Offline mock Riot API client for deterministic tests and deployments without an active API key.
#[derive(Default)]
pub struct MockRiotApiClient;

impl MockRiotApiClient {
    pub fn generate_mock_player_ranked(
        puuid: &str,
        game_name: Option<&str>,
        tag_line: Option<&str>,
    ) -> PlayerRankedData {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(puuid, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);

        let tiers = [
            CompetitiveTier::Iron1,
            CompetitiveTier::Iron2,
            CompetitiveTier::Iron3,
            CompetitiveTier::Bronze1,
            CompetitiveTier::Bronze2,
            CompetitiveTier::Bronze3,
            CompetitiveTier::Silver1,
            CompetitiveTier::Silver2,
            CompetitiveTier::Silver3,
            CompetitiveTier::Gold1,
            CompetitiveTier::Gold2,
            CompetitiveTier::Gold3,
            CompetitiveTier::Platinum1,
            CompetitiveTier::Platinum2,
            CompetitiveTier::Platinum3,
            CompetitiveTier::Diamond1,
            CompetitiveTier::Diamond2,
            CompetitiveTier::Diamond3,
            CompetitiveTier::Ascendant1,
            CompetitiveTier::Ascendant2,
            CompetitiveTier::Ascendant3,
            CompetitiveTier::Immortal1,
            CompetitiveTier::Immortal2,
            CompetitiveTier::Immortal3,
            CompetitiveTier::Radiant,
        ];
        let tier_idx = (hash % (tiers.len() as u64)) as usize;
        let tier = tiers[tier_idx];
        let ranked_rating = (hash % 100) as u32;
        let number_of_wins = ((hash % 120) + 5) as u32;

        PlayerRankedData {
            puuid: puuid.to_string(),
            game_name: game_name
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("Agent{}", &puuid[..puuid.len().min(4)])),
            tag_line: tag_line
                .map(|s| s.to_string())
                .unwrap_or_else(|| "VAL".to_string()),
            tier,
            ranked_rating,
            number_of_wins,
        }
    }

    pub fn generate_mock_recent_matches(puuid: &str, count: usize) -> Vec<RecentMatchSummary> {
        let base_matches = vec![
            RecentMatchSummary {
                match_id: format!("mock-match-1-{puuid}"),
                map_name: "Ascent".to_string(),
                game_mode: "Competitive".to_string(),
                game_start_millis: 1718000000000,
                character: "Jett".to_string(),
                rounds_won: 13,
                rounds_lost: 9,
                kills: 22,
                deaths: 14,
                assists: 5,
                won: true,
            },
            RecentMatchSummary {
                match_id: format!("mock-match-2-{puuid}"),
                map_name: "Haven".to_string(),
                game_mode: "Competitive".to_string(),
                game_start_millis: 1717990000000,
                character: "Omen".to_string(),
                rounds_won: 10,
                rounds_lost: 13,
                kills: 15,
                deaths: 17,
                assists: 9,
                won: false,
            },
            RecentMatchSummary {
                match_id: format!("mock-match-3-{puuid}"),
                map_name: "Lotus".to_string(),
                game_mode: "Competitive".to_string(),
                game_start_millis: 1717980000000,
                character: "Killjoy".to_string(),
                rounds_won: 13,
                rounds_lost: 11,
                kills: 18,
                deaths: 12,
                assists: 6,
                won: true,
            },
            RecentMatchSummary {
                match_id: format!("mock-match-4-{puuid}"),
                map_name: "Sunset".to_string(),
                game_mode: "Competitive".to_string(),
                game_start_millis: 1717970000000,
                character: "Reyna".to_string(),
                rounds_won: 8,
                rounds_lost: 13,
                kills: 21,
                deaths: 16,
                assists: 3,
                won: false,
            },
            RecentMatchSummary {
                match_id: format!("mock-match-5-{puuid}"),
                map_name: "Bind".to_string(),
                game_mode: "Competitive".to_string(),
                game_start_millis: 1717960000000,
                character: "Viper".to_string(),
                rounds_won: 13,
                rounds_lost: 7,
                kills: 17,
                deaths: 11,
                assists: 8,
                won: true,
            },
        ];

        base_matches.into_iter().take(count).collect()
    }
}

impl RiotApiClient for MockRiotApiClient {
    fn get_platform_status<'a>(
        &'a self,
        region: RiotRegion,
    ) -> BoxFuture<'a, Result<PlatformStatus>> {
        Box::pin(async move {
            Ok(PlatformStatus {
                id: "VALORANT".to_string(),
                name: format!("VALORANT ({})", region.as_str().to_ascii_uppercase()),
                locales: vec!["en_US".to_string()],
                maintenances: vec![],
                incidents: vec![],
            })
        })
    }

    fn get_leaderboard<'a>(
        &'a self,
        _region: RiotRegion,
        act_id: &'a str,
        size: u32,
        _start_index: u32,
    ) -> BoxFuture<'a, Result<LeaderboardResponse>> {
        Box::pin(async move {
            let mut players = Vec::new();
            let limit = size.min(10);
            for i in 1..=limit {
                players.push(LeaderboardPlayer {
                    leaderboard_rank: i as u64,
                    ranked_rating: 1000 - (i as u64 * 10),
                    number_of_wins: 150 - (i as u64 * 2),
                    game_name: Some(format!("ProPlayer{i}")),
                    tag_line: Some("PRO".to_string()),
                    puuid: Some(format!("mock-puuid-{i}")),
                });
            }

            Ok(LeaderboardResponse {
                act_id: act_id.to_string(),
                total_players: 500,
                players,
            })
        })
    }

    fn get_player_ranked<'a>(
        &'a self,
        _region: RiotRegion,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<PlayerRankedData>> {
        Box::pin(async move { Ok(Self::generate_mock_player_ranked(puuid, None, None)) })
    }

    fn get_account_by_riot_id<'a>(
        &'a self,
        _region: RiotRegion,
        game_name: &'a str,
        tag_line: &'a str,
    ) -> BoxFuture<'a, Result<RiotAccount>> {
        Box::pin(async move {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(game_name, &mut hasher);
            std::hash::Hash::hash(tag_line, &mut hasher);
            let hash = std::hash::Hasher::finish(&hasher);

            Ok(RiotAccount {
                puuid: format!("mock-puuid-{:x}", hash),
                game_name: game_name.to_string(),
                tag_line: tag_line.to_string(),
            })
        })
    }

    fn get_recent_matches<'a>(
        &'a self,
        _region: RiotRegion,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<RecentMatchSummary>>> {
        Box::pin(async move { Ok(Self::generate_mock_recent_matches(puuid, count)) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_riot_api_client_returns_recent_matches() {
        let client = MockRiotApiClient;
        let matches = client
            .get_recent_matches(RiotRegion::Ap, "puuid-test-1", 3)
            .await
            .unwrap();

        assert_eq!(matches.len(), 3);
        // Deterministic mock match 1: Ascent Jett 13-9 Win
        assert_eq!(matches[0].map_name, "Ascent");
        assert_eq!(matches[0].character, "Jett");
        assert_eq!(matches[0].rounds_won, 13);
        assert_eq!(matches[0].rounds_lost, 9);
        assert!(matches[0].won);
        assert_eq!(matches[0].kills, 22);

        // Deterministic mock match 2: Haven Omen 10-13 Loss
        assert_eq!(matches[1].map_name, "Haven");
        assert_eq!(matches[1].character, "Omen");
        assert_eq!(matches[1].rounds_won, 10);
        assert_eq!(matches[1].rounds_lost, 13);
        assert!(!matches[1].won);

        // Deterministic mock match 3: Lotus Killjoy 13-11 Win
        assert_eq!(matches[2].map_name, "Lotus");
        assert_eq!(matches[2].character, "Killjoy");
        assert_eq!(matches[2].rounds_won, 13);
        assert_eq!(matches[2].rounds_lost, 11);
        assert!(matches[2].won);

        // Test count limit
        let matches_1 = client
            .get_recent_matches(RiotRegion::Ap, "puuid-test-1", 1)
            .await
            .unwrap();
        assert_eq!(matches_1.len(), 1);
        assert_eq!(matches_1[0].map_name, "Ascent");
    }

    #[test]
    fn deserializes_recent_match_summary_json() {
        let json = r#"{
            "matchId": "match-abc-123",
            "mapName": "Ascent",
            "gameMode": "Competitive",
            "gameStartMillis": 1718000000000,
            "character": "Jett",
            "roundsWon": 13,
            "roundsLost": 9,
            "kills": 22,
            "deaths": 14,
            "assists": 5,
            "won": true
        }"#;
        let match_summary: RecentMatchSummary = serde_json::from_str(json).unwrap();
        assert_eq!(match_summary.match_id, "match-abc-123");
        assert_eq!(match_summary.map_name, "Ascent");
        assert_eq!(match_summary.character, "Jett");
        assert_eq!(match_summary.rounds_won, 13);
        assert_eq!(match_summary.rounds_lost, 9);
        assert!(match_summary.won);
    }

    #[tokio::test]
    async fn mock_riot_api_client_returns_status_and_leaderboard() {
        let client = MockRiotApiClient;

        let status = client.get_platform_status(RiotRegion::Ap).await.unwrap();
        assert_eq!(status.id, "VALORANT");
        assert_eq!(status.name, "VALORANT (AP)");
        assert!(status.maintenances.is_empty());

        let leaderboard = client
            .get_leaderboard(RiotRegion::Ap, "act-12345", 5, 0)
            .await
            .unwrap();
        assert_eq!(leaderboard.act_id, "act-12345");
        assert_eq!(leaderboard.players.len(), 5);
        assert_eq!(leaderboard.players[0].leaderboard_rank, 1);
        assert_eq!(
            leaderboard.players[0].game_name.as_deref(),
            Some("ProPlayer1")
        );
    }

    #[tokio::test]
    async fn mock_riot_api_client_returns_player_ranked_data() {
        let client = MockRiotApiClient;
        let data = client
            .get_player_ranked(RiotRegion::Ap, "puuid-test-1")
            .await
            .unwrap();
        assert_eq!(data.puuid, "puuid-test-1");
        assert_ne!(data.tier, CompetitiveTier::Unranked);
        assert!(!data.tier.name().is_empty());
        assert!(data.ranked_rating <= 100);
        assert!(data.number_of_wins > 0);
    }

    #[test]
    fn region_endpoint_mapping_covers_all_regions() {
        assert_eq!(
            RiotRegion::Ap.api_endpoint(),
            "https://ap.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Na.api_endpoint(),
            "https://na.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Eu.api_endpoint(),
            "https://eu.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Kr.api_endpoint(),
            "https://kr.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Latam.api_endpoint(),
            "https://latam.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Br.api_endpoint(),
            "https://br.api.riotgames.com"
        );
    }

    #[test]
    fn parses_region_aliases_and_rejects_unknown() {
        assert_eq!(RiotRegion::try_parse("ap"), Some(RiotRegion::Ap));
        assert_eq!(RiotRegion::try_parse("asia"), Some(RiotRegion::Ap));
        assert_eq!(RiotRegion::try_parse("NA"), Some(RiotRegion::Na));
        assert_eq!(RiotRegion::try_parse("europe"), Some(RiotRegion::Eu));
        assert_eq!(RiotRegion::try_parse("euw"), Some(RiotRegion::Eu));
        assert_eq!(RiotRegion::try_parse("invalid"), None);
    }

    #[test]
    fn deserializes_camel_case_riot_json() {
        let json = r#"{
            "actId": "act-123",
            "totalPlayers": 1,
            "players": [{
                "leaderboardRank": 1,
                "rankedRating": 550,
                "numberOfWins": 42,
                "gameName": "Viper",
                "tagLine": "001",
                "puuid": "uuid-1"
            }]
        }"#;
        let resp: LeaderboardResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.act_id, "act-123");
        assert_eq!(resp.total_players, 1);
        assert_eq!(resp.players[0].leaderboard_rank, 1);
        assert_eq!(resp.players[0].ranked_rating, 550);
        assert_eq!(resp.players[0].number_of_wins, 42);
        assert_eq!(resp.players[0].game_name.as_deref(), Some("Viper"));
        assert_eq!(resp.players[0].tag_line.as_deref(), Some("001"));
    }

    #[test]
    fn account_cluster_endpoint_mapping_covers_clusters() {
        assert_eq!(
            RiotRegion::Ap.account_cluster_endpoint(),
            "https://asia.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Kr.account_cluster_endpoint(),
            "https://asia.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Na.account_cluster_endpoint(),
            "https://americas.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Br.account_cluster_endpoint(),
            "https://americas.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Latam.account_cluster_endpoint(),
            "https://americas.api.riotgames.com"
        );
        assert_eq!(
            RiotRegion::Eu.account_cluster_endpoint(),
            "https://europe.api.riotgames.com"
        );
    }

    #[test]
    fn deserializes_riot_account_json() {
        let json = r#"{
            "puuid": "test-puuid-123",
            "gameName": "Blask",
            "tagLine": "3107"
        }"#;
        let acc: RiotAccount = serde_json::from_str(json).unwrap();
        assert_eq!(acc.puuid, "test-puuid-123");
        assert_eq!(acc.game_name, "Blask");
        assert_eq!(acc.tag_line, "3107");
    }

    #[tokio::test]
    async fn mock_riot_api_client_returns_account_by_riot_id() {
        let client = MockRiotApiClient;
        let acc = client
            .get_account_by_riot_id(RiotRegion::Ap, "Blask", "3107")
            .await
            .unwrap();
        assert_eq!(acc.game_name, "Blask");
        assert_eq!(acc.tag_line, "3107");
        assert!(!acc.puuid.is_empty());
    }

    #[test]
    fn riot_api_error_downcasting_and_display() {
        let err: anyhow::Error = RiotApiError::Forbidden.into();
        assert!(matches!(
            err.downcast_ref::<RiotApiError>(),
            Some(RiotApiError::Forbidden)
        ));
        assert_eq!(
            err.downcast_ref::<RiotApiError>().unwrap().to_string(),
            "Riot API access forbidden (403)"
        );

        let err: anyhow::Error = RiotApiError::NotFound.into();
        assert!(matches!(
            err.downcast_ref::<RiotApiError>(),
            Some(RiotApiError::NotFound)
        ));
    }
}
