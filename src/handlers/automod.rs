use crate::Data;
use crate::automod::observer_enabled;
use poise::serenity_prelude as serenity;

pub async fn handle_execution(execution: &serenity::ActionExecution, data: &Data) {
    if observer_enabled(&data.db_pool, execution.guild_id)
        .await
        .unwrap_or(false)
    {
        tracing::debug!(guild = %execution.guild_id, rule = %execution.rule_id, "Received observed AutoMod execution");
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
