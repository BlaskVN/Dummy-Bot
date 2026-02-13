use crate::i18n::{get_guild_language, t, tf, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Display current server configuration.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn settings(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    // Get prefix from guild_config
    let prefix = sqlx::query_as::<_, (String,)>(
        "SELECT prefix FROM guild_config WHERE guild_id = ?",
    )
        .bind(guild_id.to_string())
        .fetch_optional(&ctx.data().db_pool)
        .await?
        .map(|(p, )| p)
        .unwrap_or_else(|| "!".to_string());

    // Get log channel from message_log_config
    let log_channel = sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
        .bind(guild_id.to_string())
        .fetch_optional(&ctx.data().db_pool)
        .await?;

    let log_channel_display = match log_channel {
        Some((id, enabled)) if enabled == 1 => format!("<#{}>", id),
        Some((id, _)) => format!("<#{}> (disabled)", id),
        None => t(lang, TranslationKey::SettingsNotConfigured).to_string(),
    };

    let prefix_text = tf(lang, TranslationKey::SettingsPrefix, &[&prefix]);
    let log_channel_text = tf(lang, TranslationKey::SettingsLogChannel, &[&log_channel_display]);

    let description = format!(
        "├ {}\n└ {}",
        prefix_text, log_channel_text
    );

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::SettingsTitle))
        .description(description)
        .color(0x95a5a6); // Gray

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Set a custom command prefix for this server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn setprefix(
    ctx: Context<'_>,
    #[description = "New prefix for the server"]
    #[min_length = 1]
    #[max_length = 5]
    new_prefix: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    sqlx::query(
        "INSERT INTO guild_config (guild_id, prefix, updated_at)
         VALUES (?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(guild_id) DO UPDATE SET prefix = excluded.prefix, updated_at = CURRENT_TIMESTAMP",
    )
        .bind(guild_id.to_string())
        .bind(&new_prefix)
        .execute(&ctx.data().db_pool)
        .await?;

    tracing::info!(
        guild = %guild_id,
        new_prefix = %new_prefix,
        admin = %ctx.author().name,
        "Prefix updated"
    );

    let message = tf(lang, TranslationKey::PrefixChanged, &[&new_prefix]);

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0x2ecc71); // Green

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
