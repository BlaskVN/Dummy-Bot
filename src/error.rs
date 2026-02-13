use crate::i18n::{get_guild_language, tf, Language, TranslationKey};
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
                Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
                None => Language::English,
            };

            tracing::error!(
                command = ctx.command().name,
                user = %ctx.author().name,
                error = %error,
                "Command error"
            );

            let message = tf(lang, TranslationKey::ErrorGeneric, &[&error]);
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::ArgumentParse { error, ctx, .. } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
                None => Language::English,
            };

            tracing::warn!(
                command = ctx.command().name,
                error = %error,
                "Argument parse error"
            );

            let message = match lang {
                Language::Vietnamese => format!("Tham số không hợp lệ: {}", error),
                Language::Japanese => format!("無効なパラメータ：{}", error),
                _ => format!("Invalid argument: {}", error),
            };
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
                None => Language::English,
            };

            tracing::warn!(command = ctx.command().name, "Command check failed");
            if let Some(error) = error {
                let message = match lang {
                    Language::Vietnamese => format!("Bạn không có quyền: {}", error),
                    Language::Japanese => format!("権限がありません：{}", error),
                    _ => format!("You don't have permission: {}", error),
                };
                let _ = ctx.say(message).await;
            }
        }
        poise::FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
                None => Language::English,
            };

            tracing::warn!(
                command = ctx.command().name,
                permissions = %missing_permissions,
                "Bot missing permissions"
            );

            let message = match lang {
                Language::Vietnamese => format!("Bot thiếu quyền: {}", missing_permissions),
                Language::Japanese => format!("Botの権限が不足しています：{}", missing_permissions),
                _ => format!("Bot missing permissions: {}", missing_permissions),
            };
            let _ = ctx.say(message).await;
        }
        poise::FrameworkError::MissingUserPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            let lang = match ctx.guild_id() {
                Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
                None => Language::English,
            };

            if let Some(perms) = missing_permissions {
                let message = match lang {
                    Language::Vietnamese => format!("Bạn thiếu quyền: {}", perms),
                    Language::Japanese => format!("権限が不足しています：{}", perms),
                    _ => format!("You're missing permissions: {}", perms),
                };
                let _ = ctx.say(message).await;
            }
        }
        other => {
            tracing::error!("Unhandled framework error: {}", other);
        }
    }
}
