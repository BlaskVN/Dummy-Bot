use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

use crate::valorant::riot_api::RiotRegion;

/// Supported League of Legends platform routing shards.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, poise::ChoiceParameter,
)]
pub enum LolPlatform {
    #[name = "vn2 (Vietnam)"]
    Vn2,
    #[name = "kr (Korea)"]
    Kr,
    #[name = "jp1 (Japan)"]
    Jp1,
    #[name = "na1 (North America)"]
    Na1,
    #[name = "euw1 (Europe West)"]
    Euw1,
    #[name = "eun1 (Europe Nordic & East)"]
    Eun1,
    #[name = "oc1 (Oceania)"]
    Oc1,
    #[name = "br1 (Brazil)"]
    Br1,
    #[name = "la1 (Latin America North)"]
    La1,
    #[name = "la2 (Latin America South)"]
    La2,
    #[name = "tr1 (Turkey)"]
    Tr1,
    #[name = "ru (Russia)"]
    Ru,
    #[name = "sg2 (Singapore)"]
    Sg2,
    #[name = "ph2 (Philippines)"]
    Ph2,
    #[name = "th2 (Thailand)"]
    Th2,
    #[name = "tw2 (Taiwan)"]
    Tw2,
}

impl LolPlatform {
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "vn2" | "vn" | "vietnam" => Some(Self::Vn2),
            "kr" | "korea" => Some(Self::Kr),
            "jp1" | "jp" | "japan" => Some(Self::Jp1),
            "na1" | "na" | "north_america" => Some(Self::Na1),
            "euw1" | "euw" | "europe_west" => Some(Self::Euw1),
            "eun1" | "eun" | "eune" | "europe_nordic" => Some(Self::Eun1),
            "oc1" | "oce" | "oceania" => Some(Self::Oc1),
            "br1" | "br" | "brazil" => Some(Self::Br1),
            "la1" | "lan" | "latin_america_north" => Some(Self::La1),
            "la2" | "las" | "latin_america_south" => Some(Self::La2),
            "tr1" | "tr" | "turkey" => Some(Self::Tr1),
            "ru" | "russia" => Some(Self::Ru),
            "sg2" | "sg" | "singapore" => Some(Self::Sg2),
            "ph2" | "ph" | "philippines" => Some(Self::Ph2),
            "th2" | "th" | "thailand" => Some(Self::Th2),
            "tw2" | "tw" | "taiwan" => Some(Self::Tw2),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Vn2 => "vn2",
            Self::Kr => "kr",
            Self::Jp1 => "jp1",
            Self::Na1 => "na1",
            Self::Euw1 => "euw1",
            Self::Eun1 => "eun1",
            Self::Oc1 => "oc1",
            Self::Br1 => "br1",
            Self::La1 => "la1",
            Self::La2 => "la2",
            Self::Tr1 => "tr1",
            Self::Ru => "ru",
            Self::Sg2 => "sg2",
            Self::Ph2 => "ph2",
            Self::Th2 => "th2",
            Self::Tw2 => "tw2",
        }
    }

    pub fn api_endpoint(&self) -> &'static str {
        match self {
            Self::Vn2 => "https://vn2.api.riotgames.com",
            Self::Kr => "https://kr.api.riotgames.com",
            Self::Jp1 => "https://jp1.api.riotgames.com",
            Self::Na1 => "https://na1.api.riotgames.com",
            Self::Euw1 => "https://euw1.api.riotgames.com",
            Self::Eun1 => "https://eun1.api.riotgames.com",
            Self::Oc1 => "https://oc1.api.riotgames.com",
            Self::Br1 => "https://br1.api.riotgames.com",
            Self::La1 => "https://la1.api.riotgames.com",
            Self::La2 => "https://la2.api.riotgames.com",
            Self::Tr1 => "https://tr1.api.riotgames.com",
            Self::Ru => "https://ru.api.riotgames.com",
            Self::Sg2 => "https://sg2.api.riotgames.com",
            Self::Ph2 => "https://ph2.api.riotgames.com",
            Self::Th2 => "https://th2.api.riotgames.com",
            Self::Tw2 => "https://tw2.api.riotgames.com",
        }
    }

    pub fn regional_cluster_endpoint(&self) -> &'static str {
        match self {
            Self::Na1 | Self::Br1 | Self::La1 | Self::La2 => "https://americas.api.riotgames.com",
            Self::Kr | Self::Jp1 => "https://asia.api.riotgames.com",
            Self::Euw1 | Self::Eun1 | Self::Tr1 | Self::Ru => "https://europe.api.riotgames.com",
            Self::Oc1 | Self::Ph2 | Self::Sg2 | Self::Th2 | Self::Tw2 | Self::Vn2 => {
                "https://sea.api.riotgames.com"
            }
        }
    }

    pub fn from_riot_region(region: RiotRegion) -> Self {
        match region {
            RiotRegion::Ap => Self::Vn2,
            RiotRegion::Na => Self::Na1,
            RiotRegion::Eu => Self::Euw1,
            RiotRegion::Kr => Self::Kr,
            RiotRegion::Br => Self::Br1,
            RiotRegion::Latam => Self::La1,
        }
    }
}

