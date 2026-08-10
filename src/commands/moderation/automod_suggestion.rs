use crate::automod::{handle_suggestion, open_suggestion_id};
use crate::handlers::automod::send_suggestion;
use crate::i18n::{TranslationKey, t};
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Review and process moderation suggestions produced by Discord AutoMod.
#[poise::command(
    rename = "automod-suggestion",
    slash_command,
    subcommands("handle", "retry"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn automod_suggestion(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Mark an AutoMod suggestion as handled.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn handle(
    ctx: Context<'_>,
    #[description = "Member named in the suggestion"] member: serenity::Member,
    #[description = "Discord AutoMod rule ID"] rule_id: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let Ok(rule_id) = rule_id.parse::<u64>() else {
        ui::reply(
            ctx,
            Tone::Error,
            t(language, TranslationKey::AutoModSuggestionInvalidRule),
        )
        .await?;
        return Ok(());
    };
    let handled = handle_suggestion(
        &ctx.data().db_pool,
        guild_id,
        member.user.id.get(),
        rule_id,
        ctx.author().id.get(),
        chrono::Utc::now().timestamp(),
    )
    .await?;
    ui::reply(
        ctx,
        if handled {
            Tone::Success
        } else {
            Tone::Warning
        },
        t(
            language,
            if handled {
                TranslationKey::AutoModSuggestionHandled
            } else {
                TranslationKey::AutoModSuggestionNotFound
            },
        ),
    )
    .await?;
    Ok(())
}

/// Retry delivery of an AutoMod moderation suggestion.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn retry(
    ctx: Context<'_>,
    #[description = "Member named in the suggestion"] member: serenity::Member,
    #[description = "Discord AutoMod rule ID"] rule_id: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let Ok(rule_id) = rule_id.parse::<u64>() else {
        ui::reply(
            ctx,
            Tone::Error,
            t(language, TranslationKey::AutoModSuggestionInvalidRule),
        )
        .await?;
        return Ok(());
    };
    let Some(suggestion_id) =
        open_suggestion_id(&ctx.data().db_pool, guild_id, member.user.id.get(), rule_id).await?
    else {
        ui::reply(
            ctx,
            Tone::Warning,
            t(language, TranslationKey::AutoModSuggestionNotFound),
        )
        .await?;
        return Ok(());
    };
    let delivered = send_suggestion(
        ctx.serenity_context(),
        ctx.data(),
        suggestion_id,
        guild_id,
        member.user.id,
        serenity::RuleId::new(rule_id),
    )
    .await;
    ui::reply(
        ctx,
        if delivered {
            Tone::Success
        } else {
            Tone::Error
        },
        t(
            language,
            if delivered {
                TranslationKey::AutoModSuggestionDelivered
            } else {
                TranslationKey::AutoModSuggestionDeliveryFailed
            },
        ),
    )
    .await?;
    Ok(())
}
