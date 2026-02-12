use crate::{Context, Error};

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

    let config = sqlx::query_as::<_, (String, Option<String>)>(
        "SELECT prefix, log_channel_id FROM guild_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?;

    let (prefix, log_channel) = config.unwrap_or(("!".to_string(), None));

    let log_channel_display = match log_channel {
        Some(id) => format!("<#{}>", id),
        None => "Chưa thiết lập".to_string(),
    };

    ctx.say(format!(
        "⚙️ **Cấu hình Server**\n\
         ├ **Prefix:** `{}`\n\
         └ **Log Channel:** {}",
        prefix, log_channel_display
    ))
    .await?;

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
    #[description = "Prefix mới cho server"]
    #[min_length = 1]
    #[max_length = 5]
    new_prefix: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

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

    ctx.say(format!(
        "✅ Đã đổi prefix thành `{}`",
        new_prefix
    ))
    .await?;

    Ok(())
}
