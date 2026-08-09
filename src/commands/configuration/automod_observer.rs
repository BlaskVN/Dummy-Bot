use crate::automod::{observer_enabled, set_observer_enabled};
use crate::i18n::{TranslationKey, t};
use crate::{Context, Error};
use anyhow::Context as _;

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
        ctx.say(t(lang, TranslationKey::AutoModObserverNeedsChannel))
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
        ctx.say(t(lang, TranslationKey::AutoModObserverNeedsPermission))
            .await?;
        return Ok(());
    }
    set_observer_enabled(&ctx.data().db_pool, guild_id, true).await?;
    ctx.say(t(lang, TranslationKey::AutoModObserverEnabled))
        .await?;
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().context("Not in a guild")?;
    let lang = ctx.data().language(guild_id).await;
    set_observer_enabled(&ctx.data().db_pool, guild_id, false).await?;
    ctx.say(t(lang, TranslationKey::AutoModObserverDisabled))
        .await?;
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().context("Not in a guild")?;
    let lang = ctx.data().language(guild_id).await;
    let key = if observer_enabled(&ctx.data().db_pool, guild_id).await? {
        TranslationKey::AutoModObserverEnabled
    } else {
        TranslationKey::AutoModObserverDisabled
    };
    ctx.say(t(lang, key)).await?;
    Ok(())
}
