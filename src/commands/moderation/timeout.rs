use super::{case_summary, denial_translation, send_case_summary};
use crate::i18n::{TranslationKey, t};
use crate::moderation_cases::{ModerationAction, create_case, valid_evidence_url};
use crate::permissions::moderation_denial;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

const MAX_TIMEOUT_MINUTES: u32 = 28 * 24 * 60;

fn valid_duration(minutes: u32) -> bool {
    (1..=MAX_TIMEOUT_MINUTES).contains(&minutes)
}

/// Temporarily time out a member and record the moderation case.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MODERATE_MEMBERS",
    required_permissions = "MODERATE_MEMBERS",
    required_bot_permissions = "MODERATE_MEMBERS"
)]
pub async fn timeout(
    ctx: Context<'_>,
    #[description = "Member to time out"] mut member: serenity::Member,
    #[description = "Timeout length in minutes"] minutes: u32,
    #[description = "Reason for timeout"] reason: String,
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
    if !valid_duration(minutes) {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::ModerationTimeoutRange),
        )
        .await?;
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

    let until = serenity::Timestamp::from_unix_timestamp(
        chrono::Utc::now().timestamp() + i64::from(minutes) * 60,
    )?;
    member
        .disable_communication_until_datetime(ctx.http(), until)
        .await?;

    let case_number = match create_case(
        &ctx.data().db_pool,
        guild_id,
        ModerationAction::Timeout,
        member.user.id,
        ctx.author().id,
        &reason,
        evidence.as_deref(),
    )
    .await
    {
        Ok(number) => number,
        Err(error) => {
            tracing::error!(%guild_id, target = %member.user.id, moderator = %ctx.author().id, %error, "Discord timeout succeeded but moderation case creation failed");
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
        TranslationKey::ModerationActionTimeout,
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

#[cfg(test)]
mod tests {
    use super::valid_duration;

    #[test]
    fn accepts_only_discord_timeout_range() {
        assert!(!valid_duration(0));
        assert!(valid_duration(1));
        assert!(valid_duration(40_320));
        assert!(!valid_duration(40_321));
    }
}
