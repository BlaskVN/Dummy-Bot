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
    if let Some(donation) = load_donation_config(&ctx.data().db_pool).await? {
        let mut donate_parts = Vec::new();
        donate_parts.push(t(lang, TranslationKey::BotInfoDonate).to_string());
        if let Some(msg) = &donation.message
            && !msg.trim().is_empty()
        {
            donate_parts.push(msg.trim().to_string());
        }
        if let Some(url) = &donation.url
            && !url.trim().is_empty()
        {
            donate_parts.push(url.trim().to_string());
        }
        details.push(donate_parts.join("\n"));
    }
    let description = details.join("\n");

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(t(lang, TranslationKey::BotInfoTitle))
        .description(description);

    ctx.send(ui::embed_reply(embed)).await?;

    Ok(())
}