/// Official LoL Summoner details from Summoner-V4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LolSummoner {
    pub puuid: String,
    pub profile_icon_id: i32,
    pub summoner_level: u64,
    #[serde(default)]
    pub revision_date: i64,
}

/// League of Legends ranked tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LolRankedTier {
    Iron,
    Bronze,
    Silver,
    Gold,
    Platinum,
    Emerald,
    Diamond,
    Master,
    Grandmaster,
    Challenger,
    Unranked,
}

impl LolRankedTier {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Iron => "Iron",
            Self::Bronze => "Bronze",
            Self::Silver => "Silver",
            Self::Gold => "Gold",
            Self::Platinum => "Platinum",
            Self::Emerald => "Emerald",
            Self::Diamond => "Diamond",
            Self::Master => "Master",
            Self::Grandmaster => "Grandmaster",
            Self::Challenger => "Challenger",
            Self::Unranked => "Unranked",
        }
    }

    pub fn try_parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "IRON" => Some(Self::Iron),
            "BRONZE" => Some(Self::Bronze),
            "SILVER" => Some(Self::Silver),
            "GOLD" => Some(Self::Gold),
            "PLATINUM" => Some(Self::Platinum),
            "EMERALD" => Some(Self::Emerald),
            "DIAMOND" => Some(Self::Diamond),
            "MASTER" => Some(Self::Master),
            "GRANDMASTER" => Some(Self::Grandmaster),
            "CHALLENGER" => Some(Self::Challenger),
            "UNRANKED" => Some(Self::Unranked),
            _ => None,
        }
    }
}

impl std::fmt::Display for LolRankedTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Serialize for LolRankedTier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.name())
    }
}

impl<'de> Deserialize<'de> for LolRankedTier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self::try_parse(&s).unwrap_or(Self::Unranked))
    }
}

/// Queue type for League of Legends league entries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LolQueueType {
    SoloDuo,
    Flex,
    Other(String),
}

impl LolQueueType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::SoloDuo => "RANKED_SOLO_5x5",
            Self::Flex => "RANKED_FLEX_SR",
            Self::Other(s) => s.as_str(),
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            Self::SoloDuo => "Ranked Solo/Duo",
            Self::Flex => "Ranked Flex",
            Self::Other(s) => s.as_str(),
        }
    }
}

impl Serialize for LolQueueType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for LolQueueType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "RANKED_SOLO_5x5" => Ok(Self::SoloDuo),
            "RANKED_FLEX_SR" => Ok(Self::Flex),
            _ => Ok(Self::Other(s)),
        }
    }
}

/// Official League Entry details from League-V4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LolLeagueEntry {
    pub queue_type: LolQueueType,
    pub tier: LolRankedTier,
    #[serde(default)]
    pub rank: String,
    pub league_points: i32,
    pub wins: u32,
    pub losses: u32,
}

impl LolLeagueEntry {
    pub fn winrate(&self) -> f64 {
        let total = self.wins + self.losses;
        if total == 0 {
            0.0
        } else {
            (self.wins as f64 / total as f64) * 100.0
        }
    }
}

/// Summary of a recent League of Legends match from Match-V5.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LolRecentMatch {
    pub match_id: String,
    pub game_mode: String,
    #[serde(default)]
    pub map_id: i32,
    #[serde(default)]
    pub map_name: String,
    pub game_start_millis: i64,
    pub game_duration: u32,
    pub champion_id: u32,
    pub champion_name: String,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub win: bool,
    pub items: [u32; 7],
    pub cs: u32,
}

/// Champion mastery entry from Champion-Mastery-V4.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LolChampionMastery {
    pub champion_id: u32,
    pub champion_name: String,
    pub champion_level: u32,
    pub champion_points: u32,
    pub last_play_time: i64,
}

