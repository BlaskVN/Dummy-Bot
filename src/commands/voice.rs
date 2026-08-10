use crate::i18n::{TranslationKey, t, tf};
use crate::permissions::missing_channel_permissions;
use crate::{Context, Error, VoiceConnectionInfo};
use poise::serenity_prelude as serenity;

/// Join the voice channel you are currently in.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MOVE_MEMBERS",
    required_permissions = "MOVE_MEMBERS",
    user_cooldown = 15,
    guild_cooldown = 5,
    rename = "connect"
)]
pub async fn voice_connect(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    // Get the voice channel the user is currently in
    let user_voice_channel = {
        let guild = ctx
            .guild()
            .ok_or_else(|| anyhow::anyhow!("Could not fetch guild info"))?;
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|vs| vs.channel_id)
    };

    let voice_channel_id = match user_voice_channel {
        Some(id) => id,
        None => {
            let embed = serenity::CreateEmbed::new()
                .description(t(lang, TranslationKey::VoiceNotInChannel))
                .color(ctx.data().config.colors.error);
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
    };

    let user_missing = missing_channel_permissions(
        ctx,
        voice_channel_id,
        ctx.author().id,
        serenity::Permissions::CONNECT,
    )?;
    if !user_missing.is_empty() {
        let message = tf(
            lang,
            TranslationKey::ModerationUserMissingPermissions,
            &[&user_missing],
        );
        ctx.say(message).await?;
        return Ok(());
    }

    let bot_permissions = serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::CONNECT;
    let missing = missing_channel_permissions(
        ctx,
        voice_channel_id,
        ctx.cache().current_user().id,
        bot_permissions,
    )?;
    if !missing.is_empty() {
        let message = tf(
            lang,
            TranslationKey::ModerationBotMissingPermissions,
            &[&missing],
        );
        ctx.say(message).await?;
        return Ok(());
    }

    // Check if already connected to a voice channel in this guild
    if ctx
        .data()
        .voice_connections
        .read()
        .await
        .contains_key(&guild_id)
    {
        let embed = serenity::CreateEmbed::new()
            .description(t(lang, TranslationKey::VoiceAlreadyConnected))
            .color(ctx.data().config.colors.warning);
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    update_voice_state(ctx.serenity_context(), guild_id, Some(voice_channel_id));

    // Store connection info (text channel + voice channel) for kick notification & auto-reconnection
    {
        let mut map = ctx.data().voice_connections.write().await;
        map.insert(
            guild_id,
            VoiceConnectionInfo {
                text_channel_id: ctx.channel_id(),
                voice_channel_id,
            },
        );
    }

    tracing::info!(
        user = %ctx.author().name,
        guild = %guild_id,
        voice_channel = %voice_channel_id,
        "Bot joined voice channel"
    );

    let message = tf(lang, TranslationKey::VoiceConnected, &[&voice_channel_id]);
    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(ctx.data().config.colors.success);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Leave the current voice channel.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MOVE_MEMBERS",
    required_permissions = "MOVE_MEMBERS",
    guild_cooldown = 5,
    rename = "disconnect"
)]
pub async fn voice_disconnect(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    // Check if bot is in a voice channel
    if !ctx
        .data()
        .voice_connections
        .read()
        .await
        .contains_key(&guild_id)
    {
        let embed = serenity::CreateEmbed::new()
            .description(t(lang, TranslationKey::VoiceNotConnected))
            .color(ctx.data().config.colors.error);
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    // Remove from tracking FIRST (before remove) to prevent false kick notification / auto-reconnection
    {
        let mut map = ctx.data().voice_connections.write().await;
        map.remove(&guild_id);
    }

    update_voice_state(ctx.serenity_context(), guild_id, None);

    tracing::info!(
        user = %ctx.author().name,
        guild = %guild_id,
        "Bot left voice channel"
    );

    let embed = serenity::CreateEmbed::new()
        .description(t(lang, TranslationKey::VoiceDisconnected))
        .color(ctx.data().config.colors.success);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

pub fn all() -> Vec<poise::Command<crate::Data, Error>> {
    vec![voice_connect(), voice_disconnect()]
}

pub(crate) fn update_voice_state(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    channel_id: Option<serenity::ChannelId>,
) {
    ctx.shard
        .websocket_message(voice_state_payload(guild_id, channel_id).into());
}

fn voice_state_payload(
    guild_id: serenity::GuildId,
    channel_id: Option<serenity::ChannelId>,
) -> String {
    serenity::json::json!({
        "op": 4,
        "d": {
            "guild_id": guild_id.get(),
            "channel_id": channel_id.map(|id| id.get()),
            "self_mute": true,
            "self_deaf": true,
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::voice_state_payload;
    use poise::serenity_prelude::{ChannelId, GuildId};

    #[test]
    fn voice_payload_joins_and_leaves() {
        let join = voice_state_payload(GuildId::new(1), Some(ChannelId::new(2)));
        let leave = voice_state_payload(GuildId::new(1), None);

        assert!(join.contains(r#""channel_id":2"#));
        assert!(leave.contains(r#""channel_id":null"#));
    }
}
