use crate::database::{clear_bot_presence, load_bot_presence, save_bot_presence};
use crate::i18n::{get_guild_language, t, tf, Language, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

/// Available bot status options mapped to Discord's OnlineStatus.
#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
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
    fn to_online_status(self) -> serenity::OnlineStatus {
        match self {
            BotStatus::Online => serenity::OnlineStatus::Online,
            BotStatus::Idle => serenity::OnlineStatus::Idle,
            BotStatus::DoNotDisturb => serenity::OnlineStatus::DoNotDisturb,
            BotStatus::Invisible => serenity::OnlineStatus::Invisible,
        }
    }

    /// Get display name for logging/messages.
    fn display_name(self) -> &'static str {
        match self {
            BotStatus::Online => "Online",
            BotStatus::Idle => "Idle",
            BotStatus::DoNotDisturb => "Do Not Disturb",
            BotStatus::Invisible => "Invisible",
        }
    }

    /// Embed color for each status.
    fn color(self) -> u32 {
        match self {
            BotStatus::Online => 0x43b581,
            BotStatus::Idle => 0xfaa61a,
            BotStatus::DoNotDisturb => 0xf04747,
            BotStatus::Invisible => 0x747f8d,
        }
    }

    /// Lowercase key stored in the database.
    fn to_db_str(self) -> &'static str {
        match self {
            BotStatus::Online => "online",
            BotStatus::Idle => "idle",
            BotStatus::DoNotDisturb => "dnd",
            BotStatus::Invisible => "invisible",
        }
    }

    /// Parse a database key back to the enum.
    fn from_db_str(s: &str) -> Option<Self> {
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
#[derive(Debug, poise::ChoiceParameter, Clone, Copy)]
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
    fn to_activity_type(self) -> serenity::ActivityType {
        match self {
            ActivityKind::Playing => serenity::ActivityType::Playing,
            ActivityKind::Listening => serenity::ActivityType::Listening,
            ActivityKind::Watching => serenity::ActivityType::Watching,
            ActivityKind::Competing => serenity::ActivityType::Competing,
            ActivityKind::Custom => serenity::ActivityType::Custom,
        }
    }

    /// Get display name.
    fn display_name(self) -> &'static str {
        match self {
            ActivityKind::Playing => "Playing",
            ActivityKind::Listening => "Listening to",
            ActivityKind::Watching => "Watching",
            ActivityKind::Competing => "Competing in",
            ActivityKind::Custom => "Custom",
        }
    }

    /// Lowercase key stored in the database.
    fn to_db_str(self) -> &'static str {
        match self {
            ActivityKind::Playing => "playing",
            ActivityKind::Listening => "listening",
            ActivityKind::Watching => "watching",
            ActivityKind::Competing => "competing",
            ActivityKind::Custom => "custom",
        }
    }

    /// Parse a database key back to the enum.
    fn from_db_str(s: &str) -> Option<Self> {
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

// ─── Public helper called on bot startup ────────────────────────────────────

/// Restore the bot's presence from the database after a restart.
/// No-ops silently if no persistent presence is stored.
pub async fn restore_presence(ctx: &serenity::Context, pool: &SqlitePool) {
    match load_bot_presence(pool).await {
        Ok(Some(record)) => {
            let online_status = BotStatus::from_db_str(&record.status)
                .map(|s| s.to_online_status())
                .unwrap_or(serenity::OnlineStatus::Online);

            let activity = record
                .activity_kind
                .as_deref()
                .and_then(ActivityKind::from_db_str)
                .zip(record.activity_text.as_deref())
                .map(|(kind, text)| serenity::ActivityData {
                    name: text.to_owned(),
                    kind: kind.to_activity_type(),
                    state: if matches!(kind, ActivityKind::Custom) {
                        Some(text.to_owned())
                    } else {
                        None
                    },
                    url: None,
                });

            ctx.set_presence(activity, online_status);
            tracing::info!(
                status = %record.status,
                activity_kind = ?record.activity_kind,
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

// ─── Commands ───────────────────────────────────────────────────────────────

/// Manage bot presence — status and Rich Presence. (Owner only)
#[poise::command(
    slash_command,
    prefix_command,
    subcommands("status", "activity", "clear_activity"),
    owners_only,
    hide_in_help
)]
pub async fn presence(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceTitle))
        .description(t(lang, TranslationKey::PresenceHelp))
        .color(0x7289da);

    ctx.send(poise::CreateReply::default().embed(embed))
        .await?;
    Ok(())
}

/// Set the bot's online status with optional auto-revert duration.
#[poise::command(slash_command, prefix_command, owners_only)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "Bot status to set"] new_status: BotStatus,
    #[description = "Duration in minutes (0 = permanent)"]
    #[min = 0]
    #[max = 1440]
    duration_minutes: Option<u64>,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    let online_status = new_status.to_online_status();
    let is_permanent = duration_minutes.map_or(true, |m| m == 0);

    ctx.serenity_context()
        .set_presence(None, online_status);

    // Persist only when permanent so the bot restores it after a restart.
    if is_permanent {
        if let Err(e) = save_bot_presence(
            &ctx.data().db_pool,
            new_status.to_db_str(),
            None,
            None,
        )
        .await
        {
            tracing::warn!(error = %e, "Failed to persist bot status to database");
        }
    }

    tracing::info!(
        status = new_status.display_name(),
        duration_minutes = ?duration_minutes,
        persistent = is_permanent,
        owner = %ctx.author().name,
        "Bot status updated"
    );

    let description = if let Some(mins) = duration_minutes {
        if mins > 0 {
            let ctx_serenity = ctx.serenity_context().clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(mins * 60)).await;
                ctx_serenity.set_presence(None, serenity::OnlineStatus::Online);
                tracing::info!("Bot status reverted to Online after {} minutes", mins);
            });
            tf(
                lang,
                TranslationKey::PresenceStatusSetDuration,
                &[&new_status.display_name(), &mins],
            )
        } else {
            tf(
                lang,
                TranslationKey::PresenceStatusSet,
                &[&new_status.display_name()],
            )
        }
    } else {
        tf(
            lang,
            TranslationKey::PresenceStatusSet,
            &[&new_status.display_name()],
        )
    };

    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceStatusTitle))
        .description(description)
        .color(new_status.color());

    if is_permanent {
        embed = embed.footer(serenity::CreateEmbedFooter::new(
            "Saved persistently — will restore on bot restart",
        ));
    }

    ctx.send(poise::CreateReply::default().embed(embed))
        .await?;

    Ok(())
}

