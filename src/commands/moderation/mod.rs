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
