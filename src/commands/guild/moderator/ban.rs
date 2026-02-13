use crate::i18n::{get_guild_language, Language};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

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
        Language::Vietnamese => format!("Đã ban **{}**\nLý do: ```{}```", member_name, reason),
        Language::Japanese => format!("**{}**をBANしました\n理由：```{}```", member_name, reason),
        _ => format!("Banned **{}**\nReason: ```{}```", member_name, reason),
    };

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0xe74c3c); // Red

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
