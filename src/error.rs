use crate::{Data, Error};

/// Centralized error handler for the Poise framework.
///
/// This function is called whenever a command returns `Err(...)`.
/// It logs the error with tracing and sends a user-friendly message
/// back to the Discord channel without crashing the bot.
pub async fn on_error(error: poise::FrameworkError<'_, Data, Error>) {
    match error {
        poise::FrameworkError::Command { error, ctx, .. } => {
            tracing::error!(
                command = ctx.command().name,
                user = %ctx.author().name,
                error = %error,
                "Command error"
            );
            let _ = ctx.say(format!("❌ Đã xảy ra lỗi: {}", error)).await;
        }
        poise::FrameworkError::ArgumentParse { error, ctx, .. } => {
            tracing::warn!(
                command = ctx.command().name,
                error = %error,
                "Argument parse error"
            );
            let _ = ctx.say(format!("⚠️ Tham số không hợp lệ: {}", error)).await;
        }
        poise::FrameworkError::CommandCheckFailed { error, ctx, .. } => {
            tracing::warn!(command = ctx.command().name, "Command check failed");
            if let Some(error) = error {
                let _ = ctx.say(format!("🚫 Bạn không có quyền: {}", error)).await;
            }
        }
        poise::FrameworkError::MissingBotPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            tracing::warn!(
                command = ctx.command().name,
                permissions = %missing_permissions,
                "Bot missing permissions"
            );
            let _ = ctx
                .say(format!("🔒 Bot thiếu quyền: {}", missing_permissions))
                .await;
        }
        poise::FrameworkError::MissingUserPermissions {
            missing_permissions,
            ctx,
            ..
        } => {
            if let Some(perms) = missing_permissions {
                let _ = ctx.say(format!("🔒 Bạn thiếu quyền: {}", perms)).await;
            }
        }
        other => {
            tracing::error!("Unhandled framework error: {}", other);
        }
    }
}
