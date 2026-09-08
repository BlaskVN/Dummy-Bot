use crate::core::engine::{RhaiRuleEngine, RuleEngine};
use crate::core::models::{RuleContext, RuleDecision};
use poise::serenity_prelude as serenity;
use std::sync::Arc;

pub struct CoreEventBus {
    rule_engine: Arc<RhaiRuleEngine>,
}

impl CoreEventBus {
    pub fn new(rule_engine: Arc<RhaiRuleEngine>) -> Self {
        Self { rule_engine }
    }

    pub async fn dispatch_message(&self, message: &serenity::Message) -> Option<RuleDecision> {
        if message.author.bot {
            return None;
        }

        let guild_id = message
            .guild_id
            .map(|g| g.get().to_string())
            .unwrap_or_default();
        let author_id = message.author.id.get().to_string();
        let content = message.content.clone();

        let ctx = RuleContext {
            content,
            author_id,
            guild_id,
        };

        match self.rule_engine.evaluate_content(&ctx).await {
            Ok(decision) => {
                match &decision {
                    RuleDecision::FlagSuggestion {
                        rule_id,
                        reason,
                        should_warn,
                    } => {
                        tracing::warn!(
                            %rule_id,
                            %reason,
                            should_warn,
                            "AutoMod rule engine flagged message"
                        );
                    }
                    RuleDecision::Pass => {
                        tracing::debug!("AutoMod rule engine inspection passed");
                    }
                }
                Some(decision)
            }
            Err(err) => {
                tracing::error!(error = %err, "Failed to evaluate rule engine for message");
                None
            }
        }
    }

    pub async fn dispatch_voice_state_update(
        &self,
        old: Option<&serenity::VoiceState>,
        new: &serenity::VoiceState,
    ) {
        let user_id = new.user_id.get().to_string();
        let guild_id = new
            .guild_id
            .map(|g| g.get().to_string())
            .unwrap_or_default();

        let old_channel = old.and_then(|v| v.channel_id).map(|c| c.get().to_string());
        let new_channel = new.channel_id.map(|c| c.get().to_string());

        tracing::trace!(
            user_id = %user_id,
            guild_id = %guild_id,
            ?old_channel,
            ?new_channel,
            "Dispatched voice state update to Rhai core bus"
        );
    }
}
