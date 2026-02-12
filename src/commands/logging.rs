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
    #[description = "Kênh để gửi log tin nhắn"] log_channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

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

    ctx.say(format!(
        "✅ Đã bật message log. Kênh log: <#{}>",
        log_channel.id
    ))
    .await?;

    Ok(())
}

/// Disable message logging for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    // Update config to disabled
    let result = sqlx::query("UPDATE message_log_config SET enabled = 0 WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&ctx.data().db_pool)
        .await?;

    if result.rows_affected() == 0 {
        ctx.say("⚠️ Message logging chưa được thiết lập.").await?;
        return Ok(());
    }

    tracing::info!(
        guild = %guild_id,
        admin = %ctx.author().name,
        "Message logging disabled"
    );

    ctx.say("✅ Đã tắt message log.").await?;

    Ok(())
}

/// Show current message logging status.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let config = sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?;

    match config {
        Some((channel_id, enabled)) => {
            let status = if enabled == 1 {
                "✅ Đang bật"
            } else {
                "❌ Đang tắt"
            };
            ctx.say(format!(
                "📊 **Message Log Status**\n├ Trạng thái: {}\n└ Kênh log: <#{}>",
                status, channel_id
            ))
            .await?;
        }
        None => {
            ctx.say("⚠️ Message logging chưa được thiết lập. Sử dụng `/messagelog enable` để bật.")
                .await?;
        }
    }

    Ok(())
}
