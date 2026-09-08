use crate::i18n::{TranslationKey, t, tf};
use crate::message_log::{self, DiscordOutbox};
use crate::permissions::missing_channel_permissions;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Configure edited and deleted message logging for this Guild.
#[poise::command(
    slash_command,
    subcommands("enable", "disable", "status"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn messagelog(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enable message logging for this Guild.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn enable(
    ctx: Context<'_>,
    #[description = "Channel to send message logs to"] log_channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    let required = serenity::Permissions::VIEW_CHANNEL
        | serenity::Permissions::SEND_MESSAGES
        | serenity::Permissions::EMBED_LINKS
        | serenity::Permissions::ATTACH_FILES;
    let missing =
        missing_channel_permissions(ctx, log_channel.id, ctx.cache().current_user().id, required)?;
    if !missing.is_empty() {
        let message = tf(
            lang,
            TranslationKey::ModerationBotMissingPermissions,
            &[&missing],
        );
        ui::reply(ctx, Tone::Error, message).await?;
        return Ok(());
    }

    let warning_panel = serenity::CreateMessage::new()
        .embed(ui::panel(
            ctx.data(),
            Tone::Warning,
            t(lang, TranslationKey::MessageLogDegradedWarning),
        ))
        .allowed_mentions(serenity::CreateAllowedMentions::new());
    let outbox = DiscordOutbox::new(ctx.http());

    message_log::enable_with_outbox(
        &ctx.data().db_pool,
        guild_id,
        log_channel.id,
        ctx.data().config.message_content_enabled,
        Some((&outbox, warning_panel)),
    )
    .await?;

    tracing::info!(
        guild = %guild_id,
        channel = %log_channel.id,
        admin = %ctx.author().name,
        "Message logging enabled"
    );

    let message = tf(lang, TranslationKey::MessageLogEnabled, &[&log_channel.id]);

    let embed = ui::panel(ctx.data(), Tone::Success, message);

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}

/// Disable message logging for this Guild.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    let disabled = message_log::disable(
        &ctx.data().db_pool,
        guild_id,
        ctx.data().config.message_content_enabled,
    )
    .await?;

    if !disabled {
        let embed = ui::panel(
            ctx.data(),
            Tone::Warning,
            t(lang, TranslationKey::MessageLogNotSetup),
        );
        ctx.send(ui::embed_reply(embed)).await?;
        return Ok(());
    }

    tracing::info!(
        guild = %guild_id,
        admin = %ctx.author().name,
        "Message logging disabled"
    );

    let embed = ui::panel(
        ctx.data(),
        Tone::Success,
        t(lang, TranslationKey::MessageLogDisabled),
    );

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}

/// Show this Guild's current message logging status.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    let config = message_log::get_config(&ctx.data().db_pool, guild_id).await?;

    match config {
        Some(config) => {
            let description = message_log::format_status_description(lang, &config);

            let embed = ui::embed(ctx.data(), Tone::Primary)
                .title(t(lang, TranslationKey::MessageLogStatusTitle))
                .description(description);

            ctx.send(ui::embed_reply(embed)).await?;
        }
        None => {
            let embed = ui::panel(
                ctx.data(),
                Tone::Warning,
                t(lang, TranslationKey::MessageLogUseEnable),
            );
            ctx.send(ui::embed_reply(embed)).await?;
        }
    }

    Ok(())
}
