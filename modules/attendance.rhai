// Attendance & Activity Rewards Rhai Module v3.0.0
// Controls voice attendance duration rules, Session Credit calculation, and level tier role rewards

fn calculate_activity_level(total_play_time_minutes) {
    let hours = total_play_time_minutes / 60;
    // Level n requires n*(n+1)/2 total hours
    // n^2 + n - 2*hours = 0 -> approximate n
    let level = 0;
    let required_hours = 0;
    
    while required_hours <= hours {
        level += 1;
        required_hours = (level * (level + 1)) / 2;
    }
    
    return level - 1;
}

fn evaluate_voice_credit(session_minutes, has_activity_beacon) {
    if !has_activity_beacon {
        return #{
            qualified: false,
            credit_granted: 0,
            reason: "No active activity beacon signal"
        };
    }
    
    if session_minutes < 30 {
        return #{
            qualified: false,
            credit_granted: 0,
            reason: "Session duration under 30 minutes threshold"
        };
    }
    
    return #{
        qualified: true,
        credit_granted: 1,
        play_time_added: session_minutes
    };
}
