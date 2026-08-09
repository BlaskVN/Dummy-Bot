use crate::Data;
use poise::serenity_prelude as serenity;
use std::collections::HashSet;

/// Presence absence is not a negative signal: members may hide Activity Sharing.
pub async fn handle_presence_update(
    ctx: &serenity::Context,
    presence: &serenity::Presence,
    data: &Data,
) {
    let Some(guild_id) = presence.guild_id else {
        return;
    };
    let Ok(Some((config, pool))) = crate::game_config::game_config(&data.db_pool, guild_id).await
    else {
        return;
    };
    let user_id = presence.user.id;
    let (channel_id, is_bot) = ctx.cache.guild(guild_id).map_or((None, true), |guild| {
        (
            guild
                .voice_states
                .get(&user_id)
                .and_then(|state| state.channel_id),
            guild
                .members
                .get(&user_id)
                .is_none_or(|member| member.user.bot),
        )
    });
    let matched = !is_bot
        && presence.activities.iter().any(|activity| {
            activity_matches(
                activity.kind,
                activity.application_id.map(|id| id.get()),
                &activity.name,
                config
                    .activity_application_id
                    .as_deref()
                    .and_then(|id| id.parse().ok()),
                &config.activity_name,
            )
        });
    set_automatic_source(
        data,
        guild_id,
        user_id,
        channel_id.filter(|channel| pool.contains(channel) && matched),
    )
    .await;
}

pub async fn handle_voice_change(
    ctx: &serenity::Context,
    voice: &serenity::VoiceState,
    data: &Data,
) {
    let Some(guild_id) = voice.guild_id else {
        return;
    };
    let Ok(Some((config, pool))) = crate::game_config::game_config(&data.db_pool, guild_id).await
    else {
        remove_member(data, guild_id, voice.user_id).await;
        return;
    };
    let (presence, is_bot) = ctx.cache.guild(guild_id).map_or((None, true), |guild| {
        (
            guild.presences.get(&voice.user_id).cloned(),
            guild
                .members
                .get(&voice.user_id)
                .is_none_or(|member| member.user.bot),
        )
    });
    let matched = !is_bot
        && presence.is_some_and(|presence| {
            presence.activities.iter().any(|activity| {
                activity_matches(
                    activity.kind,
                    activity.application_id.map(|id| id.get()),
                    &activity.name,
                    config
                        .activity_application_id
                        .as_deref()
                        .and_then(|id| id.parse().ok()),
                    &config.activity_name,
                )
            })
        });
    set_automatic_source(
        data,
        guild_id,
        voice.user_id,
        voice
            .channel_id
            .filter(|channel| pool.contains(channel) && matched),
    )
    .await;
}

pub async fn remove_member(data: &Data, guild_id: serenity::GuildId, user_id: serenity::UserId) {
    let mut beacons = data.automatic_beacons.write().await;
    update_source(&mut beacons, guild_id, user_id, None);
}

pub async fn beacon_active(
    data: &Data,
    guild_id: serenity::GuildId,
    channel_id: serenity::ChannelId,
) -> bool {
    data.automatic_beacons
        .read()
        .await
        .iter()
        .any(|(guild, channel, _)| *guild == guild_id && *channel == channel_id)
}

async fn set_automatic_source(
    data: &Data,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    channel_id: Option<serenity::ChannelId>,
) {
    let mut beacons = data.automatic_beacons.write().await;
    update_source(&mut beacons, guild_id, user_id, channel_id);
}

fn update_source(
    beacons: &mut HashSet<(serenity::GuildId, serenity::ChannelId, serenity::UserId)>,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    channel_id: Option<serenity::ChannelId>,
) {
    beacons.retain(|(guild, _, user)| *guild != guild_id || *user != user_id);
    if let Some(channel_id) = channel_id {
        beacons.insert((guild_id, channel_id, user_id));
    }
}

fn activity_matches(
    kind: serenity::ActivityType,
    application_id: Option<u64>,
    name: &str,
    configured_application_id: Option<u64>,
    configured_name: &str,
) -> bool {
    kind == serenity::ActivityType::Playing
        && (configured_application_id.is_some_and(|id| application_id == Some(id))
            || name.eq_ignore_ascii_case(configured_name))
}

pub fn detection_status(enabled: bool) -> &'static str {
    if enabled { "available" } else { "degraded" }
}

#[cfg(test)]
mod tests {
    use super::{activity_matches, detection_status, update_source};
    use poise::serenity_prelude::{ActivityType, ChannelId, GuildId, UserId};
    use std::collections::HashSet;

    #[test]
    fn renders_detection_availability() {
        assert_eq!(detection_status(true), "available");
        assert_eq!(detection_status(false), "degraded");
    }

    #[test]
    fn matches_playing_by_id_then_exact_ascii_name() {
        assert!(activity_matches(
            ActivityType::Playing,
            Some(7),
            "Other",
            Some(7),
            "Minecraft"
        ));
        assert!(activity_matches(
            ActivityType::Playing,
            Some(8),
            "MINECRAFT",
            Some(7),
            "Minecraft"
        ));
        assert!(activity_matches(
            ActivityType::Playing,
            None,
            "minecraft",
            None,
            "Minecraft"
        ));
        assert!(!activity_matches(
            ActivityType::Streaming,
            Some(7),
            "Minecraft",
            Some(7),
            "Minecraft"
        ));
        assert!(!activity_matches(
            ActivityType::Playing,
            None,
            "Minecraft Java",
            None,
            "Minecraft"
        ));
        assert!(!activity_matches(
            ActivityType::Playing,
            None,
            "MİNECRAFT",
            None,
            "Minecraft"
        ));
    }

    #[test]
    fn keeps_beacons_channel_local_across_moves_and_leaves() {
        let guild = GuildId::new(1);
        let first = ChannelId::new(10);
        let second = ChannelId::new(11);
        let mut sources = HashSet::new();
        update_source(&mut sources, guild, UserId::new(20), Some(first));
        update_source(&mut sources, guild, UserId::new(21), Some(first));
        update_source(&mut sources, guild, UserId::new(20), Some(second));
        assert!(sources.contains(&(guild, first, UserId::new(21))));
        assert!(sources.contains(&(guild, second, UserId::new(20))));
        update_source(&mut sources, guild, UserId::new(21), None);
        assert!(!sources.iter().any(|(_, channel, _)| *channel == first));
        assert!(sources.iter().any(|(_, channel, _)| *channel == second));
    }
}
