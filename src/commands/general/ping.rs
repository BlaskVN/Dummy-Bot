use crate::i18n::{TranslationKey, t, tf};
use crate::{Context, Error};

/// Check bot latency and responsiveness.
#[poise::command(slash_command, prefix_command, user_cooldown = 5)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let start = std::time::Instant::now();
    let msg = ctx.say(t(lang, TranslationKey::PingPong)).await?;
    let latency = start.elapsed();

    let message = tf(lang, TranslationKey::PingLatency, &[&latency.as_millis()]);
    msg.edit(ctx, poise::CreateReply::default().content(message))
        .await?;

    tracing::debug!(latency_ms = latency.as_millis(), "Ping command executed");
    Ok(())
}
