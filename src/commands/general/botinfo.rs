use crate::database::load_donation_config;
use crate::i18n::{TranslationKey, t, tf};
use crate::ui::{self, Tone};
use crate::{Context, Error};

/// Display bot information and uptime.
#[poise::command(slash_command, user_cooldown = 5)]
pub async fn botinfo(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let uptime = ctx.data().start_time.elapsed();
    let hours = uptime.as_secs() / 3600;
    let minutes = (uptime.as_secs() % 3600) / 60;
    let seconds = uptime.as_secs() % 60;

    let guild_count = ctx.cache().guilds().len();

    let uptime_text = tf(
        lang,
        TranslationKey::BotInfoUptime,
        &[&hours, &minutes, &seconds],
    );
    let servers_text = tf(lang, TranslationKey::BotInfoServers, &[&guild_count]);
    let language_text = t(lang, TranslationKey::BotInfoLanguage);
    let framework_text = t(lang, TranslationKey::BotInfoFramework);

    let mut details = vec![
        uptime_text,
        servers_text,
        language_text.to_string(),
        framework_text.to_string(),
    ];
    if load_donation_config(&ctx.data().db_pool).await?.is_some() {
        details.push(t(lang, TranslationKey::BotInfoDonate).to_string());
    }
    let description = details.join("\n");

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(t(lang, TranslationKey::BotInfoTitle))
        .description(description);

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}
