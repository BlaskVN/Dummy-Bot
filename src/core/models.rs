use poise::serenity_prelude::{GuildId, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleContext {
    pub content: String,
    pub author_id: UserId,
    pub guild_id: GuildId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleDecision {
    Pass,
    FlagSuggestion {
        rule_id: String,
        reason: String,
        should_warn: bool,
    },
}