/// Set the bot's Rich Presence activity (Playing, Listening, Watching, Competing, Custom).
#[poise::command(slash_command, prefix_command, owners_only)]
pub async fn activity(
    ctx: Context<'_>,
    #[description = "Activity type"] kind: ActivityKind,
    #[description = "Activity text / name"]
    #[max_length = 128]
    text: String,
    #[description = "Optional status to set alongside"]
    new_status: Option<BotStatus>,
    #[description = "Duration in minutes (0 = permanent)"]
    #[min = 0]
    #[max = 1440]
    duration_minutes: Option<u64>,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    let online_status = new_status
        .map(|s| s.to_online_status())
        .unwrap_or(serenity::OnlineStatus::Online);

    let is_permanent = duration_minutes.map_or(true, |m| m == 0);

    let activity = serenity::ActivityData {
        name: text.clone(),
        kind: kind.to_activity_type(),
        state: if matches!(kind, ActivityKind::Custom) {
            Some(text.clone())
        } else {
            None
        },
        url: None,
    };

    ctx.serenity_context()
        .set_presence(Some(activity.clone()), online_status);

    // Persist only when permanent.
    if is_permanent {
        let status_str = new_status
            .map(|s| s.to_db_str())
            .unwrap_or("online");
        if let Err(e) = save_bot_presence(
            &ctx.data().db_pool,
            status_str,
            Some(kind.to_db_str()),
            Some(&text),
        )
        .await
        {
            tracing::warn!(error = %e, "Failed to persist bot activity to database");
        }
    }

    tracing::info!(
        activity_type = kind.display_name(),
        activity_text = %text,
        status = ?new_status.map(|s| s.display_name()),
        duration_minutes = ?duration_minutes,
        persistent = is_permanent,
        owner = %ctx.author().name,
        "Bot activity updated"
    );

    if let Some(mins) = duration_minutes {
        if mins > 0 {
            let ctx_serenity = ctx.serenity_context().clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(mins * 60)).await;
                ctx_serenity.set_presence(None, serenity::OnlineStatus::Online);
                tracing::info!(
                    "Bot activity cleared and status reverted to Online after {} minutes",
                    mins
                );
            });
        }
    }

    let status_name = new_status.map(|s| s.display_name()).unwrap_or("Online");

    let description = if let Some(mins) = duration_minutes {
        if mins > 0 {
            tf(
                lang,
                TranslationKey::PresenceActivitySetDuration,
                &[&kind.display_name(), &text, &status_name, &mins],
            )
        } else {
            tf(
                lang,
                TranslationKey::PresenceActivitySet,
                &[&kind.display_name(), &text, &status_name],
            )
        }
    } else {
        tf(
            lang,
            TranslationKey::PresenceActivitySet,
            &[&kind.display_name(), &text, &status_name],
        )
    };

    let color = new_status.map(|s| s.color()).unwrap_or(0x43b581);

    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceActivityTitle))
        .description(description)
        .color(color);

    if is_permanent {
        embed = embed.footer(serenity::CreateEmbedFooter::new(
            "Saved persistently — will restore on bot restart",
        ));
    }

    ctx.send(poise::CreateReply::default().embed(embed))
        .await?;

    Ok(())
}

/// Clear the bot's current activity / Rich Presence and reset to Online.
#[poise::command(
    slash_command,
    prefix_command,
    owners_only,
    rename = "clear"
)]
pub async fn clear_activity(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    ctx.serenity_context()
        .set_presence(None, serenity::OnlineStatus::Online);

    // Remove from database so the next restart doesn't restore the old presence.
    if let Err(e) = clear_bot_presence(&ctx.data().db_pool).await {
        tracing::warn!(error = %e, "Failed to clear persistent bot presence from database");
    }

    tracing::info!(
        owner = %ctx.author().name,
        "Bot activity cleared"
    );

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceActivityTitle))
        .description(t(lang, TranslationKey::PresenceActivityCleared))
        .color(0x43b581);

    ctx.send(poise::CreateReply::default().embed(embed))
        .await?;

    Ok(())
}
