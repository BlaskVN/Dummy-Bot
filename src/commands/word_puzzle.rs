use crate::i18n::Language;
use crate::ui::{self, Tone};
use crate::word_puzzle::PuzzleState;
use crate::word_puzzle_store::{Board, FinishedSession, SummaryEntry};
use crate::{Context, Data, Error};
use poise::serenity_prelude as serenity;

/// Play a collaborative five-letter English word puzzle.
#[poise::command(
    rename = "word-puzzle",
    slash_command,
    subcommands("create", "join", "start", "guess", "status", "finish"),
    guild_only
)]
pub async fn word_puzzle(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Create a new word puzzle in this channel.
#[poise::command(slash_command, guild_only)]
pub async fn create(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, language) = guild_context(ctx).await?;
    reconcile_and_deliver(ctx).await;
    match crate::word_puzzle_store::create_session(
        &ctx.data().db_pool,
        guild_id,
        ctx.author().id,
        ctx.channel_id(),
        chrono::Utc::now().timestamp(),
    )
    .await
    {
        Ok(session) => {
            ui::reply(
                ctx,
                Tone::Success,
                render(language, Text::Created, &[&session.id]),
            )
            .await?;
        }
        Err(_) => send_private(ctx, render(language, Text::AlreadyOpen, &[])).await?,
    }
    Ok(())
}

/// Join the current word puzzle in this channel.
#[poise::command(slash_command, guild_only)]
pub async fn join(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, language) = guild_context(ctx).await?;
    reconcile_and_deliver(ctx).await;
    let Some(session) =
        crate::word_puzzle_store::session_for_guild(&ctx.data().db_pool, guild_id).await?
    else {
        send_private(ctx, render(language, Text::NoSession, &[])).await?;
        return Ok(());
    };
    let joined =
        crate::word_puzzle_store::join(&ctx.data().db_pool, session.id, ctx.author().id).await?;
    send_private(
        ctx,
        render(
            language,
            if joined {
                Text::Joined
            } else {
                Text::JoinClosed
            },
            &[],
        ),
    )
    .await?;
    Ok(())
}

/// Start the lobby's puzzle with an optional deadline.
#[poise::command(slash_command, guild_only)]
pub async fn start(
    ctx: Context<'_>,
    #[description = "Deadline in minutes (default 10, max 60)"] minutes: Option<i64>,
) -> Result<(), Error> {
    let (guild_id, language) = guild_context(ctx).await?;
    reconcile_and_deliver(ctx).await;
    let Some(session) =
        crate::word_puzzle_store::session_for_guild(&ctx.data().db_pool, guild_id).await?
    else {
        send_private(ctx, render(language, Text::NoSession, &[])).await?;
        return Ok(());
    };
    let minutes = minutes.unwrap_or(10);
    if !(1..=60).contains(&minutes) {
        send_private(ctx, render(language, Text::DurationInvalid, &[])).await?;
        return Ok(());
    }
    match crate::word_puzzle_store::start(
        &ctx.data().db_pool,
        session.id,
        ctx.author().id,
        chrono::Utc::now().timestamp(),
        minutes * 60,
    )
    .await
    {
        Ok(deadline) => {
            ui::reply(
                ctx,
                Tone::Success,
                render(language, Text::Started, &[&deadline]),
            )
            .await?;
        }
        Err(_) => send_private(ctx, render(language, Text::StartDenied, &[])).await?,
    }
    Ok(())
}

