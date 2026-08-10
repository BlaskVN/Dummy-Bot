use crate::i18n::{TranslationKey, t, tf};
use crate::message_log_health::MessageLogHealth;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Display current server configuration.
#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn settings(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    // Get log channel from message_log_config
    let log_channel = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT log_channel_id, enabled, health FROM message_log_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?;

    let (log_channel_display, log_health) = match log_channel {
        Some((id, 1, health)) => (format!("<#{}>", id), health),
        Some((id, _, health)) => (
            format!(
                "<#{}> ({})",
                id,
                t(lang, TranslationKey::MessageLogStatusDisabled)
            ),
            health,
        ),
        None => (
            t(lang, TranslationKey::SettingsNotConfigured).to_string(),
            "disabled".to_owned(),
        ),
    };
    let timezone = sqlx::query_scalar::<_, Option<String>>(
        "SELECT iana_name FROM guild_timezone WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?
    .flatten()
    .unwrap_or_else(|| t(lang, TranslationKey::SettingsNotConfigured).to_string());
    let moderation_channel = sqlx::query_scalar::<_, String>(
        "SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?
    .map(|id| format!("<#{id}>"))
    .unwrap_or_else(|| t(lang, TranslationKey::SettingsNotConfigured).to_string());
    let game = crate::game_config::game_config(&ctx.data().db_pool, guild_id).await?;

    let log_channel_text = tf(
        lang,
        TranslationKey::SettingsLogChannel,
        &[&log_channel_display],
    );
    let log_health = t(
        lang,
        match MessageLogHealth::parse(&log_health) {
            MessageLogHealth::Disabled => TranslationKey::MessageLogHealthDisabled,
            MessageLogHealth::Healthy => TranslationKey::MessageLogHealthHealthy,
            MessageLogHealth::Degraded => TranslationKey::MessageLogHealthDegraded,
        },
    );
    let log_health_text = tf(lang, TranslationKey::MessageLogHealth, &[&log_health]);
    let timezone_text = tf(lang, TranslationKey::SettingsTimezone, &[&timezone]);
    let moderation_channel_text = tf(
        lang,
        TranslationKey::SettingsModerationChannel,
        &[&moderation_channel],
    );

    let game_text = game.map_or_else(
        || "Game: not configured".to_owned(),
        |(config, pool)| {
            let pool = pool.iter().map(|id| format!("<#{id}>")).collect::<Vec<_>>().join(", ");
            let status = if timezone == t(lang, TranslationKey::SettingsNotConfigured) {
                "disabled: time zone not configured"
            } else {
                "enabled"
            };
            format!("Game: {} (`{}`) — {status}\n  Role: <@&{}> | Channel: <#{}>\n  Primary: <#{}> | Pool: {}\n  Activity: {}{}",
                config.display_name, config.game_key, config.role_id, config.game_channel_id,
                config.primary_voice_channel_id, pool, config.activity_name,
                config.activity_application_id.map_or_else(String::new, |id| format!(" (app `{id}`)")))
        },
    );
    let detection = crate::handlers::activity_presence::detection_status(
        ctx.data().config.guild_presences_enabled,
    );
    let description = format!(
        "├ {}\n├ {}\n├ {}\n├ {}\n├ Activity Detection: {}\n└ {}",
        log_channel_text,
        log_health_text,
        moderation_channel_text,
        timezone_text,
        detection,
        game_text
    );

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::SettingsTitle))
        .description(description)
        .color(ctx.data().config.colors.neutral);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
