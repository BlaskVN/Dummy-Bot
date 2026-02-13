use crate::i18n::{get_guild_language, t, tf, Language, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

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
            BotStatus::Online => 0x43b581,    // Green
            BotStatus::Idle => 0xfaa61a,      // Yellow/Orange
            BotStatus::DoNotDisturb => 0xf04747, // Red
            BotStatus::Invisible => 0x747f8d,  // Gray
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
}

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

    // Set the presence using the shard messenger
    ctx.serenity_context()
        .set_presence(None, online_status);

    tracing::info!(
        status = new_status.display_name(),
        duration_minutes = ?duration_minutes,
        owner = %ctx.author().name,
        "Bot status updated"
    );

    let description = if let Some(mins) = duration_minutes {
        if mins > 0 {
            // Schedule a revert task
            let ctx_serenity = ctx.serenity_context().clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(mins * 60)).await;
                ctx_serenity.set_presence(None, serenity::OnlineStatus::Online);
                tracing::info!(
                    "Bot status reverted to Online after {} minutes",
                    mins
                );
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

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceStatusTitle))
        .description(description)
        .color(new_status.color());

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

    tracing::info!(
        activity_type = kind.display_name(),
        activity_text = %text,
        status = ?new_status.map(|s| s.display_name()),
        duration_minutes = ?duration_minutes,
        owner = %ctx.author().name,
        "Bot activity updated"
    );

    // Schedule revert if duration is set
    if let Some(mins) = duration_minutes {
        if mins > 0 {
            let ctx_serenity = ctx.serenity_context().clone();
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(mins * 60)).await;
                ctx_serenity
                    .set_presence(None, serenity::OnlineStatus::Online);
                tracing::info!(
                    "Bot activity cleared and status reverted to Online after {} minutes",
                    mins
                );
            });
        }
    }

    let status_name = new_status
        .map(|s| s.display_name())
        .unwrap_or("Online");

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

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceActivityTitle))
        .description(description)
        .color(color);

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
