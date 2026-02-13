use crate::i18n::{get_guild_language, t, TranslationKey};
use crate::Data;
use poise::serenity_prelude as serenity;

/// Handle voice state updates to detect when the bot is kicked from a voice channel.
///
/// When the bot's voice state changes to `channel_id = None` and we still have
/// an entry in `voice_text_channels` (meaning we didn't initiate the disconnect),
/// it means someone kicked/disconnected the bot. We notify the text channel
/// where `/connect` was originally used.
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
        // Bot is still in a channel or wasn't in one before — not a kick
        return;
    }

    // Bot was removed from voice. Check if we have a tracked text channel.
    // If so, this was NOT initiated by /disconnect (which removes the entry first).
    let text_channel = {
        let mut map = data.voice_text_channels.write().await;
        map.remove(&guild_id)
    };

    let text_channel_id = match text_channel {
        Some(id) => id,
        None => return, // Disconnect was initiated by /disconnect command — no notification needed
    };

    // Fully clean up songbird state (drop handler, tasks, threads, memory)
    if let Some(manager) = songbird::get(ctx).await {
        let _ = manager.remove(guild_id).await;
    }

    let lang = get_guild_language(&data.db_pool, guild_id).await;

    tracing::info!(
        guild = %guild_id,
        text_channel = %text_channel_id,
        "Bot was kicked from voice channel, notifying"
    );

    let embed = serenity::CreateEmbed::new()
        .description(t(lang, TranslationKey::VoiceKicked))
        .color(0xe74c3c);

    let builder = serenity::CreateMessage::new().embed(embed);

    if let Err(e) = text_channel_id.send_message(&ctx.http, builder).await {
        tracing::error!("Failed to send voice kick notification: {}", e);
    }
}
