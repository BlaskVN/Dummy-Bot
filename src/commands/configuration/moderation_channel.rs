use crate::i18n::{TranslationKey, t, tf};
use crate::moderation_channel::{
    clear_moderation_channel, get_moderation_channel, set_moderation_channel,
    valid_moderation_channel,
};
use crate::permissions::missing_channel_permissions;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Configure the private channel used for moderation records.
#[poise::command(
    rename = "moderation-channel",
    slash_command,
    subcommands("set", "show", "clear"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn moderation_channel(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Set the private moderation records channel.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Private text channel for moderation records"] channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    if !valid_moderation_channel(guild_id, channel.guild_id, channel.kind) {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::ModerationChannelInvalid),
        )
        .await?;
        return Ok(());
    }

    let required = serenity::Permissions::VIEW_CHANNEL
        | serenity::Permissions::SEND_MESSAGES
        | serenity::Permissions::EMBED_LINKS;
    let missing =
        missing_channel_permissions(ctx, channel.id, ctx.cache().current_user().id, required)?;
    if !missing.is_empty() {
        let message = tf(
            lang,
            TranslationKey::ModerationBotMissingPermissions,
            &[&missing],
        );
        ui::reply(ctx, Tone::Error, message).await?;
        return Ok(());
    }

    set_moderation_channel(&ctx.data().db_pool, guild_id, channel.id).await?;
    let message = tf(lang, TranslationKey::ModerationChannelSet, &[&channel.id]);
    ui::reply(ctx, Tone::Success, message).await?;
    Ok(())
}

/// Show the currently configured moderation records channel.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn show(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    let channel = get_moderation_channel(&ctx.data().db_pool, guild_id).await?;
    ui::reply(
        ctx,
        Tone::Neutral,
        match channel {
            Some(channel) => tf(lang, TranslationKey::ModerationChannelCurrent, &[&channel]),
            None => t(lang, TranslationKey::ModerationChannelNotConfigured).to_owned(),
        },
    )
    .await?;
    Ok(())
}

/// Clear the configured moderation records channel.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    clear_moderation_channel(&ctx.data().db_pool, guild_id).await?;
    ui::reply(
        ctx,
        Tone::Success,
        t(lang, TranslationKey::ModerationChannelCleared),
    )
    .await?;
    Ok(())
}
