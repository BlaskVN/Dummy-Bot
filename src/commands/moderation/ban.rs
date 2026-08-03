use crate::i18n::{TranslationKey, t, tf};
use crate::permissions::{ModerationDenial, moderation_denial};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Ban a member from the server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    default_member_permissions = "BAN_MEMBERS",
    required_permissions = "BAN_MEMBERS",
    required_bot_permissions = "BAN_MEMBERS"
)]
pub async fn ban(
    ctx: Context<'_>,
    #[description = "Member to ban"] member: serenity::Member,
    #[description = "Days of messages to delete"]
    #[max = 7]
    delete_days: Option<u8>,
    #[description = "Reason for ban"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    if let Some(denial) = moderation_denial(ctx, member.user.id)? {
        let key = match denial {
            ModerationDenial::SelfTarget => TranslationKey::ModerationCannotTargetSelf,
            ModerationDenial::UserHierarchy => TranslationKey::ModerationUserHierarchy,
            ModerationDenial::BotHierarchy => TranslationKey::ModerationBotHierarchy,
        };
        ctx.say(t(lang, key)).await?;
        return Ok(());
    }

    let reason = reason.unwrap_or_else(|| t(lang, TranslationKey::ModerationNoReason).to_string());
    let delete_days = delete_days.unwrap_or_default();
    if delete_days > ctx.data().config.ban_max_delete_days {
        let message = tf(
            lang,
            TranslationKey::ModerationDeleteDaysRange,
            &[&ctx.data().config.ban_max_delete_days],
        );
        ctx.say(message).await?;
        return Ok(());
    }
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

    let message = tf(
        lang,
        TranslationKey::ModerationBanned,
        &[&member_name, &reason],
    );

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(ctx.data().config.colors.error);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
