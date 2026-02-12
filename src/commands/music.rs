use crate::{Context, Error};

/// Play a song (placeholder — demonstrates extensibility).
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn play(
    ctx: Context<'_>,
    #[description = "URL hoặc tên bài hát"]
    #[rest]
    query: String,
) -> Result<(), Error> {
    // TODO: Integrate with songbird or similar voice crate
    tracing::info!(
        user = %ctx.author().name,
        query = %query,
        "Play command invoked (placeholder)"
    );

    ctx.say(format!(
        "🎵 **Music Module** (Placeholder)\n\
         └ Query: `{}`\n\n\
         _Module âm nhạc đang được phát triển. Hãy tích hợp `songbird` để sử dụng._",
        query
    ))
    .await?;

    Ok(())
}

/// Stop the current playback (placeholder).
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn stop(ctx: Context<'_>) -> Result<(), Error> {
    tracing::info!(
        user = %ctx.author().name,
        "Stop command invoked (placeholder)"
    );

    ctx.say("⏹️ Đã dừng phát nhạc. _(Placeholder)_").await?;

    Ok(())
}
