pub mod api;
pub mod service;

pub use api::{
    BoxFuture, HttpLolApiClient, LolApiClient, LolApiError, LolChampionMastery, LolLeagueEntry,
    LolPlatform, LolQueueType, LolRankedTier, LolRecentMatch, LolSummoner, MockLolApiClient,
    champion_name_by_id, map_name_by_id,
};
pub use service::{
    LolDomainError, LolMasteryData, LolMasteryError, LolMatchesError, LolProfile, LolProfileError,
    LolRecentMatchesData, LolService,
};
