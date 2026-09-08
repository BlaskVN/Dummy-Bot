use super::{denial_translation, handle_execution_error};
use crate::i18n::{TranslationKey, t};
use crate::moderation_cases::{
    ModerationIntent, ModerationRequest, SerenityDiscordExecutor, execute_moderation_action,
};
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

    let reason = reason.unwrap_or_else(|| t(lang, TranslationKey::ModerationNoReason).to_string());
    let executor = SerenityDiscordExecutor::new(ctx.http());

    match execute_moderation_action(
        &executor,
        &ctx.data().db_pool,
        ModerationRequest {
            guild_id,
            target: member.user.id,
            moderator: ctx.author().id,
            intent: ModerationIntent::Kick,
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
