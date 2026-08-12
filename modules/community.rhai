// Community Rhai Module v3.0.0
// Controls Game Session creation, waitlist logic, and community notifications

fn evaluate_waitlist(current_participants, max_places, user_id) {
    if max_places > 0 && current_participants >= max_places {
        return #{
            status: "waitlisted",
            position: current_participants - max_places + 1
        };
    }
    
    return #{
        status: "accepted",
        position: 0
    };
}

fn on_adhoc_session_started(role_id, guild_id, channel_id) {
    log_info("Ad-hoc game session triggered for role " + role_id + " in guild " + guild_id);
}