/// Submit one five-letter English word as your guess.
#[poise::command(slash_command, guild_only)]
pub async fn guess(
    ctx: Context<'_>,
    #[description = "One English five-letter word"] word: String,
) -> Result<(), Error> {
    let (guild_id, language) = guild_context(ctx).await?;
    reconcile_and_deliver(ctx).await;
    let Some(session) =
        crate::word_puzzle_store::session_for_guild(&ctx.data().db_pool, guild_id).await?
    else {
        send_private(ctx, render(language, Text::NoSession, &[])).await?;
        return Ok(());
    };
    let word = word.trim().to_ascii_lowercase();
    let now = chrono::Utc::now().timestamp();
    let result = crate::word_puzzle_store::submit_guess(
        &ctx.data().db_pool,
        session.id,
        ctx.author().id,
        &word,
        now,
        &ctx.id().to_string(),
    )
    .await;
    let state = match result {
        Ok(state) => state,
        Err(_) => {
            send_private(ctx, render(language, Text::GuessRejected, &[])).await?;
            return Ok(());
        }
    };
    let board = crate::word_puzzle_store::board_for(
        &ctx.data().db_pool,
        session.id,
        ctx.author().id,
        ctx.author().id,
    )
    .await?;
    let heading = match state {
        PuzzleState::Playing => Text::GuessAccepted,
        PuzzleState::Won => Text::Solved,
        PuzzleState::Lost => Text::Unsolved,
    };
    send_private(
        ctx,
        format!(
            "{}\n{}",
            render(language, heading, &[]),
            format_board(&board)
        ),
    )
    .await?;
    award_and_deliver(ctx.serenity_context(), ctx.data(), now).await;
    Ok(())
}

/// Show the current puzzle, players, guesses, and deadline.
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, language) = guild_context(ctx).await?;
    reconcile_and_deliver(ctx).await;
    let Some(session) =
        crate::word_puzzle_store::session_for_guild(&ctx.data().db_pool, guild_id).await?
    else {
        send_private(ctx, render(language, Text::NoSession, &[])).await?;
        return Ok(());
    };
    match crate::word_puzzle_store::board_for(
        &ctx.data().db_pool,
        session.id,
        ctx.author().id,
        ctx.author().id,
    )
    .await
    {
        Ok(board) => {
            send_private(
                ctx,
                format!(
                    "{}\n{}",
                    render(language, Text::Board, &[]),
                    format_board(&board)
                ),
            )
            .await?;
        }
        Err(_) => send_private(ctx, render(language, Text::NotParticipant, &[])).await?,
    }
    Ok(())
}

/// End the current puzzle early and show its results.
#[poise::command(slash_command, guild_only)]
pub async fn finish(ctx: Context<'_>) -> Result<(), Error> {
    let (guild_id, language) = guild_context(ctx).await?;
    let Some(session) =
        crate::word_puzzle_store::session_for_guild(&ctx.data().db_pool, guild_id).await?
    else {
        send_private(ctx, render(language, Text::NoSession, &[])).await?;
        return Ok(());
    };
    let now = chrono::Utc::now().timestamp();
    if crate::word_puzzle_store::finish_now(&ctx.data().db_pool, session.id, ctx.author().id, now)
        .await?
    {
        send_private(ctx, render(language, Text::Finished, &[])).await?;
        award_and_deliver(ctx.serenity_context(), ctx.data(), now).await;
    } else {
        send_private(ctx, render(language, Text::FinishDenied, &[])).await?;
    }
    Ok(())
}

async fn guild_context(ctx: Context<'_>) -> Result<(serenity::GuildId, Language), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    Ok((guild_id, ctx.data().language(guild_id).await))
}

async fn send_private(ctx: Context<'_>, content: String) -> Result<(), serenity::Error> {
    ui::private_reply(ctx, Tone::Neutral, content).await?;
    Ok(())
}

async fn reconcile_and_deliver(ctx: Context<'_>) {
    reconcile_and_deliver_all(ctx.serenity_context(), ctx.data()).await;
}

pub async fn reconcile_and_deliver_all(ctx: &serenity::Context, data: &Data) {
    let now = chrono::Utc::now().timestamp();
    if let Err(error) = crate::word_puzzle_store::reconcile_expired(&data.db_pool, now, 100).await {
        tracing::error!(%error, "Could not reconcile expired Word Puzzles");
    }
    award_and_deliver(ctx, data, now).await;
}

async fn award_and_deliver(ctx: &serenity::Context, data: &Data, now: i64) {
    if let Err(error) =
        crate::word_puzzle_store::award_pending_credits(&data.db_pool, now, 500).await
    {
        tracing::error!(%error, "Could not award Word Puzzle credits");
    }
    deliver_finished(ctx, data, now).await;
}

