use crate::i18n::{get_guild_language, t, tf, TranslationKey};
use crate::{Data, VoiceConnectionInfo};
use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Maximum number of reconnection attempts before giving up.
const MAX_RECONNECT_ATTEMPTS: u32 = 3;

/// Delays (in seconds) for each reconnection attempt (exponential backoff).
const RECONNECT_DELAYS_SECS: [u64; 3] = [5, 10, 20];

/// Handle voice state updates to detect when the bot is disconnected from a voice channel.
///
/// When the bot's voice state changes to `channel_id = None` and we still have
/// an entry in `voice_connections` (meaning we didn't initiate the disconnect),
/// it means either:
/// 1. Someone kicked/disconnected the bot, or
/// 2. The bot was disconnected due to a network drop.
///
/// In both cases, we attempt to automatically reconnect with exponential backoff.
/// If all reconnection attempts fail, we notify the text channel.
pub async fn handle_voice_state_update(
    ctx: &serenity::Context,
    old: &Option<serenity::VoiceState>,
    new: &serenity::VoiceState,
    data: &Data,
) {
    // Only care about the bot's own voice state changes
    let bot_id = ctx.cache.current_user().id;
    if new.user_id != bot_id {
        return;
    }

    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return,
    };

    // Check if bot was removed from voice (channel_id is now None)
    // and previously was in a channel (old state had a channel_id)
    let was_in_channel = old
        .as_ref()
        .and_then(|s| s.channel_id)
        .is_some();

    if new.channel_id.is_some() || !was_in_channel {
        // Bot is still in a channel or wasn't in one before — not a disconnect
        return;
    }

    // Bot was removed from voice. Check if we have tracking info.
    // Remove the entry so that:
    // 1. /disconnect won't find it (preventing false state)
    // 2. Another VoiceStateUpdate won't spawn a duplicate reconnection task
    let connection_info = {
        let mut map = data.voice_connections.write().await;
        map.remove(&guild_id)
    };

    let connection_info = match connection_info {
        Some(info) => info,
        None => return, // /disconnect was used — no reconnection needed
    };

    // Clean up existing songbird state before reconnection
    if let Some(manager) = songbird::get(ctx).await {
        let _ = manager.remove(guild_id).await;
    }

    tracing::warn!(
        guild = %guild_id,
        voice_channel = %connection_info.voice_channel_id,
        text_channel = %connection_info.text_channel_id,
        "Bot disconnected from voice, spawning auto-reconnection task"
    );

    // Spawn a background task for reconnection (so we don't block the event handler)
    let ctx = ctx.clone();
    let db_pool = data.db_pool.clone();
    let voice_connections = Arc::clone(&data.voice_connections);

    tokio::spawn(async move {
        attempt_voice_reconnect(
            &ctx,
            guild_id,
            &connection_info,
            &voice_connections,
            &db_pool,
        )
        .await;
    });
}

/// Attempts to reconnect to a voice channel with exponential backoff.
///
/// - Sends a "reconnecting..." notification to the text channel.
/// - Retries up to `MAX_RECONNECT_ATTEMPTS` times with increasing delays.
/// - On success: re-adds the connection info and notifies the text channel.
/// - On failure: notifies the text channel that manual `/connect` is needed.
async fn attempt_voice_reconnect(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    connection_info: &VoiceConnectionInfo,
    voice_connections: &Arc<RwLock<HashMap<serenity::GuildId, VoiceConnectionInfo>>>,
    db_pool: &sqlx::SqlitePool,
) {
    let lang = get_guild_language(db_pool, guild_id).await;

    // Notify text channel that reconnection is being attempted
    let embed = serenity::CreateEmbed::new()
        .description(tf(
            lang,
            TranslationKey::VoiceReconnecting,
            &[&connection_info.voice_channel_id],
        ))
        .color(0xe67e22); // Orange — warning/in-progress

    let builder = serenity::CreateMessage::new().embed(embed);
    if let Err(e) = connection_info
        .text_channel_id
        .send_message(&ctx.http, builder)
        .await
    {
        tracing::error!("Failed to send reconnection notification: {}", e);
    }

    let manager = match songbird::get(ctx).await {
        Some(m) => m,
        None => {
            tracing::error!("Songbird not initialized during reconnection attempt");
            return;
        }
    };

    for attempt in 0..MAX_RECONNECT_ATTEMPTS {
        let delay_secs = RECONNECT_DELAYS_SECS
            .get(attempt as usize)
            .copied()
            .unwrap_or(20);
        let delay = Duration::from_secs(delay_secs);

        tracing::info!(
            guild = %guild_id,
            attempt = attempt + 1,
            max_attempts = MAX_RECONNECT_ATTEMPTS,
            delay_secs = delay_secs,
            "Waiting before reconnection attempt"
        );

        tokio::time::sleep(delay).await;

        // Check if someone manually called /connect or /disconnect during our wait.
        // If voice_connections now has an entry for this guild, another /connect was used.
        {
            let map = voice_connections.read().await;
            if map.contains_key(&guild_id) {
                tracing::info!(
                    guild = %guild_id,
                    "Voice connection re-established by user during reconnection, aborting auto-reconnect"
                );
                return;
            }
        }

        // Clean up any stale songbird state before attempting rejoin
        let _ = manager.remove(guild_id).await;

        tracing::info!(
            guild = %guild_id,
            attempt = attempt + 1,
            voice_channel = %connection_info.voice_channel_id,
            "Attempting voice reconnection"
        );

        match manager.join(guild_id, connection_info.voice_channel_id).await {
            Ok(_call) => {
                // Reconnection succeeded — re-add the tracking entry
                {
                    let mut map = voice_connections.write().await;
                    map.insert(guild_id, connection_info.clone());
                }

                tracing::info!(
                    guild = %guild_id,
                    voice_channel = %connection_info.voice_channel_id,
                    attempt = attempt + 1,
                    "Successfully reconnected to voice channel"
                );

                // Notify text channel about successful reconnection
                let embed = serenity::CreateEmbed::new()
                    .description(tf(
                        lang,
                        TranslationKey::VoiceReconnected,
                        &[&connection_info.voice_channel_id],
                    ))
                    .color(0x2ecc71); // Green — success

                let builder = serenity::CreateMessage::new().embed(embed);
                if let Err(e) = connection_info
                    .text_channel_id
                    .send_message(&ctx.http, builder)
                    .await
                {
                    tracing::error!("Failed to send reconnection success notification: {}", e);
                }

                return; // Done — reconnection successful
            }
            Err(e) => {
                tracing::warn!(
                    guild = %guild_id,
                    attempt = attempt + 1,
                    error = %e,
                    "Voice reconnection attempt failed"
                );
            }
        }
    }

    // All reconnection attempts exhausted
    tracing::error!(
        guild = %guild_id,
        voice_channel = %connection_info.voice_channel_id,
        "All {} voice reconnection attempts failed",
        MAX_RECONNECT_ATTEMPTS
    );

    // Clean up songbird state one final time
    let _ = manager.remove(guild_id).await;

    // Notify text channel about failure
    let embed = serenity::CreateEmbed::new()
        .description(t(lang, TranslationKey::VoiceReconnectFailed))
        .color(0xe74c3c); // Red — error

    let builder = serenity::CreateMessage::new().embed(embed);
    if let Err(e) = connection_info
        .text_channel_id
        .send_message(&ctx.http, builder)
        .await
    {
        tracing::error!("Failed to send reconnection failure notification: {}", e);
    }
}
