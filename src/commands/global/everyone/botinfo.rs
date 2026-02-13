use crate::i18n::{get_guild_language, t, tf, Language, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Display bot information and uptime.
#[poise::command(slash_command, prefix_command)]
pub async fn botinfo(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    let uptime = ctx.data().start_time.elapsed();
    let hours = uptime.as_secs() / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    let seconds = uptime.as_secs() % 60;

    let guild_count = ctx.cache().guilds().len();

    let uptime_text = tf(lang, TranslationKey::BotInfoUptime, &[&hours, &minutes, &seconds]);
    let servers_text = tf(lang, TranslationKey::BotInfoServers, &[&guild_count]);
    let language_text = t(lang, TranslationKey::BotInfoLanguage);
    let framework_text = t(lang, TranslationKey::BotInfoFramework);

    let description = format!(
        "├ {}\n├ {}\n├ {}\n└ {}",
        uptime_text, servers_text, language_text, framework_text
    );

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::BotInfoTitle))
        .description(description)
        .color(0x3498db); // Blue

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
