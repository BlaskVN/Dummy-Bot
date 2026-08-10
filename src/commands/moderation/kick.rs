use super::{case_summary, denial_translation, send_case_summary};
use crate::i18n::{TranslationKey, t};
use crate::moderation_cases::{ModerationAction, create_case, valid_evidence_url};
use crate::permissions::moderation_denial;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Kick a member and record the moderation case.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "KICK_MEMBERS",
    required_permissions = "KICK_MEMBERS",
    required_bot_permissions = "KICK_MEMBERS"
)]
pub async fn kick(
    ctx: Context<'_>,
    #[description = "Member to kick"] member: serenity::Member,
    #[description = "Discord message link containing evidence"] evidence: Option<String>,
    #[description = "Reason for kick"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    if let Some(denial) = moderation_denial(ctx, member.user.id)? {
        ui::reply(ctx, Tone::Error, t(lang, denial_translation(denial))).await?;
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

    let reason = reason.unwrap_or_else(|| t(lang, TranslationKey::ModerationNoReason).to_string());
    member.kick_with_reason(&ctx.http(), &reason).await?;

    let case_number = match create_case(
        &ctx.data().db_pool,
        guild_id,
        ModerationAction::Kick,
        member.user.id,
        ctx.author().id,
        &reason,
        evidence.as_deref(),
    )
    .await
    {
        Ok(number) => number,
        Err(error) => {
            tracing::error!(%guild_id, target = %member.user.id, moderator = %ctx.author().id, %error, "Discord kick succeeded but moderation case creation failed");
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ModerationActionCaseFailed),
            )
            .await?;
            return Ok(());
        }
    };
    let summary = case_summary(
        lang,
        case_number,
        TranslationKey::ModerationActionKick,
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
