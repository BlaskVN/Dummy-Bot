use crate::i18n::{get_guild_language, t, tf, TranslationKey};
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

    let reason = reason.unwrap_or_else(|| t(lang, TranslationKey::ModerationNoReason).to_string());
    let member_name = member.user.name.clone();

    member.kick_with_reason(&ctx.http(), &reason).await?;

    tracing::info!(
        moderator = %ctx.author().name,
        target = %member_name,
        reason = %reason,
        "Member kicked"
    );

    let message = tf(lang, TranslationKey::ModerationKicked, &[&member_name, &reason]);

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0xe74c3c); // Red

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
