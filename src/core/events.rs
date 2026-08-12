use crate::core::engine::RhaiManager;
use poise::serenity_prelude as serenity;
use std::sync::Arc;

pub struct CoreEventBus {
    rhai_manager: Arc<RhaiManager>,
}

impl CoreEventBus {
    pub fn new(rhai_manager: Arc<RhaiManager>) -> Self {
        Self { rhai_manager }
    }

    pub async fn dispatch_message(&self, message: &serenity::Message) {
        if message.author.bot {
            return;
        }

        let guild_id = message
            .guild_id
            .map(|g| g.get().to_string())
            .unwrap_or_default();
        let author_id = message.author.id.get().to_string();
        let content = message.content.clone();

        // Call inspect_message in automod module
        if let Ok(Some(result)) = self
            .rhai_manager
            .call_fn::<rhai::Map>("automod", "inspect_message", (content, author_id, guild_id))
            .await
        {
            tracing::debug!(?result, "AutoMod Rhai module inspection result");
        }
    }

    pub async fn dispatch_voice_state_update(
        &self,
        old: Option<&serenity::VoiceState>,
        new: &serenity::VoiceState,
    ) {
        let user_id = new.user_id.get().to_string();
        let guild_id = new
            .guild_id
            .map(|g| g.get().to_string())
            .unwrap_or_default();

        let old_channel = old.and_then(|v| v.channel_id).map(|c| c.get().to_string());
        let new_channel = new.channel_id.map(|c| c.get().to_string());

        tracing::trace!(
            user_id = %user_id,
            guild_id = %guild_id,
            ?old_channel,
            ?new_channel,
            "Dispatched voice state update to Rhai core bus"
        );
    }
}
