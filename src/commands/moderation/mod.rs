pub mod ban;
pub mod kick;
pub mod purge;
pub mod timeout;
pub mod warn;

use crate::i18n::{Language, TranslationKey, t, tf};
use crate::permissions::ModerationDenial;
use crate::{Context, Data, Error};
use poise::serenity_prelude as serenity;

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        kick::kick(),
        ban::ban(),
        purge::purge(),
        warn::warn(),
        timeout::timeout(),
    ]
}

fn denial_translation(denial: ModerationDenial) -> TranslationKey {
    match denial {
        ModerationDenial::SelfTarget => TranslationKey::ModerationCannotTargetSelf,
        ModerationDenial::UserHierarchy => TranslationKey::ModerationUserHierarchy,
        ModerationDenial::BotHierarchy => TranslationKey::ModerationBotHierarchy,
    }
}

fn case_summary(
    language: Language,
    case_number: i64,
    action: TranslationKey,
    target: serenity::UserId,
    moderator: serenity::UserId,
    reason: &str,
) -> String {
    tf(
        language,
        TranslationKey::ModerationCaseSummary,
        &[
            &case_number,
            &t(language, action),
            &target,
            &moderator,
            &reason,
        ],
    )
}

async fn send_case_summary(
    ctx: Context<'_>,
    guild_id: serenity::GuildId,
    summary: &str,
) -> Result<(), Error> {
    let Some(channel): Option<String> =
        sqlx::query_scalar("SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(&ctx.data().db_pool)
            .await?
    else {
        return Ok(());
    };
    serenity::ChannelId::new(channel.parse()?)
        .send_message(
            ctx.http(),
            serenity::CreateMessage::new()
                .content(summary)
                .allowed_mentions(serenity::CreateAllowedMentions::new()),
        )
        .await?;
    Ok(())
}
