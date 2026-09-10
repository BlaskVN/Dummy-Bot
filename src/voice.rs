use crate::VoiceConnectionInfo;
use poise::serenity_prelude as serenity;
use std::collections::HashMap;
use tokio::sync::RwLock;

pub async fn disconnect_voice(
    ctx: &serenity::Context,
    connections: &RwLock<HashMap<serenity::GuildId, VoiceConnectionInfo>>,
    guild_id: serenity::GuildId,
) -> bool {
    let existed = {
        let mut map = connections.write().await;
        map.remove(&guild_id).is_some()
    };
    update_voice_state(ctx, guild_id, None);
    existed
}

/// Send Discord gateway voice state update to connect or disconnect from a voice channel.
pub fn update_voice_state(
    ctx: &serenity::Context,
    guild_id: serenity::GuildId,
    channel_id: Option<serenity::ChannelId>,
) {
    ctx.shard
        .websocket_message(voice_state_payload(guild_id, channel_id).into());
}

pub fn voice_state_payload(
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
