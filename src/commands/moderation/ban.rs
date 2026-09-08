use super::{denial_translation, handle_execution_error};
use crate::i18n::{TranslationKey, t, tf};
use crate::moderation_cases::{
    ModerationIntent, ModerationRequest, SerenityDiscordExecutor, execute_moderation_action,
};
use crate::permissions::moderation_denial;
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
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    if let Some(denial) = moderation_denial(ctx, member.user.id)? {
        ui::reply(ctx, Tone::Error, t(lang, denial_translation(denial))).await?;
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
        ui::reply(ctx, Tone::Error, message).await?;
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
            intent: ModerationIntent::Ban {
                delete_message_days: Some(delete_days),
            },
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
