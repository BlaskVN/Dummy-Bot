use crate::handlers::message_log;
use crate::i18n::{TranslationKey, tf};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Bulk delete messages in the current channel.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_MESSAGES",
    required_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "VIEW_CHANNEL | MANAGE_MESSAGES | READ_MESSAGE_HISTORY"
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Number of messages to delete (1-100)"]
    #[min = 1]
    #[max = 100]
    amount: u8,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    if amount > ctx.data().config.purge_max_messages {
        let message = tf(
            lang,
            TranslationKey::ModerationPurgeRange,
            &[&ctx.data().config.purge_max_messages],
        );
        ctx.say(message).await?;
        return Ok(());
    }

    let channel = ctx.channel_id();
    let _typing = ctx.defer_or_broadcast().await?;

    // Fetch messages to delete
    let messages = channel
        .messages(&ctx.http(), serenity::GetMessages::new().limit(amount))
        .await?;

    let count = messages.len();
    let message_ids: Vec<serenity::MessageId> = messages.iter().map(|m| m.id).collect();

    message_log::archive_purge_attachments(ctx.serenity_context(), guild_id, &messages, ctx.data())
        .await;

    // Bulk delete (only works for messages < 14 days old)
    channel.delete_messages(&ctx.http(), message_ids).await?;

    tracing::info!(
        moderator = %ctx.author().name,
        channel = %channel,
        count = count,
        "Messages purged"
    );

    let message = tf(lang, TranslationKey::ModerationPurged, &[&count]);

    let reply = ctx.say(message).await?;

    tokio::time::sleep(std::time::Duration::from_secs(
        ctx.data().config.purge_confirmation_seconds,
    ))
    .await;
    let _ = reply.delete(ctx).await;

    Ok(())
}
