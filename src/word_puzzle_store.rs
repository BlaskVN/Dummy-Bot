use crate::word_puzzle::{LetterMark, Puzzle, PuzzleState};
use anyhow::{Result, anyhow, bail};
use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use sqlx::{Sqlite, SqlitePool, Transaction};

const UNSTARTED_SESSION_TIMEOUT_SECONDS: i64 = 60 * 60;

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

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct FinishedSession {
    pub id: i64,
    pub guild_id: String,
    pub answer: String,
    pub result_channel_id: String,
}

#[derive(sqlx::FromRow)]
struct SubmissionSession {
    answer: String,
    started_at: Option<i64>,
    deadline_at: Option<i64>,
    finished_at: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct PendingCredit {
    session_id: i64,
    guild_id: String,
    user_id: String,
    completed_at: i64,
    iana_name: Option<String>,
}

pub async fn create_session(
    pool: &SqlitePool,
    guild_id: GuildId,
    creator_id: UserId,
    result_channel_id: ChannelId,
    now: i64,
) -> Result<Session> {
    let entropy: i64 = sqlx::query_scalar("SELECT random()")
        .fetch_one(pool)
        .await?;
    let answers: Vec<_> = crate::word_set::ANSWERS.lines().collect();
    let answer = answers[entropy.unsigned_abs() as usize % answers.len()];
    let mut transaction = pool.begin().await?;
    let id = sqlx::query("INSERT INTO word_puzzle_session (guild_id, creator_id, answer, created_at, result_channel_id) VALUES (?, ?, ?, ?, ?)")
        .bind(guild_id.to_string()).bind(creator_id.to_string()).bind(answer).bind(now).bind(result_channel_id.to_string())
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
    delivery_key: &str,
) -> Result<PuzzleState> {
    let mut transaction = pool.begin().await?;
    if let Some(outcome) = sqlx::query_scalar::<_, String>(
        "SELECT outcome FROM word_puzzle_interaction WHERE delivery_key = ?",
    )
    .bind(delivery_key)
    .fetch_optional(&mut *transaction)
    .await?
    {
        transaction.commit().await?;
        return parse_state(&outcome);
    }
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
    sqlx::query("INSERT INTO word_puzzle_interaction (delivery_key, guild_id, session_id, outcome, created_at) SELECT ?, guild_id, id, ?, ? FROM word_puzzle_session WHERE id = ?")
        .bind(delivery_key).bind(state_name(state)).bind(now).bind(session_id)
        .execute(&mut *transaction).await?;
    transaction.commit().await?;
    Ok(state)
}

pub async fn finish_now(
    pool: &SqlitePool,
    session_id: i64,
    creator_id: UserId,
    now: i64,
) -> Result<bool> {
    let mut transaction = pool.begin().await?;
    let deadline: Option<i64> = sqlx::query_scalar("SELECT deadline_at FROM word_puzzle_session WHERE id = ? AND creator_id = ? AND started_at IS NOT NULL AND finished_at IS NULL")
        .bind(session_id).bind(creator_id.to_string()).fetch_optional(&mut *transaction).await?.flatten();
    let allowed = deadline.is_some_and(|deadline| deadline <= now);
    if allowed {
        expire(&mut transaction, session_id, now).await?;
    }
    transaction.commit().await?;
    Ok(allowed)
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
    let stale: Vec<i64> = sqlx::query_scalar("SELECT id FROM word_puzzle_session WHERE started_at IS NULL AND finished_at IS NULL AND created_at <= ? ORDER BY created_at, id LIMIT ?")
        .bind(now.saturating_sub(UNSTARTED_SESSION_TIMEOUT_SECONDS)).bind(limit).fetch_all(pool).await?;
    let mut removed = 0_u64;
    for session_id in stale {
        let mut transaction = pool.begin().await?;
        sqlx::query("DELETE FROM word_puzzle_participant WHERE session_id = ? AND EXISTS (SELECT 1 FROM word_puzzle_session WHERE id = ? AND started_at IS NULL AND finished_at IS NULL AND created_at <= ?)")
            .bind(session_id).bind(session_id).bind(now.saturating_sub(UNSTARTED_SESSION_TIMEOUT_SECONDS))
            .execute(&mut *transaction).await?;
        removed += sqlx::query("DELETE FROM word_puzzle_session WHERE id = ? AND started_at IS NULL AND finished_at IS NULL AND created_at <= ?")
            .bind(session_id).bind(now.saturating_sub(UNSTARTED_SESSION_TIMEOUT_SECONDS))
            .execute(&mut *transaction).await?.rows_affected();
        transaction.commit().await?;
    }
    let remaining = limit.saturating_sub(removed as i64);
    let sessions: Vec<i64> = sqlx::query_scalar("SELECT id FROM word_puzzle_session WHERE started_at IS NOT NULL AND finished_at IS NULL AND deadline_at <= ? ORDER BY deadline_at, id LIMIT ?")
        .bind(now).bind(remaining).fetch_all(pool).await?;
    for session_id in &sessions {
        let mut transaction = pool.begin().await?;
        expire(&mut transaction, *session_id, now).await?;
        transaction.commit().await?;
    }
    Ok(removed + sessions.len() as u64)
}

pub async fn award_pending_credits(pool: &SqlitePool, now: i64, limit: i64) -> Result<u64> {
    if limit <= 0 {
        return Ok(0);
    }
    let pending: Vec<PendingCredit> = sqlx::query_as("SELECT c.session_id, c.guild_id, c.user_id, c.completed_at, t.iana_name FROM word_puzzle_completion c LEFT JOIN guild_timezone t ON t.guild_id = c.guild_id WHERE c.credit_processed_at IS NULL ORDER BY c.completed_at, c.session_id, c.user_id LIMIT ?")
        .bind(limit).fetch_all(pool).await?;
    let mut awarded = 0;
    for completion in pending {
        let guild = completion.guild_id.parse::<u64>().ok().map(GuildId::new);
        let user = completion.user_id.parse::<u64>().ok().map(UserId::new);
        let timezone = completion
            .iana_name
            .as_deref()
            .and_then(crate::timezone::parse);
        if let (Some(guild), Some(user), Some(timezone), Some(completed)) = (
            guild,
            user,
            timezone,
            chrono::DateTime::from_timestamp(completion.completed_at, 0),
        ) && !crate::activity_privacy::is_opted_out(pool, guild, user).await?
        {
            let day = completed.with_timezone(&timezone).date_naive();
            let source = format!("word-puzzle-day:{day}");
            if crate::activity_aggregate::add_session_credit(
                pool,
                guild,
                user,
                "word-puzzle",
                &source,
            )
            .await?
            {
                awarded += 1;
            }
        }
        sqlx::query("UPDATE word_puzzle_completion SET credit_processed_at = ? WHERE session_id = ? AND user_id = ? AND credit_processed_at IS NULL")
            .bind(now).bind(completion.session_id).bind(&completion.user_id).execute(pool).await?;
    }
    Ok(awarded)
}

pub async fn claim_finished(
    pool: &SqlitePool,
    now: i64,
    limit: i64,
) -> Result<Vec<FinishedSession>> {
    if limit <= 0 {
        return Ok(Vec::new());
    }
    let candidates: Vec<i64> = sqlx::query_scalar("SELECT id FROM word_puzzle_session WHERE finished_at IS NOT NULL AND result_channel_id IS NOT NULL AND (summary_claimed_at IS NULL OR summary_claimed_at <= ?) ORDER BY finished_at, id LIMIT ?")
        .bind(now - 300).bind(limit).fetch_all(pool).await?;
    let mut claimed = Vec::new();
    for session_id in candidates {
        let mut transaction = pool.begin().await?;
        let won = sqlx::query("UPDATE word_puzzle_session SET summary_claimed_at = ? WHERE id = ? AND (summary_claimed_at IS NULL OR summary_claimed_at <= ?)")
            .bind(now).bind(session_id).bind(now - 300).execute(&mut *transaction).await?.rows_affected() == 1;
        if won && let Some(session) = sqlx::query_as(
            "SELECT id, guild_id, answer, result_channel_id FROM word_puzzle_session WHERE id = ?",
        )
        .bind(session_id)
        .fetch_optional(&mut *transaction)
        .await?
        {
            claimed.push(session);
        }
        transaction.commit().await?;
    }
    Ok(claimed)
}

pub async fn release_summary_claim(pool: &SqlitePool, session_id: i64) -> Result<()> {
    sqlx::query("UPDATE word_puzzle_session SET summary_claimed_at = NULL WHERE id = ?")
        .bind(session_id)
        .execute(pool)
        .await?;
    Ok(())
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

const fn state_name(state: PuzzleState) -> &'static str {
    match state {
        PuzzleState::Playing => "playing",
        PuzzleState::Won => "won",
        PuzzleState::Lost => "lost",
    }
}

fn parse_state(state: &str) -> Result<PuzzleState> {
    match state {
        "playing" => Ok(PuzzleState::Playing),
        "won" => Ok(PuzzleState::Won),
        "lost" => Ok(PuzzleState::Lost),
        _ => bail!("Invalid stored puzzle outcome"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        award_pending_credits, board_for, cleanup_delivered, create_session, finish_now, join,
        reconcile_expired, session, session_for_guild, start, submit_guess, summary,
    };
    use crate::database::init_db;
    use crate::word_puzzle::PuzzleState;
    use poise::serenity_prelude::{ChannelId, GuildId, UserId};

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
        let session = create_session(
            &pool,
            GuildId::new(1),
            UserId::new(2),
            ChannelId::new(9),
            100,
        )
        .await
        .unwrap();
        assert!(join(&pool, session.id, UserId::new(3)).await.unwrap());
        start(&pool, session.id, UserId::new(2), 110, 60)
            .await
            .unwrap();
        assert!(
            !finish_now(&pool, session.id, UserId::new(2), 120)
                .await
                .unwrap()
        );
        assert!(!join(&pool, session.id, UserId::new(4)).await.unwrap());
        submit_guess(&pool, session.id, UserId::new(2), "actor", 120, "privacy-1")
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
        let created = create_session(
            &pool,
            GuildId::new(1),
            UserId::new(2),
            ChannelId::new(9),
            100,
        )
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
    async fn stale_unstarted_session_releases_guild_slot() {
        let (pool, directory) = test_pool("stale-open").await;
        let created = create_session(
            &pool,
            GuildId::new(1),
            UserId::new(2),
            ChannelId::new(9),
            100,
        )
        .await
        .unwrap();

        assert_eq!(reconcile_expired(&pool, 3_699, 10).await.unwrap(), 0);
        assert!(session(&pool, created.id).await.unwrap().is_some());
        assert_eq!(reconcile_expired(&pool, 3_700, 10).await.unwrap(), 1);
        assert!(session(&pool, created.id).await.unwrap().is_none());
        assert!(
            create_session(
                &pool,
                GuildId::new(1),
                UserId::new(3),
                ChannelId::new(9),
                3_700,
            )
            .await
            .is_ok()
        );

        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn persists_same_answer_and_private_guesses_for_all_participants() {
        let (pool, directory) = test_pool("boards").await;
        sqlx::query("INSERT INTO guild_timezone (guild_id, iana_name) VALUES ('1', 'UTC')")
            .execute(&pool)
            .await
            .unwrap();
        let created = create_session(
            &pool,
            GuildId::new(1),
            UserId::new(2),
            ChannelId::new(9),
            100,
        )
        .await
        .unwrap();
        sqlx::query("UPDATE word_puzzle_session SET answer = 'apple' WHERE id = ?")
            .bind(created.id)
            .execute(&pool)
            .await
            .unwrap();
        join(&pool, created.id, UserId::new(3)).await.unwrap();
        start(&pool, created.id, UserId::new(2), 110, 60)
            .await
            .unwrap();
        assert_eq!(
            submit_guess(&pool, created.id, UserId::new(2), "apple", 120, "boards-1",)
                .await
                .unwrap(),
            PuzzleState::Won
        );
        for (index, guess) in ["actor", "adore", "after", "agile", "alarm", "album"]
            .into_iter()
            .enumerate()
        {
            let state = submit_guess(
                &pool,
                created.id,
                UserId::new(3),
                guess,
                121,
                &format!("boards-{}", index + 2),
            )
            .await
            .unwrap();
            assert_eq!(
                state,
                if index == 5 {
                    PuzzleState::Lost
                } else {
                    PuzzleState::Playing
                }
            );
        }
        assert_eq!(
            submit_guess(&pool, created.id, UserId::new(3), "album", 121, "boards-7",)
                .await
                .unwrap(),
            PuzzleState::Lost
        );
        let result = summary(&pool, created.id).await.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!((result[0].status.as_str(), result[0].attempts), ("won", 1));
        assert_eq!((result[1].status.as_str(), result[1].attempts), ("lost", 6));
        assert_eq!(award_pending_credits(&pool, 130, 10).await.unwrap(), 2);
        let credits: Vec<i64> = sqlx::query_scalar("SELECT session_credits FROM activity_member_aggregate WHERE guild_id = '1' ORDER BY user_id")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(credits, [1, 1]);
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn credits_once_per_local_calendar_day_without_play_time() {
        use chrono::TimeZone;

        let (pool, directory) = test_pool("credit").await;
        sqlx::query("INSERT INTO guild_timezone (guild_id, iana_name) VALUES ('1', 'Asia/Bangkok'), ('2', 'America/New_York')")
            .execute(&pool).await.unwrap();
        crate::activity_privacy::opt_out(&pool, GuildId::new(1), UserId::new(5))
            .await
            .unwrap();
        let bangkok = crate::timezone::parse("Asia/Bangkok").unwrap();
        let at = |day, hour, minute| {
            bangkok
                .with_ymd_and_hms(2026, 1, day, hour, minute, 0)
                .unwrap()
                .timestamp()
        };
        let rows = [
            (1, "1", "2", at(1, 4, 59)),
            (2, "1", "2", at(1, 5, 0)),
            (3, "1", "3", at(1, 5, 0)),
            (4, "2", "2", at(1, 5, 0)),
            (5, "1", "4", at(1, 23, 59)),
            (6, "1", "4", at(2, 0, 0)),
            (7, "1", "5", at(1, 5, 0)),
        ];
        for (session_id, guild, user, completed_at) in rows {
            sqlx::query("INSERT INTO word_puzzle_completion (session_id, guild_id, user_id, completed_at) VALUES (?, ?, ?, ?)")
                .bind(session_id).bind(guild).bind(user).bind(completed_at).execute(&pool).await.unwrap();
        }
        assert_eq!(
            award_pending_credits(&pool, at(2, 1, 0), 100)
                .await
                .unwrap(),
            5
        );
        assert_eq!(
            award_pending_credits(&pool, at(2, 1, 1), 100)
                .await
                .unwrap(),
            0
        );
        let aggregates: Vec<(String, String, i64, i64)> = sqlx::query_as("SELECT guild_id, user_id, play_minutes, session_credits FROM activity_member_aggregate ORDER BY guild_id, user_id")
            .fetch_all(&pool).await.unwrap();
        assert_eq!(
            aggregates,
            vec![
                ("1".to_owned(), "2".to_owned(), 0, 1),
                ("1".to_owned(), "3".to_owned(), 0, 1),
                ("1".to_owned(), "4".to_owned(), 0, 2),
                ("2".to_owned(), "2".to_owned(), 0, 1),
            ]
        );
        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