/// Map known League of Legends champion IDs to canonical names.
pub fn champion_name_by_id(id: u32) -> String {
    let name = match id {
        1 => "Annie",
        2 => "Olaf",
        3 => "Galio",
        4 => "Twisted Fate",
        5 => "Xin Zhao",
        6 => "Urgot",
        7 => "LeBlanc",
        8 => "Vladimir",
        9 => "Fiddlesticks",
        10 => "Kayle",
        11 => "Master Yi",
        12 => "Alistar",
        13 => "Ryze",
        14 => "Sion",
        15 => "Sivir",
        16 => "Soraka",
        17 => "Teemo",
        18 => "Tristana",
        19 => "Warwick",
        20 => "Nunu & Willump",
        21 => "Miss Fortune",
        22 => "Ashe",
        23 => "Tryndamere",
        24 => "Jax",
        25 => "Morgana",
        26 => "Zilean",
        27 => "Singed",
        28 => "Evelynn",
        29 => "Twitch",
        30 => "Karthus",
        31 => "Cho'Gath",
        32 => "Amumu",
        33 => "Rammus",
        34 => "Anivia",
        35 => "Shaco",
        36 => "Dr. Mundo",
        37 => "Sona",
        38 => "Kassadin",
        39 => "Irelia",
        40 => "Janna",
        41 => "Gangplank",
        42 => "Corki",
        43 => "Karma",
        44 => "Taric",
        45 => "Veigar",
        48 => "Trundle",
        50 => "Swain",
        51 => "Caitlyn",
        53 => "Blitzcrank",
        54 => "Malphite",
        55 => "Katarina",
        56 => "Nocturne",
        57 => "Maokai",
        58 => "Renekton",
        59 => "Jarvan IV",
        60 => "Elise",
        61 => "Orianna",
        62 => "Wukong",
        63 => "Brand",
        64 => "Lee Sin",
        67 => "Vayne",
        68 => "Rumble",
        69 => "Cassiopeia",
        72 => "Skarner",
        74 => "Heimerdinger",
        75 => "Nasus",
        76 => "Nidalee",
        77 => "Udyr",
        78 => "Poppy",
        79 => "Gragas",
        80 => "Pantheon",
        81 => "Ezreal",
        82 => "Mordekaiser",
        83 => "Yorick",
        84 => "Akali",
        85 => "Kennen",
        86 => "Garen",
        89 => "Leona",
        90 => "Malzahar",
        91 => "Talon",
        92 => "Riven",
        96 => "Kog'Maw",
        98 => "Shen",
        99 => "Lux",
        101 => "Xerath",
        102 => "Shyvana",
        103 => "Ahri",
        104 => "Graves",
        105 => "Fizz",
        106 => "Volibear",
        107 => "Rengar",
        110 => "Varus",
        111 => "Nautilus",
        112 => "Viktor",
        113 => "Sejuani",
        114 => "Fiora",
        115 => "Ziggs",
        117 => "Lulu",
        119 => "Draven",
        120 => "Hecarim",
        121 => "Kha'Zix",
        122 => "Darius",
        126 => "Jayce",
        127 => "Lissandra",
        131 => "Diana",
        133 => "Quinn",
        134 => "Syndra",
        136 => "Aurelion Sol",
        141 => "Kayn",
        142 => "Zoe",
        143 => "Zyra",
        145 => "Kai'Sa",
        147 => "Seraphine",
        150 => "Gnar",
        154 => "Zac",
        157 => "Yasuo",
        161 => "Vel'Koz",
        163 => "Taliyah",
        164 => "Camille",
        166 => "Akshan",
        200 => "Bel'Veth",
        201 => "Braum",
        202 => "Jhin",
        203 => "Kindred",
        221 => "Zeri",
        222 => "Jinx",
        223 => "Tahm Kench",
        233 => "Briar",
        234 => "Viego",
        235 => "Senna",
        236 => "Lucian",
        238 => "Zed",
        240 => "Kled",
        245 => "Ekko",
        246 => "Qiyana",
        254 => "Vi",
        266 => "Aatrox",
        267 => "Nami",
        268 => "Azir",
        350 => "Yuumi",
        360 => "Samira",
        412 => "Thresh",
        420 => "Illaoi",
        421 => "Rek'Sai",
        427 => "Ivern",
        429 => "Kalista",
        432 => "Bard",
        497 => "Rakan",
        498 => "Xayah",
        516 => "Ornn",
        517 => "Sylas",
        518 => "Neeko",
        523 => "Aphelios",
        526 => "Rell",
        555 => "Pyke",
        711 => "Vex",
        777 => "Yone",
        875 => "Sett",
        876 => "Lillia",
        887 => "Gwen",
        888 => "Renata Glasc",
        893 => "Nilah",
        897 => "K'Sante",
        901 => "Smolder",
        902 => "Milio",
        910 => "Hwei",
        950 => "Naafiri",
        _ => return format!("Champion {id}"),
    };
    name.to_string()
}

/// Map known League of Legends map IDs to canonical map names.
pub fn map_name_by_id(id: i32) -> &'static str {
    match id {
        1 | 2 | 11 => "Summoner's Rift",
        3 => "The Proving Grounds",
        4 | 10 => "Twisted Treeline",
        8 => "The Crystal Scar",
        12 => "Howling Abyss",
        21 => "Nexus Blitz",
        22 => "Convergence",
        30 => "Rings of Wrath",
        _ => "Unknown Map",
    }
}

/// Seam trait for League of Legends official API interactions.
pub trait LolApiClient: Send + Sync {
    fn get_summoner_by_puuid<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<LolSummoner>>;

    fn get_league_entries<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<Vec<LolLeagueEntry>>>;

