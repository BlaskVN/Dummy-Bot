use crate::i18n::{Language, TranslationKey, set_guild_language, t, tf};
use crate::{Context, Error};

/// Change the bot's language for this server.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
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

    let language = match Language::try_parse(&lang_code) {
        Some(language) => language,
        None => {
            let guild_id = ctx.guild_id().expect("guild_only command");
            let current = ctx.data().language(guild_id).await;
            ctx.say(t(current, TranslationKey::LanguageInvalid)).await?;
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
    let message = tf(
        language,
        TranslationKey::LanguageChanged,
        &[&language.display_name()],
    );
    ctx.say(message).await?;

    Ok(())
}

/// Autocomplete for language command
async fn autocomplete_language(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    let languages = [("en", "English"), ("vi", "Tiếng Việt"), ("ja", "日本語")];

    languages
        .into_iter()
        .filter(|(code, name)| {
            code.starts_with(&partial.to_lowercase())
                || name.to_lowercase().starts_with(&partial.to_lowercase())
        })
        .map(|(code, _)| code.to_string())
        .collect()
}
