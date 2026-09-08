use super::{denial_translation, handle_execution_error};
use crate::i18n::t;
use crate::moderation_cases::{
    ModerationIntent, ModerationRequest, SerenityDiscordExecutor, execute_moderation_action,
};
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

    let executor = SerenityDiscordExecutor::new(ctx.http());
    match execute_moderation_action(
        &executor,
        &ctx.data().db_pool,
        ModerationRequest {
            guild_id,
            target: member.user.id,
            moderator: ctx.author().id,
            intent: ModerationIntent::Warn,
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