    fn get_recent_matches<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<LolRecentMatch>>>;

    fn get_top_champion_masteries<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<LolChampionMastery>>>;

    fn get_total_mastery_score<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<i32>>;
}

/// Structured domain errors for League of Legends API interactions.
#[derive(Debug)]
pub enum LolApiError {
    NotFound,
    Forbidden,
    Unauthorized,
    Api {
        status: reqwest::StatusCode,
        message: String,
    },
    Transport(reqwest::Error),
}

impl std::fmt::Display for LolApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "LoL summoner or match data not found (404)"),
            Self::Forbidden => write!(f, "LoL API access forbidden (403)"),
            Self::Unauthorized => write!(f, "LoL API unauthorized or key expired (401)"),
            Self::Api { status, message } => write!(f, "LoL API error {status}: {message}"),
            Self::Transport(err) => write!(f, "Network error communicating with LoL API: {err}"),
        }
    }
}

impl std::error::Error for LolApiError {}

/// Production HTTP League of Legends API client using reqwest with X-Riot-Token.
pub struct HttpLolApiClient {
    api_key: String,
    client: reqwest::Client,
}

impl HttpLolApiClient {
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
            .map_err(LolApiError::Transport)
            .with_context(|| format!("Failed to send GET request to {url}"))?;

