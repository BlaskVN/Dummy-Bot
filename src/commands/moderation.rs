use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Kick a member from the server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS"
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "Thành viên cần kick"] member: serenity::Member,
    #[description = "Lý do kick"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "Không có lý do".to_string());
    let member_name = member.user.name.clone();

    member.kick_with_reason(&ctx.http(), &reason).await?;

    tracing::info!(
        moderator = %ctx.author().name,
        target = %member_name,
        reason = %reason,
        "Member kicked"
    );

    ctx.say(format!(
        "👢 Đã kick **{}** — Lý do: {}",
        member_name, reason
    ))
    .await?;

    Ok(())
}

/// Ban a member from the server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "BAN_MEMBERS",
    required_bot_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "Thành viên cần ban"] member: serenity::Member,
    #[description = "Số ngày xóa tin nhắn (0-7)"] delete_days: Option<u8>,
    #[description = "Lý do ban"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let reason = reason.unwrap_or_else(|| "Không có lý do".to_string());
    let delete_days = delete_days.unwrap_or(0).min(7);
    let member_name = member.user.name.clone();

    member
        .ban_with_reason(&ctx.http(), delete_days, &reason)
        .await?;

    tracing::info!(
        moderator = %ctx.author().name,
        target = %member_name,
        reason = %reason,
        delete_days = delete_days,
        "Member banned"
    );

    ctx.say(format!(
        "🔨 Đã ban **{}** — Lý do: {}",
        member_name, reason
    ))
    .await?;

    Ok(())
}

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
    #[description = "Số tin nhắn cần xóa (1-100)"]
    #[min = 1]
    #[max = 100]
    amount: u8,
) -> Result<(), Error> {
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

    let reply = ctx
        .say(format!("🗑️ Đã xóa **{}** tin nhắn.", count))
        .await?;

    // Auto-delete the confirmation after 3 seconds
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let _ = reply.delete(ctx).await;

    Ok(())
}
