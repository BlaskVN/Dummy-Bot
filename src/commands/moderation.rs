use crate::i18n::{get_guild_language, Language};
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
    #[description = "Member to kick"] member: serenity::Member,
    #[description = "Reason for kick"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    let reason = reason.unwrap_or_else(|| match lang {
        Language::Vietnamese => "Không có lý do".to_string(),
        Language::Japanese => "理由なし".to_string(),
        _ => "No reason provided".to_string(),
    });
    let member_name = member.user.name.clone();

    member.kick_with_reason(&ctx.http(), &reason).await?;

    tracing::info!(
        moderator = %ctx.author().name,
        target = %member_name,
        reason = %reason,
        "Member kicked"
    );

    let message = match lang {
        Language::Vietnamese => format!("Đã kick **{}** — Lý do: {}", member_name, reason),
        Language::Japanese => format!("**{}**をキックしました — 理由：{}", member_name, reason),
        _ => format!("Kicked **{}** — Reason: {}", member_name, reason),
    };

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0xe74c3c); // Red

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

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
    #[description = "Member to ban"] member: serenity::Member,
    #[description = "Days of messages to delete (0-7)"] delete_days: Option<u8>,
    #[description = "Reason for ban"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    let reason = reason.unwrap_or_else(|| match lang {
        Language::Vietnamese => "Không có lý do".to_string(),
        Language::Japanese => "理由なし".to_string(),
        _ => "No reason provided".to_string(),
    });
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

    let message = match lang {
        Language::Vietnamese => format!("Đã ban **{}** — Lý do: {}", member_name, reason),
        Language::Japanese => format!("**{}**をBANしました — 理由：{}", member_name, reason),
        _ => format!("Banned **{}** — Reason: {}", member_name, reason),
    };

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0xe74c3c); // Red

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

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

    let message = match lang {
        Language::Vietnamese => format!("Đã xóa **{}** tin nhắn.", count),
        Language::Japanese => format!("**{}**件のメッセージを削除しました。", count),
        _ => format!("Deleted **{}** messages.", count),
    };

    let reply = ctx.say(message).await?;

    // Auto-delete the confirmation after 3 seconds
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let _ = reply.delete(ctx).await;

    Ok(())
}
