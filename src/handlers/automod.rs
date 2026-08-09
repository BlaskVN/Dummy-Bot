use crate::Data;
use crate::automod::{
    ExecutionMetadata, mark_suggestion_delivery, maybe_open_suggestion, observer_enabled,
    record_execution, resolve_rule_suggestions,
};
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
    let observed_at = chrono::Utc::now().timestamp();
    match record_execution(&data.db_pool, &metadata, observed_at).await {
        Ok(true) => {}
        Ok(false) => return,
        Err(error) => {
            tracing::error!(guild = %execution.guild_id, rule = %execution.rule_id, %error, "Failed to store AutoMod execution");
            return;
        }
    }
    let suggestion = match maybe_open_suggestion(
        &data.db_pool,
        execution.guild_id,
        execution.user_id.get(),
        execution.rule_id.get(),
        observed_at,
    )
    .await
    {
        Ok(suggestion) => suggestion,
        Err(error) => {
            tracing::error!(guild = %execution.guild_id, rule = %execution.rule_id, %error, "Failed to evaluate AutoMod suggestion");
            None
        }
    };
    if let Some(suggestion) = suggestion {
        let _ = send_suggestion(
            ctx,
            data,
            suggestion,
            execution.guild_id,
            execution.user_id,
            execution.rule_id,
        )
        .await;
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
        match resolve_rule_suggestions(
            &data.db_pool,
            rule.guild_id,
            rule.id.get(),
            chrono::Utc::now().timestamp(),
        )
        .await
        {
            Ok(resolved) => {
                tracing::debug!(guild = %rule.guild_id, rule = %rule.id, resolved, "Resolved AutoMod suggestions after rule update")
            }
            Err(error) => {
                tracing::error!(guild = %rule.guild_id, rule = %rule.id, %error, "Failed to resolve AutoMod suggestions")
            }
        }
    }
}

pub async fn send_suggestion(
    ctx: &serenity::Context,
    data: &Data,
    suggestion_id: i64,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    rule_id: serenity::RuleId,
) -> bool {
    let channel: Option<String> =
        sqlx::query_scalar("SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(&data.db_pool)
            .await
            .ok()
            .flatten();
    let delivered = if let Some(channel) = channel.and_then(|id| id.parse().ok()) {
        let language = data.language(guild_id).await;
        let message = tf(
            language,
            TranslationKey::AutoModSuggestion,
            &[&user_id, &rule_id],
        );
        serenity::ChannelId::new(channel)
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(message)
                    .allowed_mentions(serenity::CreateAllowedMentions::new()),
            )
            .await
            .is_ok()
    } else {
        false
    };
    if let Err(error) = mark_suggestion_delivery(&data.db_pool, suggestion_id, delivered).await {
        tracing::error!(%guild_id, %suggestion_id, %error, "Failed to record AutoMod suggestion delivery");
    }
    delivered
}
