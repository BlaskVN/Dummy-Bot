// Word Puzzle Rhai Module v3.0.0
// Controls Wordle-like mini-game rules, guess evaluation, scoring, and puzzle state rules

fn evaluate_guess(target_word, guess) {
    let target = target_word.to_upper();
    let user_guess = guess.to_upper();
    
    if user_guess.len() != 5 {
        return #{
            valid: false,
            error: "Guess must be exactly 5 letters"
        };
    }
    
    let result = [];
    for i in 0..5 {
        let char_t = target.sub_string(i, 1);
        let char_g = user_guess.sub_string(i, 1);
        
        if char_g == char_t {
            result.push("correct");
        } else if target.contains(char_g) {
            result.push("present");
        } else {
            result.push("absent");
        }
    }
    
    let is_solved = (user_guess == target);
    
    return #{
        valid: true,
        solved: is_solved,
        matches: result
    };
}