        let status = resp.status();
        if !status.is_success() {
            let message = resp.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::NOT_FOUND {
                return Err(LolApiError::NotFound.into());
            }
            if status == reqwest::StatusCode::FORBIDDEN {
                return Err(LolApiError::Forbidden.into());
            }
            if status == reqwest::StatusCode::UNAUTHORIZED {
                return Err(LolApiError::Unauthorized.into());
            }
            return Err(LolApiError::Api { status, message }.into());
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

#[derive(Deserialize)]
struct MatchParticipantDto {
    puuid: String,
    #[serde(rename = "championId")]
    champion_id: u32,
    #[serde(rename = "championName")]
    champion_name: Option<String>,
    kills: u32,
    deaths: u32,
    assists: u32,
    win: bool,
    #[serde(default)]
    item0: u32,
    #[serde(default)]
    item1: u32,
    #[serde(default)]
    item2: u32,
    #[serde(default)]
    item3: u32,
    #[serde(default)]
    item4: u32,
    #[serde(default)]
    item5: u32,
    #[serde(default)]
    item6: u32,
    #[serde(rename = "totalMinionsKilled", default)]
    total_minions_killed: u32,
    #[serde(rename = "neutralMinionsKilled", default)]
    neutral_minions_killed: u32,
}

#[derive(Deserialize)]
struct MatchInfoDto {
    #[serde(rename = "gameMode")]
    game_mode: String,
    #[serde(rename = "mapId", default)]
    map_id: i32,
    #[serde(rename = "gameStartTimestamp")]
    game_start_timestamp: i64,
    #[serde(rename = "gameDuration")]
    game_duration: u64,
    participants: Vec<MatchParticipantDto>,
}

#[derive(Deserialize)]
struct MatchDto {
    #[serde(default)]
    metadata: Option<MatchMetadataDto>,
    info: MatchInfoDto,
}

#[derive(Deserialize)]
struct MatchMetadataDto {
    #[serde(rename = "matchId")]
    match_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChampionMasteryRawDto {
    champion_id: u32,
    champion_level: u32,
    champion_points: u32,
    last_play_time: i64,
}

impl LolApiClient for HttpLolApiClient {
    fn get_summoner_by_puuid<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<LolSummoner>> {
        Box::pin(async move {
            let base = platform.api_endpoint();
            let url = format!("{base}/lol/summoner/v4/summoners/by-puuid/{puuid}");
            self.get_json(&url).await
        })
    }

    fn get_league_entries<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<Vec<LolLeagueEntry>>> {
        Box::pin(async move {
            let base = platform.api_endpoint();
            let url = format!("{base}/lol/league/v4/entries/by-puuid/{puuid}");
            self.get_json::<Vec<LolLeagueEntry>>(&url).await
        })
    }

    fn get_recent_matches<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<LolRecentMatch>>> {
        Box::pin(async move {
            let cluster_base = platform.regional_cluster_endpoint();
            let list_url = format!(
                "{cluster_base}/lol/match/v5/matches/by-puuid/{puuid}/ids?start=0&count={count}"
            );
            let match_ids: Vec<String> = self.get_json(&list_url).await?;

            let mut results = Vec::new();
            for match_id in match_ids.into_iter().take(count) {
                let match_url = format!("{cluster_base}/lol/match/v5/matches/{match_id}");
                let match_dto: MatchDto = match self.get_json(&match_url).await {
                    Ok(m) => m,
                    Err(e) => {
                        tracing::warn!(err = %e, match_id = %match_id, "Failed to fetch match-v5 details");
                        continue;
                    }
                };

                let participant = match_dto
                    .info
                    .participants
                    .into_iter()
                    .find(|p| p.puuid == puuid);

                if let Some(p) = participant {
                    let champion_name = p
                        .champion_name
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| champion_name_by_id(p.champion_id));

                    // In match-v5, duration may be in seconds or ms. If > 10,000 it is ms.
                    let duration_secs = if match_dto.info.game_duration > 10_000 {
                        (match_dto.info.game_duration / 1000) as u32
                    } else {
                        match_dto.info.game_duration as u32
                    };

                    let map_id = match_dto.info.map_id;
                    let map_name = map_name_by_id(map_id).to_string();

                    results.push(LolRecentMatch {
                        match_id: match_dto.metadata.map(|m| m.match_id).unwrap_or(match_id),
                        game_mode: match_dto.info.game_mode,
                        map_id,
                        map_name,
                        game_start_millis: match_dto.info.game_start_timestamp,
                        game_duration: duration_secs,
                        champion_id: p.champion_id,
                        champion_name,
                        kills: p.kills,
                        deaths: p.deaths,
                        assists: p.assists,
                        win: p.win,
                        items: [
                            p.item0, p.item1, p.item2, p.item3, p.item4, p.item5, p.item6,
                        ],
                        cs: p.total_minions_killed + p.neutral_minions_killed,
                    });
                }
            }

            Ok(results)
        })
    }

    fn get_top_champion_masteries<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<LolChampionMastery>>> {
        Box::pin(async move {
            let base = platform.api_endpoint();
            let url = format!(
                "{base}/lol/champion-mastery/v4/champion-masteries/by-puuid/{puuid}/top?count={count}"
            );
            let raw: Vec<ChampionMasteryRawDto> = self.get_json(&url).await?;

            let masteries = raw
                .into_iter()
                .map(|r| LolChampionMastery {
                    champion_id: r.champion_id,
                    champion_name: champion_name_by_id(r.champion_id),
                    champion_level: r.champion_level,
                    champion_points: r.champion_points,
                    last_play_time: r.last_play_time,
                })
                .collect();

            Ok(masteries)
        })
    }

    fn get_total_mastery_score<'a>(
        &'a self,
        platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<i32>> {
        Box::pin(async move {
            let base = platform.api_endpoint();
            let url = format!("{base}/lol/champion-mastery/v4/scores/by-puuid/{puuid}");
            self.get_json(&url).await
        })
    }
}

/// Deterministic offline mock League of Legends API client.
#[derive(Default)]
pub struct MockLolApiClient;

impl MockLolApiClient {
    pub fn generate_mock_summoner(puuid: &str) -> LolSummoner {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(puuid, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);

        LolSummoner {
            puuid: puuid.to_string(),
            profile_icon_id: ((hash % 500) + 1) as i32,
            summoner_level: (hash % 300) + 30,
            revision_date: 1758038760000,
        }
    }

    pub fn generate_mock_league_entries(puuid: &str) -> Vec<LolLeagueEntry> {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(puuid, &mut hasher);
        let hash = std::hash::Hasher::finish(&hasher);

        let tiers = [
            LolRankedTier::Iron,
            LolRankedTier::Bronze,
            LolRankedTier::Silver,
            LolRankedTier::Gold,
            LolRankedTier::Platinum,
            LolRankedTier::Emerald,
            LolRankedTier::Diamond,
            LolRankedTier::Master,
            LolRankedTier::Grandmaster,
            LolRankedTier::Challenger,
        ];
        let tier = tiers[(hash as usize) % tiers.len()];
        let divisions = ["I", "II", "III", "IV"];
        let rank = divisions[((hash >> 3) as usize) % divisions.len()].to_string();
        let lp = ((hash >> 5) % 100) as i32;
        let wins = ((hash >> 7) % 150 + 20) as u32;
        let losses = ((hash >> 11) % 150 + 15) as u32;

        vec![
            LolLeagueEntry {
                queue_type: LolQueueType::SoloDuo,
                tier,
                rank: rank.clone(),
                league_points: lp,
                wins,
                losses,
            },
            LolLeagueEntry {
                queue_type: LolQueueType::Flex,
                tier: LolRankedTier::Emerald,
                rank: "II".to_string(),
                league_points: 65,
                wins: 45,
                losses: 30,
            },
        ]
    }

    pub fn generate_mock_recent_matches(puuid: &str, count: usize) -> Vec<LolRecentMatch> {
        let base_matches = vec![
            LolRecentMatch {
                match_id: format!("VN2_MOCK_1_{puuid}"),
                game_mode: "CLASSIC".to_string(),
                map_id: 11,
                map_name: "Summoner's Rift".to_string(),
                game_start_millis: 1718000000000,
                game_duration: 1832,
                champion_id: 157,
                champion_name: champion_name_by_id(157),
                kills: 12,
                deaths: 4,
                assists: 7,
                win: true,
                items: [3031, 3046, 3006, 3072, 3026, 3153, 3363],
                cs: 245,
            },
            LolRecentMatch {
                match_id: format!("VN2_MOCK_2_{puuid}"),
                game_mode: "CLASSIC".to_string(),
                map_id: 11,
                map_name: "Summoner's Rift".to_string(),
                game_start_millis: 1717990000000,
                game_duration: 1520,
                champion_id: 222,
                champion_name: champion_name_by_id(222),
                kills: 8,
                deaths: 7,
                assists: 5,
                win: false,
                items: [3031, 3085, 3006, 1055, 0, 0, 3363],
                cs: 198,
            },
            LolRecentMatch {
                match_id: format!("VN2_MOCK_3_{puuid}"),
                game_mode: "ARAM".to_string(),
                map_id: 12,
                map_name: "Howling Abyss".to_string(),
                game_start_millis: 1717980000000,
                game_duration: 1150,
                champion_id: 103,
                champion_name: champion_name_by_id(103),
                kills: 15,
                deaths: 6,
                assists: 22,
                win: true,
                items: [6655, 3089, 3157, 3020, 3135, 4645, 3340],
                cs: 65,
            },
            LolRecentMatch {
                match_id: format!("VN2_MOCK_4_{puuid}"),
                game_mode: "CLASSIC".to_string(),
                map_id: 11,
                map_name: "Summoner's Rift".to_string(),
                game_start_millis: 1717970000000,
                game_duration: 2100,
                champion_id: 64,
                champion_name: champion_name_by_id(64),
                kills: 6,
                deaths: 8,
                assists: 14,
                win: false,
                items: [3078, 3053, 3111, 3748, 0, 0, 3364],
                cs: 170,
            },
            LolRecentMatch {
                match_id: format!("VN2_MOCK_5_{puuid}"),
                game_mode: "CLASSIC".to_string(),
                map_id: 11,
                map_name: "Summoner's Rift".to_string(),
                game_start_millis: 1717960000000,
                game_duration: 1650,
                champion_id: 202,
                champion_name: champion_name_by_id(202),
                kills: 11,
                deaths: 2,
                assists: 9,
                win: true,
                items: [3031, 3094, 3009, 3036, 1038, 0, 3363],
                cs: 220,
            },
        ];

        base_matches.into_iter().take(count).collect()
    }

    pub fn generate_mock_masteries(count: usize) -> Vec<LolChampionMastery> {
        let list = vec![
            LolChampionMastery {
                champion_id: 157,
                champion_name: "Yasuo".to_string(),
                champion_level: 7,
                champion_points: 450200,
                last_play_time: 1718000000000,
            },
            LolChampionMastery {
                champion_id: 64,
                champion_name: "Lee Sin".to_string(),
                champion_level: 7,
                champion_points: 310500,
                last_play_time: 1717980000000,
            },
            LolChampionMastery {
                champion_id: 222,
                champion_name: "Jinx".to_string(),
                champion_level: 6,
                champion_points: 185400,
                last_play_time: 1717900000000,
            },
            LolChampionMastery {
                champion_id: 103,
                champion_name: "Ahri".to_string(),
                champion_level: 5,
                champion_points: 98200,
                last_play_time: 1717500000000,
            },
            LolChampionMastery {
                champion_id: 238,
                champion_name: "Zed".to_string(),
                champion_level: 5,
                champion_points: 82100,
                last_play_time: 1717000000000,
            },
        ];
        list.into_iter().take(count).collect()
    }
}

impl LolApiClient for MockLolApiClient {
    fn get_summoner_by_puuid<'a>(
        &'a self,
        _platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<LolSummoner>> {
        Box::pin(async move { Ok(Self::generate_mock_summoner(puuid)) })
    }

    fn get_league_entries<'a>(
        &'a self,
        _platform: LolPlatform,
        puuid: &'a str,
    ) -> BoxFuture<'a, Result<Vec<LolLeagueEntry>>> {
        Box::pin(async move { Ok(Self::generate_mock_league_entries(puuid)) })
    }

