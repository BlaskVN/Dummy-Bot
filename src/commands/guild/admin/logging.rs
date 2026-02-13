use crate::i18n::{get_guild_language, t, tf, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Parent command for message logging management.
#[poise::command(
    slash_command,
    subcommands("enable", "disable", "status"),
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn messagelog(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enable message logging for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn enable(
    ctx: Context<'_>,
    #[description = "Channel to send message logs to"] log_channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    // Insert or update config
    sqlx::query(
        "INSERT INTO message_log_config (guild_id, log_channel_id, enabled)
         VALUES (?, ?, 1)
         ON CONFLICT(guild_id) DO UPDATE SET log_channel_id = excluded.log_channel_id, enabled = 1",
    )
        .bind(guild_id.to_string())
        .bind(log_channel.id.to_string())
        .execute(&ctx.data().db_pool)
        .await?;

    tracing::info!(
        guild = %guild_id,
        channel = %log_channel.id,
        admin = %ctx.author().name,
        "Message logging enabled"
    );

    let message = tf(lang, TranslationKey::MessageLogEnabled, &[&log_channel.id]);

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0x2ecc71); // Green

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Disable message logging for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    // Update config to disabled
    let result = sqlx::query("UPDATE message_log_config SET enabled = 0 WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&ctx.data().db_pool)
        .await?;

    if result.rows_affected() == 0 {
        let embed = serenity::CreateEmbed::new()
            .description(t(lang, TranslationKey::MessageLogNotSetup))
            .color(0xe67e22); // Orange
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }

    tracing::info!(
        guild = %guild_id,
        admin = %ctx.author().name,
        "Message logging disabled"
    );

    let embed = serenity::CreateEmbed::new()
        .description(t(lang, TranslationKey::MessageLogDisabled))
        .color(0x2ecc71); // Green

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Show current message logging status.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    let config = sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
        .bind(guild_id.to_string())
        .fetch_optional(&ctx.data().db_pool)
        .await?;

    match config {
        Some((channel_id, enabled)) => {
            let status = if enabled == 1 {
                t(lang, TranslationKey::MessageLogStatusEnabled)
            } else {
                t(lang, TranslationKey::MessageLogStatusDisabled)
            };

            let status_label = t(lang, TranslationKey::MessageLogStatus);
            let channel_text = tf(lang, TranslationKey::MessageLogChannel, &[&channel_id]);

            let description = format!(
                "├ {} {}\n└ {}",
                status_label, status, channel_text
            );

            let embed = serenity::CreateEmbed::new()
                .title(t(lang, TranslationKey::MessageLogStatusTitle))
                .description(description)
                .color(0x3498db); // Blue

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        None => {
            let embed = serenity::CreateEmbed::new()
                .description(t(lang, TranslationKey::MessageLogUseEnable))
                .color(0xe67e22); // Orange
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
    }

    Ok(())
}
