use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::path::PathBuf;
use std::str::FromStr;

/// Discord API limits. These are protocol constraints, not deployment settings.
pub mod discord_limits {
    pub const ACTIVITY_TEXT_CHARS: usize = 128;
    pub const BAN_DELETE_DAYS: u8 = 7;
    pub const BULK_DELETE_MESSAGES: u8 = 100;
    pub const EMBEDS_PER_MESSAGE: usize = 10;
    pub const EMBED_FIELD_CHARS: usize = 1_024;
}

#[derive(Debug, Clone)]
pub struct EmbedColors {
    pub primary: u32,
    pub success: u32,
    pub warning: u32,
    pub error: u32,
    pub neutral: u32,
    pub server_info: u32,
    pub presence: u32,
    pub online: u32,
    pub idle: u32,
    pub do_not_disturb: u32,
    pub invisible: u32,
}

/// Runtime settings loaded from `.env`/the process environment.
#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub database_url: String,
    pub data_directory: PathBuf,
    pub log_filter: String,
    pub owner_ids: HashSet<u64>,
    pub default_prefix: String,
    pub default_language: String,
    pub prefix_max_chars: usize,
    pub cache_max_messages: usize,
    pub purge_max_messages: u8,
    pub purge_confirmation_seconds: u64,
    pub ban_max_delete_days: u8,
    pub presence_max_duration_minutes: u64,
    pub gateway_resume_delay_seconds: u64,
    pub gateway_ready_delay_seconds: u64,
    pub message_preview_chars: usize,
    pub message_content_enabled: bool,
    pub message_log_chunk_chars: usize,
    pub message_timestamp_format: String,
    pub attachment_max_bytes: u64,
    pub purge_attachment_max_total_bytes: u64,
    pub colors: EmbedColors,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenvy::from_filename("config.env").context("Failed to load config.env")?;
        let _ = dotenvy::dotenv();
        Self::from_env()
    }

    pub fn from_env() -> Result<Self> {
        let config = Self {
            discord_token: required("DISCORD_TOKEN")?,
            database_url: required("DATABASE_URL")?,
            data_directory: required::<PathBuf>("DATA_DIRECTORY")?,
            log_filter: required("RUST_LOG")?,
            owner_ids: comma_separated("OWNER_IDS")?,
            default_prefix: required("DEFAULT_PREFIX")?,
            default_language: required("DEFAULT_LANGUAGE")?,
            prefix_max_chars: required("PREFIX_MAX_CHARS")?,
            cache_max_messages: required("CACHE_MAX_MESSAGES")?,
            purge_max_messages: required("PURGE_MAX_MESSAGES")?,
            purge_confirmation_seconds: required("PURGE_CONFIRMATION_SECONDS")?,
            ban_max_delete_days: required("BAN_MAX_DELETE_DAYS")?,
            presence_max_duration_minutes: required("PRESENCE_MAX_DURATION_MINUTES")?,
            gateway_resume_delay_seconds: required("GATEWAY_RESUME_DELAY_SECONDS")?,
            gateway_ready_delay_seconds: required("GATEWAY_READY_DELAY_SECONDS")?,
            message_preview_chars: required("MESSAGE_PREVIEW_CHARS")?,
            message_content_enabled: required("MESSAGE_CONTENT_ENABLED")?,
            message_log_chunk_chars: required("MESSAGE_LOG_CHUNK_CHARS")?,
            message_timestamp_format: required("MESSAGE_TIMESTAMP_FORMAT")?,
            attachment_max_bytes: required("ATTACHMENT_MAX_BYTES")?,
            purge_attachment_max_total_bytes: required("PURGE_ATTACHMENT_MAX_TOTAL_BYTES")?,
            colors: EmbedColors {
                primary: hex_color("EMBED_COLOR_PRIMARY")?,
                success: hex_color("EMBED_COLOR_SUCCESS")?,
                warning: hex_color("EMBED_COLOR_WARNING")?,
                error: hex_color("EMBED_COLOR_ERROR")?,
                neutral: hex_color("EMBED_COLOR_NEUTRAL")?,
                server_info: hex_color("EMBED_COLOR_SERVER_INFO")?,
                presence: hex_color("EMBED_COLOR_PRESENCE")?,
                online: hex_color("EMBED_COLOR_ONLINE")?,
                idle: hex_color("EMBED_COLOR_IDLE")?,
                do_not_disturb: hex_color("EMBED_COLOR_DO_NOT_DISTURB")?,
                invisible: hex_color("EMBED_COLOR_INVISIBLE")?,
            },
        };

        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.default_prefix.is_empty()
            || self.default_prefix.chars().count() > self.prefix_max_chars
        {
            bail!("DEFAULT_PREFIX must contain 1..=PREFIX_MAX_CHARS characters");
        }
        if !matches!(self.default_language.as_str(), "en" | "vi" | "ja") {
            bail!("DEFAULT_LANGUAGE must be one of: en, vi, ja");
        }
        if self.purge_max_messages == 0
            || self.purge_max_messages > discord_limits::BULK_DELETE_MESSAGES
        {
            bail!("PURGE_MAX_MESSAGES exceeds Discord's bulk-delete limit");
        }
        if self.ban_max_delete_days > discord_limits::BAN_DELETE_DAYS {
            bail!("BAN_MAX_DELETE_DAYS exceeds Discord's ban limit");
        }
        if self.message_preview_chars == 0
            || self.message_preview_chars > discord_limits::EMBED_FIELD_CHARS.saturating_sub(6)
            || self.message_log_chunk_chars == 0
            || self.message_log_chunk_chars > discord_limits::EMBED_FIELD_CHARS.saturating_sub(6)
        {
            bail!("message log sizes must fit in a Discord embed field");
        }
        if self.attachment_max_bytes == 0 {
            bail!("ATTACHMENT_MAX_BYTES must be greater than zero");
        }
        if self.purge_attachment_max_total_bytes < self.attachment_max_bytes {
            bail!("PURGE_ATTACHMENT_MAX_TOTAL_BYTES must be at least ATTACHMENT_MAX_BYTES");
        }
        Ok(())
    }
}

fn required<T>(name: &str) -> Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    std::env::var(name)
        .with_context(|| format!("Missing {name} environment variable"))?
        .parse()
        .with_context(|| format!("Invalid {name} environment variable"))
}

fn comma_separated<T>(name: &str) -> Result<T>
where
    T: FromIterator<u64>,
{
    let value = std::env::var(name).unwrap_or_default();
    value
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(|item| {
            item.trim()
                .parse::<u64>()
                .with_context(|| format!("Invalid value in {name}"))
        })
        .collect()
}

fn hex_color(name: &str) -> Result<u32> {
    let value =
        std::env::var(name).with_context(|| format!("Missing {name} environment variable"))?;
    let value = value
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x");
    let color = u32::from_str_radix(value, 16)
        .with_context(|| format!("Invalid hexadecimal color in {name}"))?;
    if color > 0xFF_FF_FF {
        bail!("{name} must be a 24-bit RGB color");
    }
    Ok(color)
}

#[cfg(test)]
mod tests {
    use super::hex_color;

    #[test]
    fn parses_hex_color_formats() {
        unsafe { std::env::set_var("TEST_COLOR", "#12abEF") };
        assert_eq!(hex_color("TEST_COLOR").unwrap(), 0x12_AB_EF);
        unsafe { std::env::remove_var("TEST_COLOR") };
    }
}
