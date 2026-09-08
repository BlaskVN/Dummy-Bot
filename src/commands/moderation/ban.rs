use super::execute_moderation_pipeline;
use crate::i18n::{TranslationKey, tf};
use crate::moderation_cases::ModerationIntent;
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Ban a member and record the moderation case.
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
    #[description = "Days of messages to delete (0-7)"]
    #[max = 7]
    delete_days: Option<u8>,
    #[description = "Discord message link containing evidence"] evidence: Option<String>,
    #[description = "Reason for ban"]
    #[rest]
    reason: Option<String>,
) -> Result<(), Error> {
    let delete_days = delete_days.unwrap_or_default();
    if delete_days > ctx.data().config.ban_max_delete_days {
        let guild_id = ctx
            .guild_id()
            .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
        let lang = ctx.data().language(guild_id).await;
        let message = tf(
            lang,
            TranslationKey::ModerationDeleteDaysRange,
            &[&ctx.data().config.ban_max_delete_days],
        );
        ui::reply(ctx, Tone::Error, message).await?;
        return Ok(());
    }

    execute_moderation_pipeline(
        ctx,
        member.user.id,
        ModerationIntent::Ban {
            delete_message_days: Some(delete_days),
        },
        reason,
        evidence,
    )
    .await
}
