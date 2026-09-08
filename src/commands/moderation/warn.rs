use super::execute_moderation_pipeline;
use crate::moderation_cases::ModerationIntent;
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
    execute_moderation_pipeline(
        ctx,
        member.user.id,
        ModerationIntent::Warn,
        Some(reason),
        evidence,
    )
    .await
}