    fn get_recent_matches<'a>(
        &'a self,
        _platform: LolPlatform,
        puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<LolRecentMatch>>> {
        Box::pin(async move { Ok(Self::generate_mock_recent_matches(puuid, count)) })
    }

    fn get_top_champion_masteries<'a>(
        &'a self,
        _platform: LolPlatform,
        _puuid: &'a str,
        count: usize,
    ) -> BoxFuture<'a, Result<Vec<LolChampionMastery>>> {
        Box::pin(async move { Ok(Self::generate_mock_masteries(count)) })
    }

    fn get_total_mastery_score<'a>(
        &'a self,
        _platform: LolPlatform,
        _puuid: &'a str,
    ) -> BoxFuture<'a, Result<i32>> {
        Box::pin(async move { Ok(145) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_try_parse_and_routing() {
        assert_eq!(LolPlatform::try_parse("vn2"), Some(LolPlatform::Vn2));
        assert_eq!(LolPlatform::try_parse("VN"), Some(LolPlatform::Vn2));
        assert_eq!(LolPlatform::try_parse("vietnam"), Some(LolPlatform::Vn2));
        assert_eq!(LolPlatform::try_parse("kr"), Some(LolPlatform::Kr));
        assert_eq!(LolPlatform::try_parse("korea"), Some(LolPlatform::Kr));
        assert_eq!(LolPlatform::try_parse("na"), Some(LolPlatform::Na1));
        assert_eq!(LolPlatform::try_parse("euw"), Some(LolPlatform::Euw1));
        assert_eq!(LolPlatform::try_parse("eune"), Some(LolPlatform::Eun1));
        assert_eq!(LolPlatform::try_parse("lan"), Some(LolPlatform::La1));
        assert_eq!(LolPlatform::try_parse("las"), Some(LolPlatform::La2));
        assert_eq!(LolPlatform::try_parse("oce"), Some(LolPlatform::Oc1));
        assert_eq!(LolPlatform::try_parse("br"), Some(LolPlatform::Br1));
        assert_eq!(LolPlatform::try_parse("tr"), Some(LolPlatform::Tr1));
        assert_eq!(LolPlatform::try_parse("ru"), Some(LolPlatform::Ru));
        assert_eq!(LolPlatform::try_parse("sg"), Some(LolPlatform::Sg2));
        assert_eq!(LolPlatform::try_parse("ph"), Some(LolPlatform::Ph2));
        assert_eq!(LolPlatform::try_parse("th"), Some(LolPlatform::Th2));
        assert_eq!(LolPlatform::try_parse("tw"), Some(LolPlatform::Tw2));
        assert_eq!(LolPlatform::try_parse("unknown"), None);
    }

    #[test]
    fn platform_from_riot_region_mapping() {
        assert_eq!(
            LolPlatform::from_riot_region(RiotRegion::Ap),
            LolPlatform::Vn2
        );
        assert_eq!(
            LolPlatform::from_riot_region(RiotRegion::Na),
            LolPlatform::Na1
        );
        assert_eq!(
            LolPlatform::from_riot_region(RiotRegion::Eu),
            LolPlatform::Euw1
        );
        assert_eq!(
            LolPlatform::from_riot_region(RiotRegion::Kr),
            LolPlatform::Kr
        );
        assert_eq!(
            LolPlatform::from_riot_region(RiotRegion::Br),
            LolPlatform::Br1
        );
        assert_eq!(
            LolPlatform::from_riot_region(RiotRegion::Latam),
            LolPlatform::La1
        );
    }

    #[test]
    fn regional_clusters_cover_all_platforms() {
        assert_eq!(
            LolPlatform::Vn2.regional_cluster_endpoint(),
            "https://sea.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::Sg2.regional_cluster_endpoint(),
            "https://sea.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::Na1.regional_cluster_endpoint(),
            "https://americas.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::La1.regional_cluster_endpoint(),
            "https://americas.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::Kr.regional_cluster_endpoint(),
            "https://asia.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::Jp1.regional_cluster_endpoint(),
            "https://asia.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::Euw1.regional_cluster_endpoint(),
            "https://europe.api.riotgames.com"
        );
        assert_eq!(
            LolPlatform::Ru.regional_cluster_endpoint(),
            "https://europe.api.riotgames.com"
        );
    }

    #[test]
    fn champion_name_lookup_and_fallback() {
        assert_eq!(champion_name_by_id(157), "Yasuo");
        assert_eq!(champion_name_by_id(222), "Jinx");
        assert_eq!(champion_name_by_id(266), "Aatrox");
        assert_eq!(champion_name_by_id(103), "Ahri");
        assert_eq!(champion_name_by_id(64), "Lee Sin");
        assert_eq!(champion_name_by_id(777), "Yone");
        assert_eq!(champion_name_by_id(99999), "Champion 99999");
    }

    #[test]
    fn deserializes_summoner_json() {
        let json = r#"{
            "id": "sum-123",
            "accountId": "acc-123",
            "puuid": "puuid-123",
            "profileIconId": 542,
            "revisionDate": 1600000000000,
            "summonerLevel": 250
        }"#;
        let summoner: LolSummoner = serde_json::from_str(json).unwrap();
        assert_eq!(summoner.puuid, "puuid-123");
        assert_eq!(summoner.profile_icon_id, 542);
        assert_eq!(summoner.summoner_level, 250);
        assert_eq!(summoner.revision_date, 1600000000000);
    }

    #[test]
    fn deserializes_modern_summoner_json() {
        let json = r#"{
            "puuid": "5ihTQlUoGVGv5umCYdGJZes-p60ZWbZ9VJw7_hSaIudkjDoOUTr0UqnnWMVx5MLVS50mMIGUKlslbQ",
            "profileIconId": 6353,
            "revisionDate": 1758038760000,
            "summonerLevel": 37
        }"#;
        let summoner: LolSummoner = serde_json::from_str(json).unwrap();
        assert_eq!(
            summoner.puuid,
            "5ihTQlUoGVGv5umCYdGJZes-p60ZWbZ9VJw7_hSaIudkjDoOUTr0UqnnWMVx5MLVS50mMIGUKlslbQ"
        );
        assert_eq!(summoner.profile_icon_id, 6353);
        assert_eq!(summoner.summoner_level, 37);
        assert_eq!(summoner.revision_date, 1758038760000);
    }

    #[test]
    fn deserializes_league_entry_and_calculates_winrate() {
        let json = r#"{
            "queueType": "RANKED_SOLO_5x5",
            "tier": "EMERALD",
            "rank": "I",
            "leaguePoints": 75,
            "wins": 60,
            "losses": 40
        }"#;
        let entry: LolLeagueEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.queue_type, LolQueueType::SoloDuo);
        assert_eq!(entry.tier, LolRankedTier::Emerald);
        assert_eq!(entry.rank, "I");
        assert_eq!(entry.league_points, 75);
        assert_eq!(entry.wins, 60);
        assert_eq!(entry.losses, 40);
        assert!((entry.winrate() - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn deserializes_match_dto_and_parses_participant() {
        let json = r#"{
            "metadata": {
                "matchId": "VN2_99999"
            },
            "info": {
                "gameMode": "CLASSIC",
                "gameStartTimestamp": 1718000000000,
                "gameDuration": 1820,
                "participants": [
                    {
                        "puuid": "puuid-test-1",
                        "championId": 157,
                        "championName": "Yasuo",
                        "kills": 10,
                        "deaths": 3,
                        "assists": 8,
                        "win": true,
                        "item0": 3031,
                        "item1": 3046,
                        "item2": 3006,
                        "item3": 0,
                        "item4": 0,
                        "item5": 0,
                        "item6": 3340,
                        "totalMinionsKilled": 190,
                        "neutralMinionsKilled": 20
                    }
                ]
            }
        }"#;
        let dto: MatchDto = serde_json::from_str(json).unwrap();
        assert_eq!(dto.info.game_mode, "CLASSIC");
        assert_eq!(dto.info.participants.len(), 1);
        let p = &dto.info.participants[0];
        assert_eq!(p.champion_id, 157);
        assert_eq!(p.kills, 10);
        assert!(p.win);
        assert_eq!(p.total_minions_killed + p.neutral_minions_killed, 210);
    }

    #[tokio::test]
    async fn mock_client_returns_complete_data() {
        let mock = MockLolApiClient;
        let puuid = "puuid-mock-test";

        let summoner = mock
            .get_summoner_by_puuid(LolPlatform::Vn2, puuid)
            .await
            .unwrap();
        assert_eq!(summoner.puuid, puuid);
        assert!(summoner.summoner_level >= 30);

        let entries = mock
            .get_league_entries(LolPlatform::Vn2, puuid)
            .await
            .unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].queue_type, LolQueueType::SoloDuo);
        assert_eq!(entries[1].queue_type, LolQueueType::Flex);

        let matches = mock
            .get_recent_matches(LolPlatform::Vn2, puuid, 3)
            .await
            .unwrap();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0].champion_id, 157);
        assert_eq!(matches[0].champion_name, "Yasuo");
        assert_eq!(matches[0].items.len(), 7);
        assert_eq!(matches[0].map_id, 11);
        assert_eq!(matches[0].map_name, "Summoner's Rift");
        assert_eq!(matches[2].map_id, 12);
        assert_eq!(matches[2].map_name, "Howling Abyss");

        let masteries = mock
            .get_top_champion_masteries(LolPlatform::Vn2, puuid, 2)
            .await
            .unwrap();
        assert_eq!(masteries.len(), 2);
        assert_eq!(masteries[0].champion_name, "Yasuo");
        assert_eq!(masteries[1].champion_name, "Lee Sin");

        let score = mock
            .get_total_mastery_score(LolPlatform::Vn2, puuid)
            .await
            .unwrap();
        assert_eq!(score, 145);
    }

    #[test]
    fn map_name_mapping_covers_known_maps() {
        assert_eq!(map_name_by_id(11), "Summoner's Rift");
        assert_eq!(map_name_by_id(12), "Howling Abyss");
        assert_eq!(map_name_by_id(21), "Nexus Blitz");
        assert_eq!(map_name_by_id(9999), "Unknown Map");
    }

    #[test]
    fn lol_api_error_display() {
        assert_eq!(
            LolApiError::NotFound.to_string(),
            "LoL summoner or match data not found (404)"
        );
        assert_eq!(
            LolApiError::Forbidden.to_string(),
            "LoL API access forbidden (403)"
        );
    }
}
