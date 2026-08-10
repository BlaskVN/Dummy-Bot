#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterMark {
    Exact,
    Present,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PuzzleState {
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuessError {
    Finished,
    Malformed,
    NotAllowed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Guess {
    pub word: String,
    pub marks: [LetterMark; 5],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Puzzle {
    answer: [u8; 5],
    guesses: Vec<Guess>,
    state: PuzzleState,
}

impl Puzzle {
    pub fn new(answer: &str) -> Result<Self, GuessError> {
        Ok(Self {
            answer: parse_word(answer)?,
            guesses: Vec::with_capacity(6),
            state: PuzzleState::Playing,
        })
    }

    pub fn submit(&mut self, guess: &str) -> Result<[LetterMark; 5], GuessError> {
        if self.state != PuzzleState::Playing {
            return Err(GuessError::Finished);
        }
        let guess_bytes = parse_word(guess)?;
        if !crate::word_set::is_allowed(guess) {
            return Err(GuessError::NotAllowed);
        }
        let marks = score(self.answer, guess_bytes);
        self.guesses.push(Guess {
            word: guess.to_owned(),
            marks,
        });
        self.state = if marks.iter().all(|mark| *mark == LetterMark::Exact) {
            PuzzleState::Won
        } else if self.guesses.len() == 6 {
            PuzzleState::Lost
        } else {
            PuzzleState::Playing
        };
        Ok(marks)
    }

    pub fn guesses(&self) -> &[Guess] {
        &self.guesses
    }

    pub const fn state(&self) -> PuzzleState {
        self.state
    }
}

fn parse_word(word: &str) -> Result<[u8; 5], GuessError> {
    let bytes: [u8; 5] = word
        .as_bytes()
        .try_into()
        .map_err(|_| GuessError::Malformed)?;
    bytes
        .iter()
        .all(u8::is_ascii_lowercase)
        .then_some(bytes)
        .ok_or(GuessError::Malformed)
}

fn score(answer: [u8; 5], guess: [u8; 5]) -> [LetterMark; 5] {
    let mut marks = [LetterMark::Absent; 5];
    let mut remaining = [0_u8; 26];
    for index in 0..5 {
        if answer[index] == guess[index] {
            marks[index] = LetterMark::Exact;
        } else {
            remaining[(answer[index] - b'a') as usize] += 1;
        }
    }
    for index in 0..5 {
        if marks[index] == LetterMark::Exact {
            continue;
        }
        let count = &mut remaining[(guess[index] - b'a') as usize];
        if *count > 0 {
            marks[index] = LetterMark::Present;
            *count -= 1;
        }
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::{GuessError, LetterMark, Puzzle, PuzzleState, score};

    #[test]
    fn scores_exact_absent_and_duplicate_letters() {
        use LetterMark::{Absent as A, Exact as E, Present as P};
        assert_eq!(score(*b"apple", *b"apple"), [E, E, E, E, E]);
        assert_eq!(score(*b"apple", *b"brick"), [A, A, A, A, A]);
        assert_eq!(score(*b"apple", *b"adapt"), [E, A, A, P, A]);
        assert_eq!(score(*b"apple", *b"allee"), [E, P, A, A, E]);
    }

    #[test]
    fn invalid_guesses_do_not_consume_attempts_and_win_finishes() {
        let mut puzzle = Puzzle::new("apple").unwrap();
        assert_eq!(puzzle.submit("NOPE!"), Err(GuessError::Malformed));
        assert_eq!(puzzle.submit("zzzzz"), Err(GuessError::NotAllowed));
        assert!(puzzle.guesses().is_empty());
        assert!(puzzle.submit("apple").is_ok());
        assert_eq!(puzzle.state(), PuzzleState::Won);
        assert_eq!(puzzle.submit("actor"), Err(GuessError::Finished));
    }

    #[test]
    fn sixth_valid_guess_loses_and_stops_play() {
        let mut puzzle = Puzzle::new("apple").unwrap();
        for guess in ["actor", "adore", "after", "agile", "alarm", "album"] {
            puzzle.submit(guess).unwrap();
        }
        assert_eq!(puzzle.state(), PuzzleState::Lost);
        assert_eq!(puzzle.guesses().len(), 6);
        assert_eq!(puzzle.submit("apple"), Err(GuessError::Finished));
    }
}
