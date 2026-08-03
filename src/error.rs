use crate::i18n::{TranslationKey, t, tf};
use crate::{Data, Error};

/// Centralized error handler for the Poise framework.
///
/// This function is called whenever a command returns `Err(...)`.
/// It logs the error with tracing and sends a user-friendly message
/// back to the Discord channel without crashing the bot.
pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => ctx.data().language(guild_id).await,
                None => ctx.data().default_language(),
            };

            tracing::error!(
                command = ctx.command().name,
                user = %ctx.author().name,
                error = %error,
                "Command error"
            );

            let message = t(lang, TranslationKey::ErrorGeneric);
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::ArgumentParse { error, ctx, .. } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => ctx.data().language(guild_id).await,
                None => ctx.data().default_language(),
            };

            tracing::warn!(
                command = ctx.command().name,
                error = %error,
                "Argument parse error"
            );

            let message = tf(lang, TranslationKey::ModerationInvalidArgument, &[&error]);
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::CooldownHit {
            remaining_cooldown,
            ctx,
            ..
        } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => ctx.data().language(guild_id).await,
                None => ctx.data().default_language(),
            };
            let seconds =
                remaining_cooldown.as_secs() + u64::from(remaining_cooldown.subsec_nanos() != 0);
            let message = tf(lang, TranslationKey::ErrorCooldown, &[&seconds]);
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => ctx.data().language(guild_id).await,
                None => ctx.data().default_language(),
            };

            tracing::warn!(command = ctx.command().name, "Command check failed");
            let message = if let Some(error) = error {
                tf(
                    lang,
                    TranslationKey::ModerationUserMissingPermissions,
                    &[&error],
                )
            } else {
                // owners_only and other checks fire with error = None
                t(lang, TranslationKey::ErrorNoPermission).to_string()
            };
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => ctx.data().language(guild_id).await,
                None => ctx.data().default_language(),
            };

            tracing::warn!(
                command = ctx.command().name,
                permissions = %missing_permissions,
                "Bot missing permissions"
            );

            let message = tf(
                lang,
                TranslationKey::ModerationBotMissingPermissions,
                &[&missing_permissions],
            );
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::MissingUserPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => ctx.data().language(guild_id).await,
                None => ctx.data().default_language(),
            };

            if let Some(perms) = missing_permissions {
                let message = tf(
                    lang,
                    TranslationKey::ModerationUserMissingPermissions,
                    &[&perms],
                );
                let _ = ctx.say(message).await;
            }
        }
        other => {
            tracing::error!("Unhandled framework error: {}", other);
        }
    }
}
