use super::{case_summary, denial_translation, send_case_summary};
use crate::i18n::{TranslationKey, t, tf};
use crate::moderation_cases::{ModerationAction, create_case, valid_evidence_url};
use crate::permissions::moderation_denial;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Ban a member from the server.
#[poise::command(
    slash_command,
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
    #[description = "Discord message link containing evidence"] evidence: Option<String>,
    #[description = "Reason for ban"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    if let Some(denial) = moderation_denial(ctx, member.user.id)? {
        ctx.say(t(lang, denial_translation(denial))).await?;
        return Ok(());
    }
    if evidence
        .as_deref()
        .is_some_and(|url| !valid_evidence_url(url, guild_id))
    {
        ctx.say(t(lang, TranslationKey::ModerationInvalidEvidence))
            .await?;
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
    member
        .ban_with_reason(&ctx.http(), delete_days, &reason)
        .await?;

    let case_number = match create_case(
        &ctx.data().db_pool,
        guild_id,
        ModerationAction::Ban,
        member.user.id,
        ctx.author().id,
        &reason,
        evidence.as_deref(),
    )
    .await
    {
        Ok(number) => number,
        Err(error) => {
            tracing::error!(%guild_id, target = %member.user.id, moderator = %ctx.author().id, %error, "Discord ban succeeded but moderation case creation failed");
            ctx.say(t(lang, TranslationKey::ModerationActionCaseFailed))
                .await?;
            return Ok(());
        }
    };
    let summary = case_summary(
        lang,
        case_number,
        TranslationKey::ModerationActionBan,
        member.user.id,
        ctx.author().id,
        &reason,
    );
    ctx.send(
        poise::CreateReply::default()
            .content(&summary)
            .allowed_mentions(serenity::CreateAllowedMentions::new()),
    )
    .await?;
    if let Err(error) = send_case_summary(ctx, guild_id, &summary).await {
        tracing::warn!(%guild_id, case_number, %error, "Failed to send moderation case summary");
    }

    Ok(())
}
