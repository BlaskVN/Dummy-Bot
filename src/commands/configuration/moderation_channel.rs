use crate::i18n::{TranslationKey, t, tf};
use crate::permissions::missing_channel_permissions;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

fn valid_channel(
    guild_id: serenity::GuildId,
    channel_guild_id: serenity::GuildId,
    kind: serenity::ChannelType,
) -> bool {
    guild_id == channel_guild_id && kind == serenity::ChannelType::Text
}

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

    if !valid_channel(guild_id, channel.guild_id, channel.kind) {
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

    sqlx::query(
        "INSERT INTO moderation_channel_config (guild_id, channel_id) VALUES (?, ?)\n         ON CONFLICT(guild_id) DO UPDATE SET channel_id = excluded.channel_id, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .bind(channel.id.to_string())
    .execute(&ctx.data().db_pool)
    .await?;
    let message = tf(lang, TranslationKey::ModerationChannelSet, &[&channel.id]);
    ui::reply(ctx, Tone::Success, message).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::valid_channel;
    use poise::serenity_prelude::{ChannelType, GuildId};

    #[test]
    fn accepts_only_current_guild_text_channels() {
        assert!(valid_channel(
            GuildId::new(1),
            GuildId::new(1),
            ChannelType::Text
        ));
        assert!(!valid_channel(
            GuildId::new(1),
            GuildId::new(2),
            ChannelType::Text
        ));
        assert!(!valid_channel(
            GuildId::new(1),
            GuildId::new(1),
            ChannelType::Voice
        ));
    }
}

/// Show the currently configured moderation records channel.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn show(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    let channel = sqlx::query_scalar::<_, String>(
        "SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?;
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
    sqlx::query("DELETE FROM moderation_channel_config WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&ctx.data().db_pool)
        .await?;
    ui::reply(
        ctx,
        Tone::Success,
        t(lang, TranslationKey::ModerationChannelCleared),
    )
    .await?;
    Ok(())
}
