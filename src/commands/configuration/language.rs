use crate::i18n::{Language, TranslationKey, set_guild_language, tf};
use crate::ui::{self, Tone};
use crate::{Context, Error};

/// Set the bot's response language for this server.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn language(
    ctx: Context<'_>,
    #[description = "Language to set for this server"] language: Language,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

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
    ui::reply(ctx, Tone::Success, message).await?;

    Ok(())
}
