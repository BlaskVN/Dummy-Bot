use crate::i18n::{TranslationKey, t, tf};
use crate::message_log_health::{MessageLogHealth, mark_warning_sent, reconcile};
use crate::permissions::missing_channel_permissions;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Parent command for message logging management.
#[poise::command(
    slash_command,
    subcommands("enable", "disable", "status"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn messagelog(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enable message logging for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn enable(
    ctx: Context<'_>,
    #[description = "Channel to send message logs to"] log_channel: serenity::GuildChannel,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    let required = serenity::Permissions::VIEW_CHANNEL
        | serenity::Permissions::SEND_MESSAGES
        | serenity::Permissions::EMBED_LINKS
        | serenity::Permissions::ATTACH_FILES;
    let missing =
        missing_channel_permissions(ctx, log_channel.id, ctx.cache().current_user().id, required)?;
    if !missing.is_empty() {
        let message = tf(
            lang,
            TranslationKey::ModerationBotMissingPermissions,
            &[&missing],
        );
        ctx.say(message).await?;
        return Ok(());
    }

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

    let (_, warn) = reconcile(
        &ctx.data().db_pool,
        guild_id,
        ctx.data().config.message_content_enabled,
    )
    .await?;
    if warn
        && log_channel
            .id
            .send_message(
                ctx.http(),
                serenity::CreateMessage::new()
                    .content(t(lang, TranslationKey::MessageLogDegradedWarning))
                    .allowed_mentions(serenity::CreateAllowedMentions::new()),
            )
            .await
            .is_ok()
    {
        mark_warning_sent(&ctx.data().db_pool, guild_id).await?;
    }

    tracing::info!(
        guild = %guild_id,
        channel = %log_channel.id,
        admin = %ctx.author().name,
        "Message logging enabled"
    );

    let message = tf(lang, TranslationKey::MessageLogEnabled, &[&log_channel.id]);

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(ctx.data().config.colors.success);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Disable message logging for this server.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    // Update config to disabled
    let result = sqlx::query("UPDATE message_log_config SET enabled = 0 WHERE guild_id = ?")
        .bind(guild_id.to_string())
        .execute(&ctx.data().db_pool)
        .await?;

    if result.rows_affected() == 0 {
        let embed = serenity::CreateEmbed::new()
            .description(t(lang, TranslationKey::MessageLogNotSetup))
            .color(ctx.data().config.colors.warning);
        ctx.send(poise::CreateReply::default().embed(embed)).await?;
        return Ok(());
    }
    reconcile(
        &ctx.data().db_pool,
        guild_id,
        ctx.data().config.message_content_enabled,
    )
    .await?;

    tracing::info!(
        guild = %guild_id,
        admin = %ctx.author().name,
        "Message logging disabled"
    );

    let embed = serenity::CreateEmbed::new()
        .description(t(lang, TranslationKey::MessageLogDisabled))
        .color(ctx.data().config.colors.success);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Show current message logging status.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    let config = sqlx::query_as::<_, (String, i64, String)>(
        "SELECT log_channel_id, enabled, health FROM message_log_config WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?;

    match config {
        Some((channel_id, enabled, health)) => {
            let status = if enabled == 1 {
                t(lang, TranslationKey::MessageLogStatusEnabled)
            } else {
                t(lang, TranslationKey::MessageLogStatusDisabled)
            };

            let status_label = t(lang, TranslationKey::MessageLogStatus);
            let channel_text = tf(lang, TranslationKey::MessageLogChannel, &[&channel_id]);
            let health = t(
                lang,
                match MessageLogHealth::parse(&health) {
                    MessageLogHealth::Disabled => TranslationKey::MessageLogHealthDisabled,
                    MessageLogHealth::Healthy => TranslationKey::MessageLogHealthHealthy,
                    MessageLogHealth::Degraded => TranslationKey::MessageLogHealthDegraded,
                },
            );
            let health_text = tf(lang, TranslationKey::MessageLogHealth, &[&health]);

            let description = format!(
                "├ {} {}\n├ {}\n└ {}",
                status_label, status, channel_text, health_text
            );

            let embed = serenity::CreateEmbed::new()
                .title(t(lang, TranslationKey::MessageLogStatusTitle))
                .description(description)
                .color(ctx.data().config.colors.primary);

            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
        None => {
            let embed = serenity::CreateEmbed::new()
                .description(t(lang, TranslationKey::MessageLogUseEnable))
                .color(ctx.data().config.colors.warning);
            ctx.send(poise::CreateReply::default().embed(embed)).await?;
        }
    }

    Ok(())
}
