#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleContext {
    pub content: String,
    pub author_id: String,
    pub guild_id: String,
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
