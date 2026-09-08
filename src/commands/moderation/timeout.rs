use super::{denial_translation, handle_execution_error};
use crate::i18n::{TranslationKey, t};
use crate::moderation_cases::{
    ModerationIntent, ModerationRequest, SerenityDiscordExecutor, execute_moderation_action,
};
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
    #[description = "Member to time out"] member: serenity::Member,
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

    let duration = std::time::Duration::from_secs(u64::from(minutes) * 60);
    let executor = SerenityDiscordExecutor::new(ctx.http());

    match execute_moderation_action(
        &executor,
        &ctx.data().db_pool,
        ModerationRequest {
            guild_id,
            target: member.user.id,
            moderator: ctx.author().id,
            intent: ModerationIntent::Timeout { duration },
            reason: &reason,
            evidence_url: evidence.as_deref(),
            language: lang,
        },
    )
    .await
    {
        Ok(executed) => {
            ui::reply(ctx, Tone::Success, executed.summary_text).await?;
        }
        Err(error) => {
            handle_execution_error(ctx, lang, error).await?;
        }
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
