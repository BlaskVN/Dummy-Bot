use crate::i18n::{TranslationKey, t, tf};
use crate::presence::{
    ActivityKind, BotPresenceRecord, BotStatus, clear_bot_presence, save_bot_presence,
};
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

// ─── Commands ───────────────────────────────────────────────────────────────

/// Manage the bot's status and Rich Presence activity.
#[poise::command(
    slash_command,
    subcommands("status", "activity", "clear_activity"),
    owners_only,
    hide_in_help
)]
pub async fn presence(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(t(lang, TranslationKey::PresenceTitle))
        .description(t(lang, TranslationKey::PresenceHelp));

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

/// Set the bot's online status with optional auto-revert duration.
#[poise::command(slash_command, owners_only)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "Bot status to set"] new_status: BotStatus,
    #[description = "Duration in minutes (0 = permanent)"]
    #[min = 0]
    duration_minutes: Option<u64>,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    if duration_minutes.is_some_and(|value| value > ctx.data().config.presence_max_duration_minutes)
    {
        let message = tf(
            lang,
            TranslationKey::PresenceDurationRange,
            &[&ctx.data().config.presence_max_duration_minutes],
        );
        ui::reply(ctx, Tone::Error, message).await?;
        return Ok(());
    }

    let online_status = new_status.to_online_status();
    let is_permanent = duration_minutes.is_none_or(|m| m == 0);

    ctx.serenity_context().set_presence(None, online_status);

    // Persist only when permanent so the bot restores it after a restart.
    if is_permanent
        && let Err(e) = save_bot_presence(
            &ctx.data().db_pool,
            &BotPresenceRecord::new(new_status, None, None),
        )
        .await
    {
        tracing::warn!(error = %e, "Failed to persist bot status to database");
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
        .color(new_status.color(&ctx.data().config.colors));

    if is_permanent {
        embed = embed.footer(serenity::CreateEmbedFooter::new(t(
            lang,
            TranslationKey::PresencePersistent,
        )));
    }

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}

/// Set the bot's Rich Presence activity (Playing, Listening, Watching, Competing, Custom).
#[poise::command(slash_command, owners_only)]
pub async fn activity(
    ctx: Context<'_>,
    #[description = "Activity type"] kind: ActivityKind,
    #[description = "Activity text / name"]
    #[max_length = 128]
    text: String,
    #[description = "Optional status to set alongside"] new_status: Option<BotStatus>,
    #[description = "Duration in minutes (0 = permanent)"]
    #[min = 0]
    duration_minutes: Option<u64>,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    if duration_minutes.is_some_and(|value| value > ctx.data().config.presence_max_duration_minutes)
    {
        let message = tf(
            lang,
            TranslationKey::PresenceDurationRange,
            &[&ctx.data().config.presence_max_duration_minutes],
        );
        ui::reply(ctx, Tone::Error, message).await?;
        return Ok(());
    }

    let online_status = new_status
        .map(|s| s.to_online_status())
        .unwrap_or(serenity::OnlineStatus::Online);

    let is_permanent = duration_minutes.is_none_or(|m| m == 0);

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
        let status = new_status.unwrap_or(BotStatus::Online);
        let record = BotPresenceRecord::new(status, Some(kind), Some(text.clone()));
        if let Err(e) = save_bot_presence(&ctx.data().db_pool, &record).await {
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

    if let Some(mins) = duration_minutes
        && mins > 0
    {
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

    let color = new_status
        .map(|s| s.color(&ctx.data().config.colors))
        .unwrap_or(ctx.data().config.colors.online);

    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::PresenceActivityTitle))
        .description(description)
        .color(color);

    if is_permanent {
        embed = embed.footer(serenity::CreateEmbedFooter::new(t(
            lang,
            TranslationKey::PresencePersistent,
        )));
    }

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}

/// Clear the bot's current activity / Rich Presence and reset to Online.
#[poise::command(slash_command, owners_only, rename = "clear")]
pub async fn clear_activity(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
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
        .color(ctx.data().config.colors.online);

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}
