use super::{case_summary, denial_translation, send_case_summary};
use crate::i18n::{TranslationKey, t};
use crate::moderation_cases::{ModerationAction, create_case, valid_evidence_url};
use crate::permissions::moderation_denial;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Warn a member and record the moderation case.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_permissions = "MODERATE_MEMBERS"
)]
pub async fn warn(
    ctx: Context<'_>,
    #[description = "Member to warn"] member: serenity::Member,
    #[description = "Reason for warning"] reason: String,
    #[description = "Discord message link containing evidence"] evidence: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    if let Some(denial) = moderation_denial(ctx, member.user.id)? {
        ui::reply(ctx, Tone::Error, t(lang, denial_translation(denial))).await?;
        return Ok(());
    }
    if reason.trim().is_empty() {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::ModerationReasonRequired),
        )
        .await?;
        return Ok(());
    }
    if evidence
        .as_deref()
        .is_some_and(|url| !valid_evidence_url(url, guild_id))
    {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::ModerationInvalidEvidence),
        )
        .await?;
        return Ok(());
    }

    let case_number = create_case(
        &ctx.data().db_pool,
        guild_id,
        ModerationAction::Warn,
        member.user.id,
        ctx.author().id,
        &reason,
        evidence.as_deref(),
    )
    .await?;
    let summary = case_summary(
        lang,
        case_number,
        TranslationKey::ModerationActionWarn,
        member.user.id,
        ctx.author().id,
        &reason,
    );
    ui::reply(ctx, Tone::Success, &summary).await?;
    if let Err(error) = send_case_summary(ctx, guild_id, &summary).await {
        tracing::warn!(%guild_id, case_number, %error, "Failed to send moderation case summary");
    }
    Ok(())
}
