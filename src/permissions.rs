use crate::Context;
use anyhow::{Context as _, Result};
use poise::serenity_prelude as serenity;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModerationDenial {
    SelfTarget,
    UserHierarchy,
    BotHierarchy,
}

/// Apply Discord's native role hierarchy to a moderation target.
pub fn moderation_denial(
    ctx: Context<'_>,
    target: serenity::UserId,
) -> Result<Option<ModerationDenial>> {
    let guild = ctx.guild().context("Guild is unavailable in cache")?;
    let actor = ctx.author().id;
    let bot = ctx.cache().current_user().id;

    if target == actor || target == bot {
        return Ok(Some(ModerationDenial::SelfTarget));
    }

    if actor != guild.owner_id
        && guild.greater_member_hierarchy(ctx.cache(), actor, target) != Some(actor)
    {
        return Ok(Some(ModerationDenial::UserHierarchy));
    }

    if guild.greater_member_hierarchy(ctx.cache(), bot, target) != Some(bot) {
        return Ok(Some(ModerationDenial::BotHierarchy));
    }

    Ok(None)
}

/// Return permissions missing after Discord guild roles and channel overwrites are resolved.
pub fn missing_channel_permissions(
    ctx: Context<'_>,
    channel_id: serenity::ChannelId,
    user_id: serenity::UserId,
    required: serenity::Permissions,
) -> Result<serenity::Permissions> {
    let guild = ctx.guild().context("Guild is unavailable in cache")?;
    let channel = guild
        .channels
        .get(&channel_id)
        .context("Channel is unavailable in cache")?;
    let member = guild
        .members
        .get(&user_id)
        .context("Member is unavailable in cache")?;
    Ok(required - guild.user_permissions_in(channel, member))
}
