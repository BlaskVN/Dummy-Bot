pub mod message_log;
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
        serenity::FullEvent::MessageUpdate { event, .. } => {
            message_log::handle_message_update(ctx, event, data).await;
        }
        serenity::FullEvent::VoiceStateUpdate { old, new, .. } => {
            voice::handle_voice_state_update(ctx, old, new, data).await;
        }
        serenity::FullEvent::Resume { .. } => reconnect::handle_resume(ctx, data).await,
        serenity::FullEvent::Ready { .. } => reconnect::handle_ready_reconnect(ctx, data).await,
        _ => {}
    }
    Ok(())
}
