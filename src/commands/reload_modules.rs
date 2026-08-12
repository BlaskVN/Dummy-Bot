use crate::i18n::Language;
use crate::ui::{self, Tone};
use crate::{Context, Error};

/// Reload all Rhai script modules from disk without restarting the bot.
#[poise::command(slash_command, owners_only, rename = "reload_modules")]
pub async fn reload_modules(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    match ctx.data().rhai_manager.reload().await {
        Ok(()) => {
            let msg = match lang {
                Language::Vietnamese => "Tất cả Rhai script modules đã được nạp lại thành công!",
                Language::Japanese => {
                    "すべての Rhai スクリプトモジュールが正常に再読み込みされました！"
                }
                Language::English => "All Rhai script modules reloaded successfully!",
            };
            ui::private_reply(ctx, Tone::Success, msg).await?;
        }
        Err(err) => {
            let msg = match lang {
                Language::Vietnamese => format!("Lỗi khi nạp lại Rhai modules: {}", err),
                Language::Japanese => format!("Rhai モジュールの再読み込みエラー: {}", err),
                Language::English => format!("Error reloading Rhai modules: {}", err),
            };
            ui::private_reply(ctx, Tone::Error, msg).await?;
        }
    }
    Ok(())
}
