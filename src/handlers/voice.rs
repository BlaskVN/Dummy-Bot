use crate::Data;
use crate::i18n::{TranslationKey, t};
use poise::serenity_prelude as serenity;

/// Stop tracking an unexpected voice disconnect. Rejoining requires a new
/// authorized `/connect`, so a server member can always remove the bot.
pub async fn handle_voice_state_update(
    ctx: &serenity::Context,
    old: &Option<serenity::VoiceState>,
    new: &serenity::VoiceState,
    data: &Data,
) {
    if new.user_id != ctx.cache.current_user().id
        || !was_disconnected(
            old.as_ref().and_then(|state| state.channel_id),
            new.channel_id,
        )
    {
        return;
    }

    let Some(guild_id) = new.guild_id else {
        return;
    };
    let Some(connection) = data.voice_connections.write().await.remove(&guild_id) else {
        return;
    };

    let lang = data.language(guild_id).await;
    let builder = serenity::CreateMessage::new()
        .content(t(lang, TranslationKey::VoiceKicked))
        .allowed_mentions(serenity::CreateAllowedMentions::new());
    if let Err(error) = connection
        .text_channel_id
        .send_message(&ctx.http, builder)
        .await
    {
        tracing::warn!(%error, "Failed to report voice disconnect");
    }
}

fn was_disconnected(
    old_channel: Option<serenity::ChannelId>,
    new_channel: Option<serenity::ChannelId>,
) -> bool {
    old_channel.is_some() && new_channel.is_none()
}

#[cfg(test)]
mod tests {
    use super::was_disconnected;
    use poise::serenity_prelude::ChannelId;

    #[test]
    fn only_leaving_voice_counts_as_disconnect() {
        let channel = Some(ChannelId::new(1));
        assert!(was_disconnected(channel, None));
        assert!(!was_disconnected(channel, channel));
        assert!(!was_disconnected(None, None));
    }
}
