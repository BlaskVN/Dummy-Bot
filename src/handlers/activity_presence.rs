use crate::Data;
use poise::serenity_prelude as serenity;

/// Presence absence is not a negative signal: members may hide Activity Sharing.
pub async fn handle_presence_update(
    _ctx: &serenity::Context,
    _presence: &serenity::Presence,
    _data: &Data,
) {
}

pub fn detection_status(enabled: bool) -> &'static str {
    if enabled { "available" } else { "degraded" }
}

#[cfg(test)]
mod tests {
    use super::detection_status;

    #[test]
    fn renders_detection_availability() {
        assert_eq!(detection_status(true), "available");
        assert_eq!(detection_status(false), "degraded");
    }
}
