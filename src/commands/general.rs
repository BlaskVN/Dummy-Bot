use crate::i18n::{get_guild_language, t, tf, Language, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Check bot latency and responsiveness.
#[poise::command(slash_command, prefix_command)]
pub async fn ping(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    let start = std::time::Instant::now();
    let msg = ctx.say(t(lang, TranslationKey::PingPong)).await?;
    let latency = start.elapsed();

    let message = tf(lang, TranslationKey::PingLatency, &[&latency.as_millis()]);
    msg.edit(ctx, poise::CreateReply::default().content(message)).await?;

    tracing::debug!(latency_ms = latency.as_millis(), "Ping command executed");
    Ok(())
}

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

/// Display current server information.
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => Language::English,
    };

    let guild = ctx
        .guild()
        .ok_or_else(|| anyhow::anyhow!("Could not fetch guild info"))?
        .clone();

    let member_count = guild.member_count;
    let name = &guild.name;
    let channel_count = guild.channels.len();
    let role_count = guild.roles.len();
    let created_at = guild.id.created_at();

    let name_text = tf(lang, TranslationKey::ServerInfoName, &[&name]);
    let members_text = tf(lang, TranslationKey::ServerInfoMembers, &[&member_count]);
    let channels_text = tf(lang, TranslationKey::ServerInfoChannels, &[&channel_count]);
    let roles_text = tf(lang, TranslationKey::ServerInfoRoles, &[&role_count]);
    let created_text = tf(lang, TranslationKey::ServerInfoCreated, &[&created_at.unix_timestamp()]);

    let description = format!(
        "├ {}\n├ {}\n├ {}\n├ {}\n└ {}",
        name_text,
        members_text,
        channels_text,
        roles_text,
        created_text
    );

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::ServerInfoTitle))
        .description(description)
        .color(0x9b59b6); // Purple

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
