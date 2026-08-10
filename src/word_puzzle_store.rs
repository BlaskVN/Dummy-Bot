use crate::word_puzzle::{LetterMark, Puzzle, PuzzleState};
use anyhow::{Result, anyhow, bail};
use poise::serenity_prelude::{GuildId, UserId};
use sqlx::{Sqlite, SqlitePool, Transaction};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Session {
    pub id: i64,
    pub guild_id: String,
    pub creator_id: String,
    pub answer: String,
    pub created_at: i64,
    pub started_at: Option<i64>,
    pub deadline_at: Option<i64>,
    pub finished_at: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct BoardGuess {
    pub attempt: i64,
    pub word: String,
    pub marks: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Board {
    pub user_id: String,
    pub status: String,
    pub guesses: Vec<BoardGuess>,
}

#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct SummaryEntry {
    pub user_id: String,
    pub status: String,
    pub attempts: i64,
}

#[derive(sqlx::FromRow)]
struct SubmissionSession {
    answer: String,
    started_at: Option<i64>,
    deadline_at: Option<i64>,
    finished_at: Option<i64>,
}

pub async fn create_session(
    pool: &SqlitePool,
    guild_id: GuildId,
    creator_id: UserId,
    now: i64,
) -> Result<Session> {
    let entropy: i64 = sqlx::query_scalar("SELECT random()")
        .fetch_one(pool)
        .await?;
    let answers: Vec<_> = crate::word_set::ANSWERS.lines().collect();
    let answer = answers[entropy.unsigned_abs() as usize % answers.len()];
    let mut transaction = pool.begin().await?;
    let id = sqlx::query("INSERT INTO word_puzzle_session (guild_id, creator_id, answer, created_at) VALUES (?, ?, ?, ?)")
        .bind(guild_id.to_string()).bind(creator_id.to_string()).bind(answer).bind(now)
        .execute(&mut *transaction).await?.last_insert_rowid();
    sqlx::query("INSERT INTO word_puzzle_participant (session_id, user_id) VALUES (?, ?)")
        .bind(id)
        .bind(creator_id.to_string())
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    session(pool, id)
        .await?
        .ok_or_else(|| anyhow!("Created puzzle session is missing"))
}

pub async fn session(pool: &SqlitePool, session_id: i64) -> Result<Option<Session>> {
    Ok(sqlx::query_as("SELECT id, guild_id, creator_id, answer, created_at, started_at, deadline_at, finished_at FROM word_puzzle_session WHERE id = ?")
        .bind(session_id).fetch_optional(pool).await?)
}

pub async fn session_for_guild(pool: &SqlitePool, guild_id: GuildId) -> Result<Option<Session>> {
    Ok(sqlx::query_as("SELECT id, guild_id, creator_id, answer, created_at, started_at, deadline_at, finished_at FROM word_puzzle_session WHERE guild_id = ?")
        .bind(guild_id.to_string()).fetch_optional(pool).await?)
}

pub async fn join(pool: &SqlitePool, session_id: i64, user_id: UserId) -> Result<bool> {
    Ok(sqlx::query("INSERT INTO word_puzzle_participant (session_id, user_id) SELECT id, ? FROM word_puzzle_session WHERE id = ? AND started_at IS NULL AND finished_at IS NULL ON CONFLICT DO NOTHING")
        .bind(user_id.to_string()).bind(session_id).execute(pool).await?.rows_affected() == 1)
}

pub async fn start(
    pool: &SqlitePool,
    session_id: i64,
    creator_id: UserId,
    now: i64,
    duration_seconds: i64,
) -> Result<i64> {
    if duration_seconds <= 0 {
        bail!("Puzzle duration must be positive");
    }
    let deadline = now
        .checked_add(duration_seconds)
        .ok_or_else(|| anyhow!("Puzzle deadline overflow"))?;
    let mut transaction = pool.begin().await?;
    let started = sqlx::query("UPDATE word_puzzle_session SET started_at = ?, deadline_at = ? WHERE id = ? AND creator_id = ? AND started_at IS NULL AND finished_at IS NULL")
        .bind(now).bind(deadline).bind(session_id).bind(creator_id.to_string())
        .execute(&mut *transaction).await?.rows_affected() == 1;
    if !started {
        bail!("Only the creator can start an open puzzle");
    }
    sqlx::query("UPDATE word_puzzle_participant SET status = 'playing' WHERE session_id = ? AND status = 'joined'")
        .bind(session_id).execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(deadline)
}

pub async fn submit_guess(
    pool: &SqlitePool,
    session_id: i64,
    user_id: UserId,
    guess: &str,
    now: i64,
) -> Result<PuzzleState> {
    let mut transaction = pool.begin().await?;
    let row: Option<SubmissionSession> = sqlx::query_as(
        "SELECT answer, started_at, deadline_at, finished_at FROM word_puzzle_session WHERE id = ?",
    )
    .bind(session_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(session) = row else {
        bail!("Puzzle session not found");
    };
    if session.started_at.is_none() || session.finished_at.is_some() {
        bail!("Puzzle is not accepting guesses");
    }
    if session.deadline_at.is_some_and(|deadline| deadline <= now) {
        expire(&mut transaction, session_id, now).await?;
        transaction.commit().await?;
        bail!("Puzzle deadline has passed");
    }
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM word_puzzle_participant WHERE session_id = ? AND user_id = ?",
    )
    .bind(session_id)
    .bind(user_id.to_string())
    .fetch_optional(&mut *transaction)
    .await?;
    if status.as_deref() != Some("playing") {
        bail!("Participant is not accepting guesses");
    }
    let prior: Vec<String> = sqlx::query_scalar(
        "SELECT word FROM word_puzzle_guess WHERE session_id = ? AND user_id = ? ORDER BY attempt",
    )
    .bind(session_id)
    .bind(user_id.to_string())
    .fetch_all(&mut *transaction)
    .await?;
    let mut puzzle = Puzzle::new(&session.answer)
        .map_err(|error| anyhow!("Invalid stored answer: {error:?}"))?;
    for word in prior {
        puzzle
            .submit(&word)
            .map_err(|error| anyhow!("Invalid stored guess: {error:?}"))?;
    }
    let marks = puzzle
        .submit(guess)
        .map_err(|error| anyhow!("Guess rejected: {error:?}"))?;
    let attempt = puzzle.guesses().len() as i64;
    sqlx::query("INSERT INTO word_puzzle_guess (session_id, user_id, attempt, word, marks) VALUES (?, ?, ?, ?, ?)")
        .bind(session_id).bind(user_id.to_string()).bind(attempt).bind(guess).bind(encode_marks(marks))
        .execute(&mut *transaction).await?;
    let state = puzzle.state();
    if state != PuzzleState::Playing {
        let status = if state == PuzzleState::Won {
            "won"
        } else {
            "lost"
        };
        sqlx::query(
            "UPDATE word_puzzle_participant SET status = ? WHERE session_id = ? AND user_id = ?",
        )
        .bind(status)
        .bind(session_id)
        .bind(user_id.to_string())
        .execute(&mut *transaction)
        .await?;
        let playing: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM word_puzzle_participant WHERE session_id = ? AND status = 'playing'")
            .bind(session_id).fetch_one(&mut *transaction).await?;
        if playing == 0 {
            finish(&mut transaction, session_id, now).await?;
        }
    }
    transaction.commit().await?;
    Ok(state)
}

pub async fn board_for(
    pool: &SqlitePool,
    session_id: i64,
    requester: UserId,
    target: UserId,
) -> Result<Board> {
    let finished: Option<Option<i64>> =
        sqlx::query_scalar("SELECT finished_at FROM word_puzzle_session WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?;
    let Some(finished) = finished else {
        bail!("Puzzle session not found");
    };
    if requester != target && finished.is_none() {
        bail!("Another participant's unfinished board is private");
    }
    let status: Option<String> = sqlx::query_scalar(
        "SELECT status FROM word_puzzle_participant WHERE session_id = ? AND user_id = ?",
    )
    .bind(session_id)
    .bind(target.to_string())
    .fetch_optional(pool)
    .await?;
    let Some(status) = status else {
        bail!("Participant not found");
    };
    let guesses = sqlx::query_as("SELECT attempt, word, marks FROM word_puzzle_guess WHERE session_id = ? AND user_id = ? ORDER BY attempt")
        .bind(session_id).bind(target.to_string()).fetch_all(pool).await?;
    Ok(Board {
        user_id: target.to_string(),
        status,
        guesses,
    })
}

pub async fn summary(pool: &SqlitePool, session_id: i64) -> Result<Vec<SummaryEntry>> {
    let finished: Option<i64> =
        sqlx::query_scalar("SELECT finished_at FROM word_puzzle_session WHERE id = ?")
            .bind(session_id)
            .fetch_optional(pool)
            .await?
            .flatten();
    if finished.is_none() {
        bail!("Puzzle result is not ready");
    }
    Ok(sqlx::query_as("SELECT p.user_id, p.status, COUNT(g.attempt) AS attempts FROM word_puzzle_participant p LEFT JOIN word_puzzle_guess g ON g.session_id = p.session_id AND g.user_id = p.user_id WHERE p.session_id = ? GROUP BY p.user_id, p.status ORDER BY p.user_id")
        .bind(session_id).fetch_all(pool).await?)
}

pub async fn reconcile_expired(pool: &SqlitePool, now: i64, limit: i64) -> Result<u64> {
    if limit <= 0 {
        return Ok(0);
    }
    let sessions: Vec<i64> = sqlx::query_scalar("SELECT id FROM word_puzzle_session WHERE finished_at IS NULL AND deadline_at <= ? ORDER BY deadline_at, id LIMIT ?")
        .bind(now).bind(limit).fetch_all(pool).await?;
    for session_id in &sessions {
        let mut transaction = pool.begin().await?;
        expire(&mut transaction, *session_id, now).await?;
        transaction.commit().await?;
    }
    Ok(sessions.len() as u64)
}

pub async fn cleanup_delivered(pool: &SqlitePool, session_id: i64) -> Result<bool> {
    let mut transaction = pool.begin().await?;
    let finished: Option<i64> =
        sqlx::query_scalar("SELECT finished_at FROM word_puzzle_session WHERE id = ?")
            .bind(session_id)
            .fetch_optional(&mut *transaction)
            .await?
            .flatten();
    if finished.is_none() {
        transaction.commit().await?;
        return Ok(false);
    }
    sqlx::query("DELETE FROM word_puzzle_guess WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM word_puzzle_participant WHERE session_id = ?")
        .bind(session_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM word_puzzle_session WHERE id = ?")
        .bind(session_id)
        .execute(&mut *transaction)
        .await?;
    transaction.commit().await?;
    Ok(true)
}

async fn expire(
    transaction: &mut Transaction<'_, Sqlite>,
    session_id: i64,
    now: i64,
) -> Result<()> {
    sqlx::query("UPDATE word_puzzle_participant SET status = 'lost' WHERE session_id = ? AND status IN ('joined', 'playing')")
        .bind(session_id).execute(&mut **transaction).await?;
    finish(transaction, session_id, now).await
}

async fn finish(
    transaction: &mut Transaction<'_, Sqlite>,
    session_id: i64,
    now: i64,
) -> Result<()> {
    sqlx::query(
        "UPDATE word_puzzle_session SET finished_at = ? WHERE id = ? AND finished_at IS NULL",
    )
    .bind(now)
    .bind(session_id)
    .execute(&mut **transaction)
    .await?;
    sqlx::query("INSERT INTO word_puzzle_completion (session_id, guild_id, user_id, completed_at) SELECT s.id, s.guild_id, p.user_id, ? FROM word_puzzle_session s JOIN word_puzzle_participant p ON p.session_id = s.id WHERE s.id = ? ON CONFLICT DO NOTHING")
        .bind(now).bind(session_id).execute(&mut **transaction).await?;
    Ok(())
}

fn encode_marks(marks: [LetterMark; 5]) -> String {
    marks
        .into_iter()
        .map(|mark| match mark {
            LetterMark::Exact => 'X',
            LetterMark::Present => 'P',
            LetterMark::Absent => '-',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        board_for, cleanup_delivered, create_session, join, reconcile_expired, session,
        session_for_guild, start, submit_guess, summary,
    };
    use crate::database::init_db;
    use crate::word_puzzle::PuzzleState;
    use poise::serenity_prelude::{GuildId, UserId};

    async fn test_pool(name: &str) -> (sqlx::SqlitePool, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-word-puzzle-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        (pool, directory)
    }

    #[tokio::test]
    async fn freezes_roster_and_keeps_unfinished_boards_private() {
        let (pool, directory) = test_pool("privacy").await;
        let session = create_session(&pool, GuildId::new(1), UserId::new(2), 100)
            .await
            .unwrap();
        assert!(join(&pool, session.id, UserId::new(3)).await.unwrap());
        start(&pool, session.id, UserId::new(2), 110, 60)
            .await
            .unwrap();
        assert!(!join(&pool, session.id, UserId::new(4)).await.unwrap());
        submit_guess(&pool, session.id, UserId::new(2), "actor", 120)
            .await
            .unwrap();
        assert_eq!(
            board_for(&pool, session.id, UserId::new(2), UserId::new(2))
                .await
                .unwrap()
                .guesses
                .len(),
            1
        );
        assert!(
            board_for(&pool, session.id, UserId::new(3), UserId::new(2))
                .await
                .is_err()
        );
        assert_eq!(
            session_for_guild(&pool, GuildId::new(1))
                .await
                .unwrap()
                .unwrap()
                .answer,
            session.answer
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn restart_preserves_answer_deadline_and_expiry_then_cleanup_keeps_keys() {
        let (pool, directory) = test_pool("restart").await;
        let created = create_session(&pool, GuildId::new(1), UserId::new(2), 100)
            .await
            .unwrap();
        join(&pool, created.id, UserId::new(3)).await.unwrap();
        start(&pool, created.id, UserId::new(2), 110, 60)
            .await
            .unwrap();
        pool.close().await;
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();
        let restored = session(&pool, created.id).await.unwrap().unwrap();
        assert_eq!(
            (restored.answer, restored.deadline_at),
            (created.answer, Some(170))
        );
        assert_eq!(reconcile_expired(&pool, 169, 10).await.unwrap(), 0);
        assert_eq!(reconcile_expired(&pool, 170, 10).await.unwrap(), 1);
        assert!(
            summary(&pool, created.id)
                .await
                .unwrap()
                .iter()
                .all(|entry| entry.status == "lost")
        );
        assert!(cleanup_delivered(&pool, created.id).await.unwrap());
        assert!(session(&pool, created.id).await.unwrap().is_none());
        let completions: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM word_puzzle_completion WHERE session_id = ?")
                .bind(created.id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(completions, 2);
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn persists_same_answer_and_private_guesses_for_all_participants() {
        let (pool, directory) = test_pool("boards").await;
        let created = create_session(&pool, GuildId::new(1), UserId::new(2), 100)
            .await
            .unwrap();
        join(&pool, created.id, UserId::new(3)).await.unwrap();
        start(&pool, created.id, UserId::new(2), 110, 60)
            .await
            .unwrap();
        assert_eq!(
            submit_guess(&pool, created.id, UserId::new(2), &created.answer, 120)
                .await
                .unwrap(),
            PuzzleState::Won
        );
        assert_eq!(
            submit_guess(&pool, created.id, UserId::new(3), &created.answer, 121)
                .await
                .unwrap(),
            PuzzleState::Won
        );
        let result = summary(&pool, created.id).await.unwrap();
        assert_eq!(result.len(), 2);
        assert!(
            result
                .iter()
                .all(|entry| entry.status == "won" && entry.attempts == 1)
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
