use crate::commands::presence::restore_presence;
use crate::commands::voice::update_voice_state;
use crate::{Data, VoiceConnectionInfo};
use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Restore bot state after a gateway Resume.
///
/// Discord may drop Rich Presence and voice connections during outages.
/// Re-applies the saved presence and checks whether any tracked voice
/// channels need to be rejoined.
pub async fn handle_resume(ctx: &serenity::Context, data: &Data) {
    tracing::info!("Gateway resumed — restoring bot state");
    restore_presence(ctx, &data.db_pool).await;
    if let Err(error) = crate::attendance::clear_stale_active_starts(&data.db_pool).await {
        tracing::error!(%error, "Could not clear stale attendance starts after resume");
    }
    if let Err(error) =
        crate::activity_aggregate::finalize_pending(&data.db_pool, chrono::Utc::now().timestamp())
            .await
    {
        tracing::error!(%error, "Could not finalize pending activity aggregates after resume");
    }
    crate::commands::word_puzzle::reconcile_and_deliver_all(ctx, data).await;
    super::activity_presence::reconcile_known_channels(ctx, data).await;
    super::community::reconcile_all(ctx, data).await;
    super::rewards::reconcile_all(ctx, data).await;
    super::game_session::wake_expiry(data);
    spawn_voice_reconnect(
        ctx.clone(),
        data.voice_connections.clone(),
        data.config.gateway_resume_delay_seconds,
    );
}

/// Restore bot state after a shard restart (subsequent Ready event).
///
/// A shard restart means a full new gateway Identify — presence is definitely
/// reset and all voice connections are lost.
pub async fn handle_ready_reconnect(ctx: &serenity::Context, data: &Data) {
    tracing::info!("Shard restarted (new Ready) — restoring bot state");
    restore_presence(ctx, &data.db_pool).await;
    if let Err(error) = crate::attendance::clear_stale_active_starts(&data.db_pool).await {
        tracing::error!(%error, "Could not clear stale attendance starts after ready");
    }
    if let Err(error) =
        crate::activity_aggregate::finalize_pending(&data.db_pool, chrono::Utc::now().timestamp())
            .await
    {
        tracing::error!(%error, "Could not finalize pending activity aggregates after ready");
    }
    crate::commands::word_puzzle::reconcile_and_deliver_all(ctx, data).await;
    super::activity_presence::reconcile_known_channels(ctx, data).await;
    super::community::reconcile_all(ctx, data).await;
    super::rewards::reconcile_all(ctx, data).await;
    super::game_session::wake_expiry(data);
    spawn_voice_reconnect(
        ctx.clone(),
        data.voice_connections.clone(),
        data.config.gateway_ready_delay_seconds,
    );
}

/// Spawn a background task that waits `delay_secs`, then attempts to rejoin
/// any voice channels the bot was tracking but is no longer connected to.
fn spawn_voice_reconnect(
    ctx: serenity::Context,
    voice_connections: Arc<RwLock<HashMap<serenity::GuildId, VoiceConnectionInfo>>>,
    delay_secs: u64,
) {
    tokio::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
        reconnect_stale_voice(&ctx, &voice_connections).await;
    });
}

/// Walk every tracked voice connection and rejoin if the bot is no longer
/// present in the expected channel (e.g. after a network drop or shard restart).
async fn reconnect_stale_voice(
    ctx: &serenity::Context,
    voice_connections: &Arc<RwLock<HashMap<serenity::GuildId, VoiceConnectionInfo>>>,
) {
    let snapshot: Vec<_> = {
        let map = voice_connections.read().await;
        map.iter().map(|(g, i)| (*g, i.clone())).collect()
    };

    if snapshot.is_empty() {
        return;
    }

    let bot_id = ctx.cache.current_user().id;

    for (guild_id, info) in snapshot {
        if guild_id.shard_id(&ctx.cache) != ctx.shard_id.get() {
            continue;
        }

        // The VoiceStateUpdate handler may have already removed this entry
        // while we were waiting — re-check before acting.
        {
            let map = voice_connections.read().await;
            if !map.contains_key(&guild_id) {
                continue;
            }
        }

        let bot_channel = ctx
            .cache
            .guild(guild_id)
            .and_then(|guild| guild.voice_states.get(&bot_id).and_then(|vs| vs.channel_id));

        if bot_channel == Some(info.voice_channel_id) {
            tracing::debug!(
                guild = %guild_id,
                "Bot still in voice channel after reconnect, skipping"
            );
            continue;
        }

        tracing::info!(
            guild = %guild_id,
            voice_channel = %info.voice_channel_id,
            "Rejoining voice channel after reconnection"
        );

        update_voice_state(ctx, guild_id, Some(info.voice_channel_id));
    }
}
