use crate::Data;
use crate::automod::{ExecutionMetadata, observer_enabled, record_execution};
use crate::i18n::{TranslationKey, t, tf};
use poise::serenity_prelude as serenity;

pub async fn handle_execution(
    ctx: &serenity::Context,
    execution: &serenity::ActionExecution,
    data: &Data,
) {
    if execution.user_id == ctx.cache.current_user().id {
        return;
    }
    match observer_enabled(&data.db_pool, execution.guild_id).await {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::error!(guild = %execution.guild_id, %error, "Failed to load AutoMod observer configuration");
            return;
        }
    }
    let metadata = ExecutionMetadata {
        guild_id: execution.guild_id,
        user_id: execution.user_id.get(),
        rule_id: execution.rule_id.get(),
        action_type: u8::from(execution.action.kind()),
        channel_id: execution.channel_id.map(|id| id.get()),
        message_id: execution.message_id.map(|id| id.get()),
        alert_message_id: execution.alert_system_message_id.map(|id| id.get()),
    };
    match record_execution(&data.db_pool, &metadata, chrono::Utc::now().timestamp()).await {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::error!(guild = %execution.guild_id, rule = %execution.rule_id, %error, "Failed to store AutoMod execution");
            return;
        }
    }
    let channel: Option<String> = match sqlx::query_scalar(
        "SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?",
    )
    .bind(execution.guild_id.to_string())
    .fetch_optional(&data.db_pool)
    .await
    {
        Ok(channel) => channel,
        Err(error) => {
            tracing::error!(guild = %execution.guild_id, %error, "Failed to load moderation channel");
            return;
        }
    };
    let Some(channel) = channel else { return };
    let Ok(channel) = channel.parse() else {
        tracing::error!(guild = %execution.guild_id, "Invalid stored moderation channel ID");
        return;
    };
    let language = data.language(execution.guild_id).await;
    let jump = match (execution.channel_id, execution.message_id) {
        (Some(channel), Some(message)) => format!(
            "https://discord.com/channels/{}/{channel}/{message}",
            execution.guild_id
        ),
        _ => t(language, TranslationKey::SettingsNotConfigured).to_owned(),
    };
    let message = tf(
        language,
        TranslationKey::AutoModExecution,
        &[
            &execution.rule_id,
            &execution.user_id,
            &metadata.action_type,
            &jump,
        ],
    );
    if let Err(error) = serenity::ChannelId::new(channel)
        .send_message(
            &ctx.http,
            serenity::CreateMessage::new()
                .content(message)
                .allowed_mentions(serenity::CreateAllowedMentions::new()),
        )
        .await
    {
        tracing::warn!(guild = %execution.guild_id, rule = %execution.rule_id, %error, "Failed to notify moderators of AutoMod execution");
    }
}

pub async fn handle_rule_update(rule: &serenity::Rule, data: &Data) {
    if observer_enabled(&data.db_pool, rule.guild_id)
        .await
        .unwrap_or(false)
    {
        tracing::debug!(guild = %rule.guild_id, rule = %rule.id, "Received observed AutoMod rule update");
    }
}
