use poise::serenity_prelude::ChannelId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLogHealth {
    Disabled,
    Healthy,
    Degraded,
}

impl MessageLogHealth {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "healthy" => Self::Healthy,
            "degraded" => Self::Degraded,
            _ => Self::Disabled,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageLogConfig {
    pub channel_id: ChannelId,
    pub enabled: bool,
    pub health: MessageLogHealth,
}

#[derive(Debug, Clone)]
pub struct MessageLogOptions {
    pub preview_chars: usize,
    pub chunk_chars: usize,
    pub timestamp_format: String,
    pub attachment_max_bytes: u64,
    pub purge_attachment_max_total_bytes: u64,
    pub message_content_enabled: bool,
    pub error_color: poise::serenity_prelude::Colour,
    pub warning_color: poise::serenity_prelude::Colour,
}

impl Default for MessageLogOptions {
    fn default() -> Self {
        Self {
            preview_chars: 500,
            chunk_chars: 1000,
            timestamp_format: "%Y-%m-%d %H:%M:%S".to_string(),
            attachment_max_bytes: 1024 * 1024,
            purge_attachment_max_total_bytes: 10 * 1024 * 1024,
            message_content_enabled: true,
            error_color: poise::serenity_prelude::Colour::RED,
            warning_color: poise::serenity_prelude::Colour::GOLD,
        }
    }
}
