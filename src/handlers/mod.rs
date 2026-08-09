pub mod activity_presence;
pub mod automod;
pub mod community;
pub mod game_session;
pub mod guild_lifecycle;
pub mod message_log;
pub mod onboarding;
pub mod reconnect;
pub mod voice;

use crate::{Data, Error};
use poise::serenity_prelude as serenity;

pub async fn dispatch(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::PresenceUpdate { new_data } => {
            activity_presence::handle_presence_update(ctx, new_data, data).await;
        }
        serenity::FullEvent::Message { new_message } => {
            game_session::handle_message(ctx, new_message, data).await?;
        }
        serenity::FullEvent::MessageDelete {
            channel_id,
            deleted_message_id,
            guild_id,
        } => {
            message_log::handle_message_delete(
                ctx,
                *channel_id,
                *deleted_message_id,
                *guild_id,
                data,
            )
            .await;
        }
        serenity::FullEvent::MessageDeleteBulk {
            channel_id,
            multiple_deleted_messages_ids,
            guild_id,
        } => {
            message_log::handle_message_delete_bulk(
                ctx,
                *channel_id,
                multiple_deleted_messages_ids,
                *guild_id,
                data,
            )
            .await;
        }
        serenity::FullEvent::MessageUpdate {
            old_if_available,
            event,
            ..
        } => {
            message_log::handle_message_update(ctx, old_if_available.as_ref(), event, data).await;
        }
        serenity::FullEvent::VoiceStateUpdate { old, new, .. } => {
            voice::handle_voice_state_update(ctx, old, new, data).await;
        }
        serenity::FullEvent::GuildCreate { guild, is_new } => {
            onboarding::handle_guild_create(ctx, guild, *is_new, data).await;
        }
        serenity::FullEvent::GuildDelete { incomplete, .. } => {
            guild_lifecycle::handle_guild_delete(incomplete, data).await;
        }
        serenity::FullEvent::AutoModActionExecution { execution } => {
            automod::handle_execution(ctx, execution, data).await;
        }
        serenity::FullEvent::AutoModRuleUpdate { rule } => {
            automod::handle_rule_update(rule, data).await;
        }
        serenity::FullEvent::GuildScheduledEventCreate { event }
        | serenity::FullEvent::GuildScheduledEventUpdate { event } => {
            community::handle_native_update(data, event).await;
        }
        serenity::FullEvent::GuildScheduledEventDelete { event } => {
            community::handle_native_delete(ctx, data, event).await;
        }
        serenity::FullEvent::InteractionCreate {
            interaction: serenity::Interaction::Component(interaction),
        } => community::handle_component(ctx, interaction, data).await?,
        serenity::FullEvent::Resume { .. } => reconnect::handle_resume(ctx, data).await,
        serenity::FullEvent::Ready { .. } => reconnect::handle_ready_reconnect(ctx, data).await,
        _ => {}
    }
    Ok(())
}
