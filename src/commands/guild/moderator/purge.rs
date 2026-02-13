use crate::i18n::{get_guild_language, tf, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Bulk delete messages in the current channel.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MANAGE_MESSAGES",
    required_bot_permissions = "MANAGE_MESSAGES"
)]
pub async fn purge(
    ctx: Context<'_>,
    #[description = "Number of messages to delete (1-100)"]
    #[min = 1]
    #[max = 100]
    amount: u8,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    let channel = ctx.channel_id();

    // Fetch messages to delete
    let messages = channel
        .messages(&ctx.http(), serenity::GetMessages::new().limit(amount))
        .await?;

    let count = messages.len();
    let message_ids: Vec<serenity::MessageId> = messages.iter().map(|m| m.id).collect();

    // Bulk delete (only works for messages < 14 days old)
    channel
        .delete_messages(&ctx.http(), message_ids)
        .await?;

    tracing::info!(
        moderator = %ctx.author().name,
        channel = %channel,
        count = count,
        "Messages purged"
    );

    let message = tf(lang, TranslationKey::ModerationPurged, &[&count]);

    let reply = ctx.say(message).await?;

    // Auto-delete the confirmation after 3 seconds
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let _ = reply.delete(ctx).await;

    Ok(())
}
