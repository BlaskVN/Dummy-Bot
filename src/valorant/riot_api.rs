use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Supported Riot Games VALORANT API routing regions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RiotRegion {
    Ap,
    Br,
    Eu,
    Kr,
    Latam,
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
}

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
            .send()
            .await
            .with_context(|| format!("Failed to send GET request to {url}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            bail!("Riot API returned error status {status}: {text}");
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
}

/// Offline mock Riot API client for deterministic tests and deployments without an active API key.
#[derive(Default)]
pub struct MockRiotApiClient;

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
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