async fn deliver_finished(ctx: &serenity::Context, data: &Data, now: i64) {
    let sessions = match crate::word_puzzle_store::claim_finished(&data.db_pool, now, 20).await {
        Ok(sessions) => sessions,
        Err(error) => {
            tracing::error!(%error, "Could not claim Word Puzzle summaries");
            return;
        }
    };
    for session in sessions {
        if deliver_summary(ctx, data, &session).await.is_ok() {
            if let Err(error) =
                crate::word_puzzle_store::cleanup_delivered(&data.db_pool, session.id).await
            {
                tracing::error!(session_id = session.id, %error, "Could not clean delivered Word Puzzle");
            }
        } else if let Err(error) =
            crate::word_puzzle_store::release_summary_claim(&data.db_pool, session.id).await
        {
            tracing::error!(session_id = session.id, %error, "Could not release Word Puzzle summary claim");
        }
    }
}

async fn deliver_summary(
    ctx: &serenity::Context,
    data: &Data,
    session: &FinishedSession,
) -> Result<(), Error> {
    let guild_id = serenity::GuildId::new(session.guild_id.parse()?);
    let channel_id = serenity::ChannelId::new(session.result_channel_id.parse()?);
    let language = data.language(guild_id).await;
    let rows = crate::word_puzzle_store::summary(&data.db_pool, session.id).await?;
    channel_id
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .embed(ui::panel(
                    data,
                    Tone::Primary,
                    format_summary(language, &session.answer, &rows),
                ))
                .allowed_mentions(serenity::CreateAllowedMentions::new()),
        )
        .await?;
    Ok(())
}

