pub mod automod_suggestion;
pub mod ban;
pub mod case;
pub mod kick;
pub mod purge;
pub mod timeout;
pub mod warn;

use crate::i18n::{Language, TranslationKey, t};
use crate::permissions::ModerationDenial;
use crate::ui::{self, Tone};
use crate::{Context, Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        automod_suggestion::automod_suggestion(),
        kick::kick(),
        ban::ban(),
        purge::purge(),
        warn::warn(),
        timeout::timeout(),
        case::cases(),
    ]
}

pub fn denial_translation(denial: ModerationDenial) -> TranslationKey {
    match denial {
        ModerationDenial::SelfTarget => TranslationKey::ModerationCannotTargetSelf,
        ModerationDenial::UserHierarchy => TranslationKey::ModerationUserHierarchy,
        ModerationDenial::BotHierarchy => TranslationKey::ModerationBotHierarchy,
    }
}

pub async fn handle_execution_error(
    ctx: Context<'_>,
    lang: Language,
    error: crate::moderation_cases::ModerationExecutionError,
) -> Result<(), Error> {
    match error {
        crate::moderation_cases::ModerationExecutionError::Denial(denial) => {
            ui::reply(ctx, Tone::Error, t(lang, denial_translation(denial))).await?;
        }
        crate::moderation_cases::ModerationExecutionError::EmptyReason => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ModerationReasonRequired),
            )
            .await?;
        }
        crate::moderation_cases::ModerationExecutionError::InvalidEvidence => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ModerationInvalidEvidence),
            )
            .await?;
        }
        crate::moderation_cases::ModerationExecutionError::DatabaseFailedAfterDiscordAction {
            ..
        } => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ModerationActionCaseFailed),
            )
            .await?;
        }
        crate::moderation_cases::ModerationExecutionError::DiscordFailed(err) => {
            return Err(err.into());
        }
        crate::moderation_cases::ModerationExecutionError::Other(err) => {
            return Err(err.into());
        }
    }
    Ok(())
}

pub async fn execute_moderation_pipeline(
    ctx: Context<'_>,
    target_user_id: poise::serenity_prelude::UserId,
    intent: crate::moderation_cases::ModerationIntent,
    reason: Option<String>,
    evidence_url: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    let denial = crate::permissions::moderation_denial(ctx, target_user_id)?;
    let default_reason = t(lang, TranslationKey::ModerationNoReason).to_string();
    let final_reason = reason.unwrap_or(default_reason);

    let executor = crate::moderation_cases::SerenityDiscordExecutor::new(ctx.http());
    match crate::moderation_cases::execute_moderation_action(
        &executor,
        &ctx.data().db_pool,
        crate::moderation_cases::ModerationRequest {
            guild_id,
            target: target_user_id,
            moderator: ctx.author().id,
            intent,
            reason: &final_reason,
            evidence_url: evidence_url.as_deref(),
            language: lang,
            denial,
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
