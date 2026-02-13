use crate::i18n::{set_guild_language, tf, Language, TranslationKey};
use crate::{Context, Error};

/// Change the bot's language for this server.
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn language(
    ctx: Context<'_>,
    #[description = "Language code (en, vi, ja)"]
    #[autocomplete = "autocomplete_language"]
    lang_code: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    // Parse language code
    let language = match lang_code.to_lowercase().as_str() {
        "en" | "english" => Language::English,
        "vi" | "vietnamese" | "tiếng việt" => Language::Vietnamese,
        "ja" | "japanese" | "日本語" => Language::Japanese,
        _ => {
            ctx.say("Invalid language code. Available: `en`, `vi`, `ja`").await?;
            return Ok(());
        }
    };

    // Save to database
    set_guild_language(&ctx.data().db_pool, guild_id, language).await?;

    tracing::info!(
        guild = %guild_id,
        admin = %ctx.author().name,
        language = %language.to_str(),
        "Language changed"
    );

    // Send confirmation in the new language
    let message = tf(language, TranslationKey::LanguageChanged, &[&language.display_name()]);
    ctx.say(message).await?;

    Ok(())
}

/// Autocomplete for language command
async fn autocomplete_language<'a>(
    _ctx: Context<'_>,
    partial: &'a str,
) -> Vec<String> {
    let languages = vec![
        ("en", "English"),
        ("vi", "Tiếng Việt"),
        ("ja", "日本語"),
    ];

    languages
        .iter()
        .filter(|(code, name)| {
            code.starts_with(&partial.to_lowercase())
                || name.to_lowercase().starts_with(&partial.to_lowercase())
        })
        .map(|(code, _)| code.to_string())
        .collect()
}
