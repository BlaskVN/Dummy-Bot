pub mod api;
pub mod service;

pub use api::{
    BoxFuture, HttpLolApiClient, LolApiClient, LolApiError, LolChampionMastery, LolLeagueEntry,
    LolPlatform, LolQueueType, LolRankedTier, LolRecentMatch, LolSummoner, MockLolApiClient,
    champion_name_by_id,
};
pub use service::{
    LolMasteryData, LolMasteryError, LolMatchesError, LolProfile, LolProfileError,
    LolRecentMatchesData, LolService,
};
