use crate::{Context, Error};

/// Check bot latency and responsiveness.
#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let start = std::time::Instant::now();
    let msg = ctx.say("🏓 Pong!").await?;
    let latency = start.elapsed();

    msg.edit(ctx, poise::CreateReply::default().content(
        format!("🏓 Pong! Latency: **{}ms**", latency.as_millis())
    )).await?;

    tracing::debug!(latency_ms = latency.as_millis(), "Ping command executed");
    Ok(())
}

/// Display bot information and uptime.
#[poise::command(slash_command, prefix_command)]
pub async fn botinfo(ctx: Context<'_>) -> Result<(), Error> {
    let uptime = ctx.data().start_time.elapsed();
    let hours = uptime.as_secs() / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    let seconds = uptime.as_secs() % 60;

    let guild_count = ctx.cache().guilds().len();

    ctx.say(format!(
        "🤖 **Bot Information**\n\
         ├ **Uptime:** {}h {}m {}s\n\
         ├ **Servers:** {}\n\
         ├ **Language:** Rust 🦀\n\
         └ **Framework:** Poise + Serenity",
        hours, minutes, seconds, guild_count
    ))
    .await?;

    Ok(())
}

/// Display current server information.
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    let guild = ctx
        .guild()
        .ok_or_else(|| anyhow::anyhow!("Could not fetch guild info"))?
        .clone();

    let member_count = guild.member_count;
    let name = &guild.name;
    let channel_count = guild.channels.len();
    let role_count = guild.roles.len();
    let created_at = guild.id.created_at();

    ctx.say(format!(
        "📊 **Server Information**\n\
         ├ **Name:** {}\n\
         ├ **Members:** {}\n\
         ├ **Channels:** {}\n\
         ├ **Roles:** {}\n\
         └ **Created:** <t:{}:R>",
        name,
        member_count,
        channel_count,
        role_count,
        created_at.unix_timestamp()
    ))
    .await?;

    Ok(())
}
