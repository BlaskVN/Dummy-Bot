// AutoMod Rhai Rule Module
// Controls word filtering, toxic pattern detection, and moderation suggestion dispatching

fn inspect_message(content, author_id, guild_id) {
    log_info("AutoMod rule evaluating message from user " + author_id + " in guild " + guild_id);
    
    let lower_content = content.to_lower();
    
    // Check for blocked words or patterns
    if lower_content.contains("badword_test") {
        log_warn("AutoMod detected blocked content in guild " + guild_id);
        return #{
            action: "flag_suggestion",
            rule_id: "blocked_words",
            reason: "Detected blocked content",
            should_warn: true
        };
    }

    return #{
        action: "pass"
    };
}
