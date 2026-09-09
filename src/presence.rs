use anyhow::{Context, Result};
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

use crate::config::EmbedColors;

/// Persistent presence configuration stored across bot restarts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotPresenceRecord {
    pub status: BotStatus,
    pub activity_kind: Option<ActivityKind>,
    pub activity_text: Option<String>,
}

impl BotPresenceRecord {
    pub fn new(
        status: BotStatus,
        activity_kind: Option<ActivityKind>,
        activity_text: Option<String>,
    ) -> Self {
        Self {
            status,
            activity_kind,
            activity_text,
        }
    }

    /// Convert status to serenity's OnlineStatus.
    pub fn to_online_status(&self) -> serenity::OnlineStatus {
        self.status.to_online_status()
    }

    /// Convert activity kind and text to serenity's ActivityData.
    pub fn to_activity_data(&self) -> Option<serenity::ActivityData> {
        self.activity_kind
            .zip(self.activity_text.as_deref())
            .map(|(kind, text)| serenity::ActivityData {
                name: text.to_owned(),
                kind: kind.to_activity_type(),
                state: if matches!(kind, ActivityKind::Custom) {
                    Some(text.to_owned())
                } else {
                    None
                },
                url: None,
            })
    }
}

/// Available bot status options mapped to Discord's OnlineStatus.
#[derive(Debug, poise::ChoiceParameter, Clone, Copy, PartialEq, Eq)]
pub enum BotStatus {
    #[name = "Online"]
    Online,
    #[name = "Idle"]
    Idle,
    #[name = "Do Not Disturb"]
    DoNotDisturb,
    #[name = "Invisible"]
    Invisible,
}

impl BotStatus {
    /// Convert to serenity's OnlineStatus.
    pub fn to_online_status(self) -> serenity::OnlineStatus {
        match self {
            BotStatus::Online => serenity::OnlineStatus::Online,
            BotStatus::Idle => serenity::OnlineStatus::Idle,
            BotStatus::DoNotDisturb => serenity::OnlineStatus::DoNotDisturb,
            BotStatus::Invisible => serenity::OnlineStatus::Invisible,
        }
    }

    /// Get display name for logging/messages.
    pub fn display_name(self) -> &'static str {
        match self {
            BotStatus::Online => "Online",
            BotStatus::Idle => "Idle",
            BotStatus::DoNotDisturb => "Do Not Disturb",
            BotStatus::Invisible => "Invisible",
        }
    }

    /// Embed color for each status.
    pub fn color(self, colors: &EmbedColors) -> u32 {
        match self {
            BotStatus::Online => colors.online,
            BotStatus::Idle => colors.idle,
            BotStatus::DoNotDisturb => colors.do_not_disturb,
            BotStatus::Invisible => colors.invisible,
        }
    }

    /// Lowercase key stored in the database.
    pub fn to_db_str(self) -> &'static str {
        match self {
            BotStatus::Online => "online",
            BotStatus::Idle => "idle",
            BotStatus::DoNotDisturb => "dnd",
            BotStatus::Invisible => "invisible",
        }
    }

    /// Parse a database key back to the enum.
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "online" => Some(BotStatus::Online),
            "idle" => Some(BotStatus::Idle),
            "dnd" => Some(BotStatus::DoNotDisturb),
            "invisible" => Some(BotStatus::Invisible),
            _ => None,
        }
    }
}

/// Activity type options for Rich Presence.
#[derive(Debug, poise::ChoiceParameter, Clone, Copy, PartialEq, Eq)]
pub enum ActivityKind {
    #[name = "Playing"]
    Playing,
    #[name = "Listening"]
    Listening,
    #[name = "Watching"]
    Watching,
    #[name = "Competing"]
    Competing,
    #[name = "Custom"]
    Custom,
}

impl ActivityKind {
    /// Convert to serenity's ActivityType.
    pub fn to_activity_type(self) -> serenity::ActivityType {
        match self {
            ActivityKind::Playing => serenity::ActivityType::Playing,
            ActivityKind::Listening => serenity::ActivityType::Listening,
            ActivityKind::Watching => serenity::ActivityType::Watching,
            ActivityKind::Competing => serenity::ActivityType::Competing,
            ActivityKind::Custom => serenity::ActivityType::Custom,
        }
    }

    /// Get display name.
    pub fn display_name(self) -> &'static str {
        match self {
            ActivityKind::Playing => "Playing",
            ActivityKind::Listening => "Listening to",
            ActivityKind::Watching => "Watching",
            ActivityKind::Competing => "Competing in",
            ActivityKind::Custom => "Custom",
        }
    }

    /// Lowercase key stored in the database.
    pub fn to_db_str(self) -> &'static str {
        match self {
            ActivityKind::Playing => "playing",
            ActivityKind::Listening => "listening",
            ActivityKind::Watching => "watching",
            ActivityKind::Competing => "competing",
            ActivityKind::Custom => "custom",
        }
    }

    /// Parse a database key back to the enum.
    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "playing" => Some(ActivityKind::Playing),
            "listening" => Some(ActivityKind::Listening),
            "watching" => Some(ActivityKind::Watching),
            "competing" => Some(ActivityKind::Competing),
            "custom" => Some(ActivityKind::Custom),
            _ => None,
        }
    }
}

