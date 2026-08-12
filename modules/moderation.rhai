// Moderation Rhai Module v3.0.0
// Controls moderation case lifecycle (warn, timeout, kick, ban, void case)

fn evaluate_moderation_action(action_type, guild_id, target_user_id, reason) {
    log_info("Evaluating moderation action " + action_type + " for user " + target_user_id + " in guild " + guild_id);
    
    if reason.is_empty() {
        return #{
            allowed: false,
            message: "Reason cannot be empty for moderation actions"
        };
    }
    
    return #{
        allowed: true,
        action: action_type,
        audit_reason: reason
    };
}

fn on_case_created(case_number, guild_id, target_user_id, action_type, reason) {
    log_info("Moderation case #" + case_number + " created in guild " + guild_id + " for user " + target_user_id);
}
