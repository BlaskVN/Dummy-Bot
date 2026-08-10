use crate::automod::{observer_enabled, set_observer_enabled};
use crate::i18n::{TranslationKey, t};
use crate::ui::{self, Tone};
use crate::{Context, Error};
use anyhow::Context as _;

/// Configure passive observation of Discord AutoMod events.
#[poise::command(
    rename = "automod-observer",
    slash_command,
    subcommands("enable", "disable", "status"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn automod_observer(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enable passive AutoMod event observation for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn enable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().context("Not in a guild")?;
    let lang = ctx.data().language(guild_id).await;
    let channel_configured: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM moderation_channel_config WHERE guild_id = ?)",
    )
    .bind(guild_id.to_string())
    .fetch_one(&ctx.data().db_pool)
    .await?;
    if !channel_configured {
        ui::reply(
            ctx,
            Tone::Warning,
            t(lang, TranslationKey::AutoModObserverNeedsChannel),
        )
        .await?;
        return Ok(());
    }
    let bot_can_manage_guild = {
        let guild = ctx.guild().context("Guild is unavailable in cache")?;
        let bot = guild
            .members
            .get(&ctx.cache().current_user().id)
            .context("Bot member is unavailable in cache")?;
        guild.member_permissions(bot).manage_guild()
    };
    if !bot_can_manage_guild {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::AutoModObserverNeedsPermission),
        )
        .await?;
        return Ok(());
    }
    set_observer_enabled(&ctx.data().db_pool, guild_id, true).await?;
    ui::reply(
        ctx,
        Tone::Success,
        t(lang, TranslationKey::AutoModObserverEnabled),
    )
    .await?;
    Ok(())
}

/// Disable passive AutoMod event observation for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().context("Not in a guild")?;
    let lang = ctx.data().language(guild_id).await;
    set_observer_enabled(&ctx.data().db_pool, guild_id, false).await?;
    ui::reply(
        ctx,
        Tone::Success,
        t(lang, TranslationKey::AutoModObserverDisabled),
    )
    .await?;
    Ok(())
}

/// Show whether AutoMod event observation is enabled.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().context("Not in a guild")?;
    let lang = ctx.data().language(guild_id).await;
    let key = if observer_enabled(&ctx.data().db_pool, guild_id).await? {
        TranslationKey::AutoModObserverEnabled
    } else {
        TranslationKey::AutoModObserverDisabled
    };
    ui::reply(ctx, Tone::Neutral, t(lang, key)).await?;
    Ok(())
}
