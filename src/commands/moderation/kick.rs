use super::execute_moderation_pipeline;
use crate::moderation_cases::ModerationIntent;
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
    execute_moderation_pipeline(
        ctx,
        member.user.id,
        ModerationIntent::Kick,
        reason,
        evidence,
    )
    .await
}