/// Upsert the bot's persistent presence into the database.
/// Only call this when duration is permanent (0 or unset).
pub async fn save_bot_presence(pool: &SqlitePool, record: &BotPresenceRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO bot_presence (id, status, activity_kind, activity_text, updated_at)
         VALUES (1, ?, ?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET
             status        = excluded.status,
             activity_kind = excluded.activity_kind,
             activity_text = excluded.activity_text,
             updated_at    = CURRENT_TIMESTAMP",
    )
    .bind(record.status.to_db_str())
    .bind(record.activity_kind.map(|k| k.to_db_str()))
    .bind(&record.activity_text)
    .execute(pool)
    .await
    .context("Failed to save bot presence")?;
    Ok(())
}

/// Load the persistent presence row (there is at most one row with id = 1).
pub async fn load_bot_presence(pool: &SqlitePool) -> Result<Option<BotPresenceRecord>> {
    let row = sqlx::query_as::<_, (String, Option<String>, Option<String>)>(
        "SELECT status, activity_kind, activity_text FROM bot_presence WHERE id = 1",
    )
    .fetch_optional(pool)
    .await
    .context("Failed to load bot presence")?;

    Ok(row.and_then(|(status, activity_kind, activity_text)| {
        let status = BotStatus::from_db_str(&status)?;
        let activity_kind = activity_kind.as_deref().and_then(ActivityKind::from_db_str);
        Some(BotPresenceRecord {
            status,
            activity_kind,
            activity_text,
        })
    }))
}

/// Remove the persistent presence row so the bot starts with Discord's default.
pub async fn clear_bot_presence(pool: &SqlitePool) -> Result<()> {
    sqlx::query("DELETE FROM bot_presence WHERE id = 1")
        .execute(pool)
        .await
        .context("Failed to clear bot presence")?;
    Ok(())
}

/// Restore the bot's presence from the database after a restart.
/// No-ops silently if no persistent presence is stored.
pub async fn restore_presence(ctx: &serenity::Context, pool: &SqlitePool) {
    match load_bot_presence(pool).await {
        Ok(Some(record)) => {
            let online_status = record.to_online_status();
            let activity = record.to_activity_data();
            ctx.set_presence(activity, online_status);
            tracing::info!(
                status = record.status.display_name(),
                activity_kind = ?record.activity_kind.map(|k| k.display_name()),
                activity_text = ?record.activity_text,
                "Persistent bot presence restored from database"
            );
        }
        Ok(None) => {
            tracing::debug!("No persistent bot presence found in database");
        }
        Err(e) => {
            tracing::warn!(error = %e, "Failed to load persistent bot presence from database");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_and_activity_db_str_conversions() {
        let statuses = [
            (BotStatus::Online, "online", "Online"),
            (BotStatus::Idle, "idle", "Idle"),
            (BotStatus::DoNotDisturb, "dnd", "Do Not Disturb"),
            (BotStatus::Invisible, "invisible", "Invisible"),
        ];

        for (status, db_str, display_name) in statuses {
            assert_eq!(status.to_db_str(), db_str);
            assert_eq!(BotStatus::from_db_str(db_str), Some(status));
            assert_eq!(status.display_name(), display_name);
        }
        assert_eq!(BotStatus::from_db_str("unknown"), None);

        let activities = [
            (ActivityKind::Playing, "playing", "Playing"),
            (ActivityKind::Listening, "listening", "Listening to"),
            (ActivityKind::Watching, "watching", "Watching"),
            (ActivityKind::Competing, "competing", "Competing in"),
            (ActivityKind::Custom, "custom", "Custom"),
        ];

        for (activity, db_str, display_name) in activities {
            assert_eq!(activity.to_db_str(), db_str);
            assert_eq!(ActivityKind::from_db_str(db_str), Some(activity));
            assert_eq!(activity.display_name(), display_name);
        }
        assert_eq!(ActivityKind::from_db_str("unknown"), None);
    }

    #[tokio::test]
    async fn bot_presence_crud_roundtrip() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("Failed to connect to in-memory sqlite");

        sqlx::migrate!()
            .run(&pool)
            .await
            .expect("Failed to run migrations");

        // Initially empty
        let initial = load_bot_presence(&pool).await.unwrap();
        assert!(initial.is_none());

        // Save presence with record
        let record = BotPresenceRecord::new(
            BotStatus::Online,
            Some(ActivityKind::Playing),
            Some("Rust".to_string()),
        );
        save_bot_presence(&pool, &record).await.unwrap();

        let loaded = load_bot_presence(&pool).await.unwrap();
        assert_eq!(loaded, Some(record));

        // Update presence
        let updated_record = BotPresenceRecord::new(BotStatus::DoNotDisturb, None, None);
        save_bot_presence(&pool, &updated_record).await.unwrap();

        let loaded_updated = load_bot_presence(&pool).await.unwrap();
        assert_eq!(loaded_updated, Some(updated_record));

        // Clear presence
        clear_bot_presence(&pool).await.unwrap();

        let cleared = load_bot_presence(&pool).await.unwrap();
        assert!(cleared.is_none());
    }
}
