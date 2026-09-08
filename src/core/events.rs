use crate::core::engine::RuleEngine;
use crate::core::models::{RuleContext, RuleDecision};
use poise::serenity_prelude as serenity;
use std::sync::Arc;

pub struct CoreEventBus {
    rule_engine: Arc<dyn RuleEngine>,
}

impl CoreEventBus {
    pub fn new(rule_engine: Arc<dyn RuleEngine>) -> Self {
        Self { rule_engine }
    }

    pub async fn dispatch_message(&self, message: &serenity::Message) -> Option<RuleDecision> {
        if message.author.bot {
            return None;
        }

        let ctx = RuleContext {
            content: message.content.clone(),
            author_id: message.author.id,
            guild_id: message.guild_id,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::engine::MockRuleEngine;
    use poise::serenity_prelude::{GuildId, Message, MessageId, User, UserId};

    #[tokio::test]
    async fn core_event_bus_dispatches_clean_message_via_mock_engine() {
        let mock = Arc::new(MockRuleEngine::new(RuleDecision::Pass));
        let bus = CoreEventBus::new(mock);

        let mut message = Message::default();
        message.id = MessageId::new(100);
        message.author = User::default();
        message.author.id = UserId::new(200);
        message.author.bot = false;
        message.guild_id = Some(GuildId::new(300));
        message.content = "hello world".to_string();

        let decision = bus.dispatch_message(&message).await;
        assert_eq!(decision, Some(RuleDecision::Pass));
    }

    #[tokio::test]
    async fn core_event_bus_ignores_bot_messages() {
        let mock = Arc::new(MockRuleEngine::new(RuleDecision::FlagSuggestion {
            rule_id: "rule".into(),
            reason: "bad".into(),
            should_warn: true,
        }));
        let bus = CoreEventBus::new(mock);

        let mut message = Message::default();
        message.author.bot = true;

        let decision = bus.dispatch_message(&message).await;
        assert_eq!(decision, None);
    }
}
