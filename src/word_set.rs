pub const ANSWERS: &str = include_str!("../assets/word_puzzle/answers.txt");
pub const ALLOWED: &str = include_str!("../assets/word_puzzle/allowed.txt");

pub fn is_allowed(word: &str) -> bool {
    ALLOWED.lines().any(|allowed| allowed == word)
}

#[cfg(test)]
mod tests {
    use super::{ALLOWED, ANSWERS};
    use std::collections::HashSet;

    #[test]
    fn word_set_is_normalized_unique_and_answers_are_allowed() {
        let validate = |contents: &'static str| {
            let words: Vec<_> = contents.lines().collect();
            assert!(!words.is_empty());
            assert!(words.iter().all(|word| {
                word.len() == 5 && word.bytes().all(|byte| byte.is_ascii_lowercase())
            }));
            let unique: HashSet<_> = words.iter().copied().collect();
            assert_eq!(unique.len(), words.len());
            unique
        };
        let allowed = validate(ALLOWED);
        let answers = validate(ANSWERS);
        assert!(answers.len() < allowed.len());
        assert!(answers.is_subset(&allowed));
    }
}
