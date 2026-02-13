use crate::i18n::{get_guild_language, t, tf, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Join the voice channel you are currently in.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "connect"
)]
pub async fn voice_connect(
    ctx: Context<'_>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

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
                .color(0xe74c3c);
            ctx.send(poise::CreateReply::default().embed(embed))
                .await?;
            return Ok(());
        }
    };

    // Get songbird manager
    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or_else(|| anyhow::anyhow!("Songbird not initialized"))?;

    // Check if already connected to a voice channel in this guild
    if manager.get(guild_id).is_some() {
        let embed = serenity::CreateEmbed::new()
            .description(t(lang, TranslationKey::VoiceAlreadyConnected))
            .color(0xe67e22);
        ctx.send(poise::CreateReply::default().embed(embed))
            .await?;
        return Ok(());
    }

    // Join the voice channel
    match manager.join(guild_id, voice_channel_id).await {
        Ok(_call) => {}
        Err(e) => {
            tracing::error!(
                guild = %guild_id,
                channel = %voice_channel_id,
                error = %e,
                "Failed to join voice channel"
            );
            let embed = serenity::CreateEmbed::new()
                .description(t(lang, TranslationKey::VoiceJoinFailed))
                .color(0xe74c3c);
            ctx.send(poise::CreateReply::default().embed(embed))
                .await?;
            return Ok(());
        }
    }

    // Store the text channel where connect was used (for kick notification)
    {
        let mut map = ctx.data().voice_text_channels.write().await;
        map.insert(guild_id, ctx.channel_id());
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
        .color(0x2ecc71);

    ctx.send(poise::CreateReply::default().embed(embed))
        .await?;

    Ok(())
}

/// Leave the current voice channel.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    rename = "disconnect"
)]
pub async fn voice_disconnect(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .ok_or_else(|| anyhow::anyhow!("Songbird not initialized"))?;

    // Check if bot is in a voice channel
    if manager.get(guild_id).is_none() {
        let embed = serenity::CreateEmbed::new()
            .description(t(lang, TranslationKey::VoiceNotConnected))
            .color(0xe74c3c);
        ctx.send(poise::CreateReply::default().embed(embed))
            .await?;
        return Ok(());
    }

    // Remove from tracking FIRST (before remove) to prevent false kick notification
    {
        let mut map = ctx.data().voice_text_channels.write().await;
        map.remove(&guild_id);
    }

    // Leave and fully clean up the voice handler (drop tasks, threads, memory)
    if let Err(e) = manager.remove(guild_id).await {
        tracing::error!(
            guild = %guild_id,
            error = %e,
            "Failed to remove voice handler"
        );
    }

    tracing::info!(
        user = %ctx.author().name,
        guild = %guild_id,
        "Bot left voice channel"
    );

    let embed = serenity::CreateEmbed::new()
        .description(t(lang, TranslationKey::VoiceDisconnected))
        .color(0x2ecc71);

    ctx.send(poise::CreateReply::default().embed(embed))
        .await?;

    Ok(())
}