fn format_board(board: &Board) -> String {
    if board.guesses.is_empty() {
        return "—".to_owned();
    }
    board
        .guesses
        .iter()
        .map(|guess| {
            let marks: String = guess
                .marks
                .chars()
                .map(|mark| match mark {
                    'X' => '🟩',
                    'P' => '🟨',
                    _ => '⬛',
                })
                .collect();
            format!("{}. `{}` {marks}", guess.attempt, guess.word.to_uppercase())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_summary(language: Language, answer: &str, rows: &[SummaryEntry]) -> String {
    let rows = rows
        .iter()
        .map(|row| {
            render(
                language,
                if row.status == "won" {
                    Text::SummaryWon
                } else {
                    Text::SummaryLost
                },
                &[&row.user_id, &row.attempts],
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n{}\n{rows}",
        render(language, Text::SummaryTitle, &[]),
        render(language, Text::Answer, &[&answer.to_uppercase()])
    )
}

#[derive(Debug, Clone, Copy)]
#[repr(usize)]
enum Text {
    Created,
    AlreadyOpen,
    NoSession,
    Joined,
    JoinClosed,
    DurationInvalid,
    Started,
    StartDenied,
    GuessRejected,
    GuessAccepted,
    Solved,
    Unsolved,
    Board,
    NotParticipant,
    Finished,
    FinishDenied,
    SummaryTitle,
    Answer,
    SummaryWon,
    SummaryLost,
}

#[cfg(test)]
const TEXT_KEYS: [Text; 20] = [
    Text::Created,
    Text::AlreadyOpen,
    Text::NoSession,
    Text::Joined,
    Text::JoinClosed,
    Text::DurationInvalid,
    Text::Started,
    Text::StartDenied,
    Text::GuessRejected,
    Text::GuessAccepted,
    Text::Solved,
    Text::Unsolved,
    Text::Board,
    Text::NotParticipant,
    Text::Finished,
    Text::FinishDenied,
    Text::SummaryTitle,
    Text::Answer,
    Text::SummaryWon,
    Text::SummaryLost,
];

fn template(language: Language, key: Text) -> &'static str {
    const EN: [&str; 20] = [
        "Word Puzzle #{} created. Use `/word-puzzle join`, then the creator starts it.",
        "This server already has a Word Puzzle awaiting completion or summary delivery.",
        "There is no current Word Puzzle in this server.",
        "You joined the Word Puzzle.",
        "You already joined, or the roster is closed.",
        "The deadline must be from 1 to 60 minutes.",
        "Word Puzzle started. Private guesses close <t:{}:R>.",
        "Only the creator can start an open Word Puzzle.",
        "That guess is invalid, not allowed, late, or your board is already finished.",
        "Guess accepted.",
        "Solved! Your board is complete.",
        "Six attempts used. Your board is complete.",
        "Your private board:",
        "Join the puzzle before it starts to receive a board.",
        "Puzzle finished. The public result follows.",
        "Only the creator can finish a started puzzle.",
        "**Word Puzzle complete**",
        "Answer: **{}**",
        "<@{}> — solved in {} attempt(s)",
        "<@{}> — unsolved after {} attempt(s)",
    ];
    const VI: [&str; 20] = [
        "Đã tạo Câu đố chữ #{}. Dùng `/word-puzzle join`, sau đó người tạo bắt đầu.",
        "Server này đã có một Câu đố chữ đang chờ hoàn tất hoặc gửi kết quả.",
        "Server này hiện không có Câu đố chữ.",
        "Bạn đã tham gia Câu đố chữ.",
        "Bạn đã tham gia hoặc danh sách đã đóng.",
        "Thời hạn phải từ 1 đến 60 phút.",
        "Câu đố chữ đã bắt đầu. Lượt đoán riêng kết thúc <t:{}:R>.",
        "Chỉ người tạo mới có thể bắt đầu câu đố đang mở.",
        "Từ đoán không hợp lệ, không được phép, quá hạn hoặc bảng của bạn đã hoàn tất.",
        "Đã nhận từ đoán.",
        "Đã giải! Bảng của bạn đã hoàn tất.",
        "Đã dùng sáu lượt. Bảng của bạn đã hoàn tất.",
        "Bảng riêng của bạn:",
        "Hãy tham gia trước khi câu đố bắt đầu để nhận bảng.",
        "Câu đố đã kết thúc. Kết quả công khai sẽ xuất hiện ngay.",
        "Chỉ người tạo mới có thể kết thúc câu đố đã bắt đầu.",
        "**Câu đố chữ đã hoàn tất**",
        "Đáp án: **{}**",
        "<@{}> — giải được trong {} lượt",
        "<@{}> — chưa giải được sau {} lượt",
    ];
    const JA: [&str; 20] = [
        "ワードパズル #{} を作成しました。`/word-puzzle join` の後、作成者が開始します。",
        "このサーバーには完了または結果送信待ちのワードパズルがあります。",
        "このサーバーに進行中のワードパズルはありません。",
        "ワードパズルに参加しました。",
        "参加済み、または参加受付は終了しています。",
        "制限時間は1〜60分にしてください。",
        "ワードパズルを開始しました。非公開の回答期限は <t:{}:R> です。",
        "開始できるのは作成者だけです。",
        "無効・辞書外・期限切れの単語か、ボードが既に終了しています。",
        "回答を受け付けました。",
        "正解です！あなたのボードは終了しました。",
        "6回使いました。あなたのボードは終了しました。",
        "あなたの非公開ボード：",
        "開始前に参加するとボードを受け取れます。",
        "パズルを終了しました。公開結果を続けて表示します。",
        "開始済みパズルを終了できるのは作成者だけです。",
        "**ワードパズル終了**",
        "答え：**{}**",
        "<@{}> — {}回で正解",
        "<@{}> — {}回で未正解",
    ];
    match language {
        Language::English => EN[key as usize],
        Language::Vietnamese => VI[key as usize],
        Language::Japanese => JA[key as usize],
    }
}

fn render(language: Language, key: Text, args: &[&(dyn std::fmt::Display + Sync)]) -> String {
    let mut rendered = template(language, key).to_owned();
    for argument in args {
        rendered = rendered.replacen("{}", &argument.to_string(), 1);
    }
    rendered
}

#[cfg(test)]
mod tests {
    use super::{TEXT_KEYS, Text, format_summary, template};
    use crate::i18n::Language;
    use crate::word_puzzle_store::SummaryEntry;

    #[test]
    fn every_word_puzzle_text_exists_in_all_languages_and_final_reveals_answer() {
        for language in [Language::English, Language::Vietnamese, Language::Japanese] {
            for key in TEXT_KEYS {
                assert!(!template(language, key).is_empty());
            }
        }
        let final_text = format_summary(
            Language::English,
            "apple",
            &[SummaryEntry {
                user_id: "1".to_owned(),
                status: "won".to_owned(),
                attempts: 2,
            }],
        );
        assert!(final_text.contains("APPLE"));
        assert!(final_text.contains("solved in 2"));
        assert!(!template(Language::English, Text::Board).contains("answer"));
    }
}
