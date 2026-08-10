use crate::community::create_activity;
use crate::i18n::Language;
use crate::permissions::missing_channel_permissions;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::collections::HashSet;

const REQUIRED_CREATE_PERMISSIONS: serenity::Permissions = serenity::Permissions::CREATE_EVENTS
    .union(serenity::Permissions::VIEW_CHANNEL)
    .union(serenity::Permissions::CONNECT);

#[poise::command(
    slash_command,
    subcommands(
        "create",
        "view",
        "update",
        "cancel",
        "check_in",
        "profile",
        "leaderboard",
        "opt_out",
        "opt_in",
        "reward"
    ),
    guild_only
)]
pub async fn activity(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands("reward_set"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn reward(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    rename = "set",
    slash_command,
    guild_only,
    required_permissions = "MANAGE_GUILD",
    required_bot_permissions = "MANAGE_ROLES"
)]
pub async fn reward_set(
    ctx: Context<'_>,
    #[description = "Activity Level required"] level: i64,
    #[description = "Safe existing role; omit to create one"] role: Option<serenity::Role>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    if level <= 0 {
        ctx.say("Reward level must be positive.").await?;
        return Ok(());
    }
    let bot_id = ctx.cache().current_user().id;
    let (role, ownership) = match role {
        Some(role) => (role, "guild_owned"),
        None => (
            guild_id
                .create_role(
                    ctx.http(),
                    serenity::EditRole::new()
                        .name(format!("Activity Level {level}"))
                        .permissions(serenity::Permissions::empty())
                        .hoist(false)
                        .mentionable(false),
                )
                .await?,
            "bot_owned",
        ),
    };
    let validation = ctx.guild().map_or(
        Err(crate::reward_roles::RewardRoleDenial::Missing),
        |guild| crate::reward_roles::validate_reward_role_data(&guild, bot_id, &role),
    );
    if let Err(denial) = validation {
        if ownership == "bot_owned" {
            let _ = guild_id.delete_role(ctx.http(), role.id).await;
        }
        ctx.say(format!("Unsafe Activity Reward Role: {denial}"))
            .await?;
        return Ok(());
    }
    let old = crate::reward_roles::save_reward_config(
        &ctx.data().db_pool,
        guild_id,
        role.id,
        level,
        ownership,
    )
    .await?;
    crate::handlers::rewards::replace_and_reconcile(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        old,
    )
    .await;
    ctx.say(format!(
        "Activity Reward Role set to <@&{}> at Level {level}.",
        role.id
    ))
    .await?;
    Ok(())
}

#[poise::command(rename = "check-in", slash_command, guild_only)]
pub async fn check_in(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let Some(member) = ctx.author_member().await else {
        return Ok(());
    };
    let is_bot = member.user.bot;
    drop(member);
    if is_bot {
        ctx.say(check_in_response(language, false)).await?;
        return Ok(());
    }
    let channel_id = ctx.guild().and_then(|guild| {
        guild
            .voice_states
            .get(&ctx.author().id)
            .and_then(|state| state.channel_id)
    });
    let Some(channel_id) = channel_id else {
        ctx.say(check_in_response(language, false)).await?;
        return Ok(());
    };
    let Some((_, pool)) = crate::game_config::game_config(&ctx.data().db_pool, guild_id).await?
    else {
        ctx.say(check_in_response(language, false)).await?;
        return Ok(());
    };
    if !pool.contains(&channel_id) {
        ctx.say(check_in_response(language, false)).await?;
        return Ok(());
    }
    let Some(event_id) = crate::handlers::activity_presence::find_session(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        channel_id,
    )
    .await
    else {
        ctx.say(check_in_response(language, false)).await?;
        return Ok(());
    };
    crate::handlers::activity_presence::manual_check_in(
        ctx.data(),
        guild_id,
        event_id,
        channel_id,
        ctx.author().id,
    )
    .await;
    crate::handlers::activity_presence::reconcile_channel(
        ctx.serenity_context(),
        ctx.data(),
        guild_id,
        channel_id,
    )
    .await;
    ctx.send(
        poise::CreateReply::default()
            .content(check_in_response(language, true))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

fn check_in_response(language: Language, success: bool) -> &'static str {
    match (language, success) {
        (Language::English, true) => "Checked in for this session and voice channel.",
        (Language::English, false) => "Join a configured session voice channel first.",
        (Language::Vietnamese, true) => "Đã check-in cho phiên và kênh thoại này.",
        (Language::Vietnamese, false) => "Hãy vào kênh thoại của phiên đã cấu hình trước.",
        (Language::Japanese, true) => "このセッションとボイスチャンネルにチェックインしました。",
        (Language::Japanese, false) => "設定済みセッションのボイスチャンネルに参加してください。",
    }
}

#[derive(Debug, sqlx::FromRow)]
struct GameStat {
    game_key: String,
    play_minutes: i64,
    session_credits: i64,
}

#[poise::command(slash_command, guild_only)]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "Member in this server"] member: Option<serenity::Member>,
    #[description = "Per-game page"] page: Option<i64>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let user = member
        .as_ref()
        .map_or(ctx.author().id, |member| member.user.id);
    let page = page.unwrap_or(1);
    if page < 1 {
        ctx.say(profile_unavailable(language)).await?;
        return Ok(());
    }
    let opted_out: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM activity_opt_out WHERE guild_id = ? AND user_id = ?)",
    )
    .bind(guild_id.to_string())
    .bind(user.to_string())
    .fetch_one(&ctx.data().db_pool)
    .await?;
    if opted_out {
        ctx.say(profile_unavailable(language)).await?;
        return Ok(());
    }
    let (minutes, credits): (i64, i64) = sqlx::query_as("SELECT play_minutes, session_credits FROM activity_member_aggregate WHERE guild_id = ? AND user_id = ?")
        .bind(guild_id.to_string()).bind(user.to_string()).fetch_optional(&ctx.data().db_pool).await?.unwrap_or((0, 0));
    let games: Vec<GameStat> = sqlx::query_as("SELECT game_key, play_minutes, session_credits FROM activity_member_game_aggregate WHERE guild_id = ? AND user_id = ? ORDER BY play_minutes DESC, game_key LIMIT 10 OFFSET ?")
        .bind(guild_id.to_string()).bind(user.to_string()).bind((page - 1) * 10)
        .fetch_all(&ctx.data().db_pool).await?;
    let game_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM activity_member_game_aggregate WHERE guild_id = ? AND user_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(user.to_string())
    .fetch_one(&ctx.data().db_pool)
    .await?;
    let level = crate::activity_aggregate::activity_level(minutes);
    let next_minutes =
        ((level + 1) as u128 * (level + 2) as u128 * 30).min(i64::MAX as u128) as i64;
    let labels = profile_labels(language);
    let game_rows = if games.is_empty() {
        labels.5.to_owned()
    } else {
        games
            .iter()
            .map(|game| {
                format!(
                    "`{}` — {} — {} {}",
                    game.game_key,
                    format_duration(game.play_minutes),
                    game.session_credits,
                    labels.2
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let pages = ((game_count + 9) / 10).max(1);
    let description = format!(
        "{}: {}\n{}: {}\n{}: {}\n{}: {}\n\n{} ({}/{}):\n{}",
        labels.0,
        format_duration(minutes),
        labels.1,
        credits,
        labels.3,
        level,
        labels.4,
        format_duration((next_minutes - minutes).max(0)),
        labels.6,
        page,
        pages,
        game_rows,
    );
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::new()
                .title(format!("{} — {}", labels.7, user))
                .description(description)
                .color(ctx.data().config.colors.primary),
        ),
    )
    .await?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RankedMember {
    user_id: u64,
    play_minutes: i64,
    rank: usize,
}

#[poise::command(slash_command, guild_only)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let rows: Vec<(String, i64)> = sqlx::query_as("SELECT a.user_id, a.play_minutes FROM activity_member_aggregate a LEFT JOIN activity_opt_out o ON o.guild_id = a.guild_id AND o.user_id = a.user_id WHERE a.guild_id = ? AND o.user_id IS NULL ORDER BY a.play_minutes DESC, a.user_id LIMIT 1000")
        .bind(guild_id.to_string()).fetch_all(&ctx.data().db_pool).await?;
    let bot_ids = ctx.guild().map_or_else(HashSet::new, |guild| {
        guild
            .members
            .iter()
            .filter_map(|(id, member)| member.user.bot.then_some(id.get()))
            .collect()
    });
    let ranked = rank_members(
        rows.into_iter()
            .filter_map(|(id, minutes)| id.parse().ok().map(|id| (id, minutes)))
            .collect(),
        &bot_ids,
    );
    let mut shown = ranked.iter().take(10).collect::<Vec<_>>();
    if let Some(caller) = ranked
        .iter()
        .find(|row| row.user_id == ctx.author().id.get())
        && !shown.iter().any(|row| row.user_id == caller.user_id)
    {
        shown.push(caller);
    }
    let title = leaderboard_title(language);
    let content = if shown.is_empty() {
        format!("**{title}**\n{}", profile_unavailable(language))
    } else {
        format!(
            "**{title}**\n{}",
            shown
                .iter()
                .map(|row| format!(
                    "{}. <@{}> — {}",
                    row.rank,
                    row.user_id,
                    format_duration(row.play_minutes)
                ))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    ctx.send(
        poise::CreateReply::default()
            .content(content)
            .allowed_mentions(serenity::CreateAllowedMentions::new()),
    )
    .await?;
    Ok(())
}

#[poise::command(rename = "opt-out", slash_command, guild_only)]
pub async fn opt_out(
    ctx: Context<'_>,
    #[description = "Permanently delete this server's Activity data"] confirm: bool,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    if !confirm {
        ctx.send(
            poise::CreateReply::default()
                .content(privacy_response(language, "confirm"))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }
    crate::activity_privacy::opt_out(&ctx.data().db_pool, guild_id, ctx.author().id).await?;
    crate::handlers::activity_presence::remove_member(ctx.data(), guild_id, ctx.author().id).await;
    ctx.send(
        poise::CreateReply::default()
            .content(privacy_response(language, "out"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

#[poise::command(rename = "opt-in", slash_command, guild_only)]
pub async fn opt_in(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    crate::activity_privacy::opt_in(&ctx.data().db_pool, guild_id, ctx.author().id).await?;
    ctx.send(
        poise::CreateReply::default()
            .content(privacy_response(language, "in"))
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

fn privacy_response(language: Language, state: &str) -> &'static str {
    match (language, state) {
        (Language::English, "confirm") => {
            "Run `/activity opt-out confirm:true` to permanently delete this server's Activity data."
        }
        (Language::English, "out") => {
            "Activity tracking is off and this server's Activity data was deleted."
        }
        (Language::English, _) => "Activity tracking is on with empty totals.",
        (Language::Vietnamese, "confirm") => {
            "Chạy `/activity opt-out confirm:true` để xóa vĩnh viễn dữ liệu Hoạt động trong server này."
        }
        (Language::Vietnamese, "out") => {
            "Đã tắt theo dõi và xóa dữ liệu Hoạt động trong server này."
        }
        (Language::Vietnamese, _) => "Đã bật theo dõi Hoạt động với tổng số trống.",
        (Language::Japanese, "confirm") => {
            "このサーバーのアクティビティデータを完全に削除するには `/activity opt-out confirm:true` を実行してください。"
        }
        (Language::Japanese, "out") => {
            "追跡を停止し、このサーバーのアクティビティデータを削除しました。"
        }
        (Language::Japanese, _) => "空の集計でアクティビティ追跡を有効にしました。",
    }
}

fn rank_members(mut rows: Vec<(u64, i64)>, excluded: &HashSet<u64>) -> Vec<RankedMember> {
    rows.retain(|(user, _)| !excluded.contains(user));
    rows.sort_by_key(|(user, minutes)| (std::cmp::Reverse(*minutes), *user));
    let mut previous = None;
    let mut rank = 0;
    rows.into_iter()
        .enumerate()
        .map(|(index, (user_id, play_minutes))| {
            if previous != Some(play_minutes) {
                rank = index + 1;
                previous = Some(play_minutes);
            }
            RankedMember {
                user_id,
                play_minutes,
                rank,
            }
        })
        .collect()
}

fn format_duration(minutes: i64) -> String {
    format!("{}h {:02}m", minutes.max(0) / 60, minutes.max(0) % 60)
}

fn profile_labels(
    language: Language,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match language {
        Language::English => (
            "Play Time",
            "Session Credit",
            "credits",
            "Level",
            "Next Level",
            "No game totals",
            "Games",
            "Activity Profile",
        ),
        Language::Vietnamese => (
            "Thời gian chơi",
            "Điểm phiên",
            "điểm",
            "Cấp",
            "Cấp tiếp theo",
            "Chưa có tổng theo game",
            "Game",
            "Hồ sơ hoạt động",
        ),
        Language::Japanese => (
            "プレイ時間",
            "セッションクレジット",
            "クレジット",
            "レベル",
            "次のレベル",
            "ゲーム別集計なし",
            "ゲーム",
            "アクティビティプロフィール",
        ),
    }
}

fn profile_unavailable(language: Language) -> &'static str {
    match language {
        Language::English => "No Activity Profile is available.",
        Language::Vietnamese => "Không có Hồ sơ hoạt động.",
        Language::Japanese => "アクティビティプロフィールはありません。",
    }
}

fn leaderboard_title(language: Language) -> &'static str {
    match language {
        Language::English => "Play-Time Leaderboard",
        Language::Vietnamese => "Bảng xếp hạng thời gian chơi",
        Language::Japanese => "プレイ時間ランキング",
    }
}

#[derive(Debug, Clone, Copy, poise::ChoiceParameter)]
pub enum ActivityStatus {
    #[name = "Active"]
    Active,
    #[name = "Completed"]
    Completed,
}

#[poise::command(slash_command, guild_only)]
pub async fn create(
    ctx: Context<'_>,
    #[description = "Activity name"] name: String,
    #[description = "RFC 3339 start time, for example 2026-08-10T19:00:00+07:00"]
    start_time: String,
    #[description = "Voice channel"] voice_channel: serenity::GuildChannel,
    #[description = "Optional activity description"] description: Option<String>,
    #[description = "Optional participant limit"] capacity: Option<i64>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let start = match validate_create_input(
        guild_id,
        &voice_channel,
        &name,
        &start_time,
        description.as_deref(),
        capacity,
    ) {
        Ok(start) => start,
        Err(message) => {
            ctx.say(message).await?;
            return Ok(());
        }
    };

    let bot_id = ctx.cache().current_user().id;
    for (user, who) in [(ctx.author().id, "You are"), (bot_id, "The bot is")] {
        let missing =
            missing_channel_permissions(ctx, voice_channel.id, user, REQUIRED_CREATE_PERMISSIONS)?;
        if !missing.is_empty() {
            ctx.say(format!(
                "{who} missing permissions in that channel: {missing}"
            ))
            .await?;
            return Ok(());
        }
    }

    let mut builder =
        serenity::CreateScheduledEvent::new(serenity::ScheduledEventType::Voice, name, start)
            .channel_id(voice_channel.id);
    if let Some(description) = description {
        builder = builder.description(description);
    }
    let event = guild_id.create_scheduled_event(ctx.http(), builder).await?;
    let url = event_url(guild_id, event.id);
    let game_key = crate::game_config::game_config(&ctx.data().db_pool, guild_id)
        .await?
        .filter(|(_, pool)| pool.contains(&voice_channel.id))
        .map(|(config, _)| config.game_key);

    if let Err(error) = create_activity(
        &ctx.data().db_pool,
        guild_id,
        event.id,
        event.kind,
        Some(ctx.author().id),
        game_key.as_deref(),
        capacity,
    )
    .await
    {
        tracing::error!(%guild_id, event_id = %event.id, %error, "Scheduled event extension persistence failed");
        let message = match guild_id.delete_scheduled_event(ctx.http(), event.id).await {
            Ok(()) => "Activity storage failed. The Discord event was removed.".to_owned(),
            Err(delete_error) => {
                tracing::error!(%guild_id, event_id = %event.id, %delete_error, "Compensating scheduled event deletion failed");
                format!(
                    "Activity storage failed and the Discord event could not be removed. Orphan: {url}"
                )
            }
        };
        ctx.say(message).await?;
        return Ok(());
    }

    let (join, leave) = control_labels(language);
    ctx.send(
        poise::CreateReply::default()
            .content(format!("Activity created: {url}"))
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(format!("activity:join:{}", event.id)).label(join),
                serenity::CreateButton::new(format!("activity:leave:{}", event.id))
                    .label(leave)
                    .style(serenity::ButtonStyle::Secondary),
            ])]),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn view(
    ctx: Context<'_>,
    #[description = "Discord Scheduled Event ID"] event_id: String,
) -> Result<(), Error> {
    let Some((guild_id, event_id, record)) = managed_activity(ctx, &event_id).await? else {
        return Ok(());
    };
    match guild_id.scheduled_event(ctx.http(), event_id, true).await {
        Ok(event) => {
            ctx.say(format!(
                "**{}**\nID: `{}`\nStatus: {:?}\nStarts: <t:{}:F>\nChannel: {}\nHost: {}\nCapacity: {}\nParticipants interested: {}",
                event.name,
                event.id,
                event.status,
                event.start_time.unix_timestamp(),
                event.channel_id.map_or_else(|| "—".to_owned(), |id| format!("<#{id}>")),
                record.host_user_id.map_or_else(|| "—".to_owned(), |id| format!("<@{id}>")),
                record.capacity.map_or_else(|| "unlimited".to_owned(), |value| value.to_string()),
                event.user_count.unwrap_or(0),
            ))
            .await?;
        }
        Err(error) if is_not_found(&error) => {
            crate::community::update_activity_extension(
                &ctx.data().db_pool,
                guild_id,
                event_id,
                None,
                Some("deleted"),
            )
            .await?;
            finalize_local(ctx, guild_id, event_id).await?;
            ctx.say("The native event is missing; local state was reconciled.")
                .await?;
        }
        Err(error) => return Err(error.into()),
    }
    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn update(
    ctx: Context<'_>,
    #[description = "Discord Scheduled Event ID"] event_id: String,
    #[description = "New name"] name: Option<String>,
    #[description = "New RFC 3339 start time"] start_time: Option<String>,
    #[description = "New description"] description: Option<String>,
    #[description = "New participant limit"] capacity: Option<i64>,
    #[description = "Lifecycle transition"] status: Option<ActivityStatus>,
) -> Result<(), Error> {
    let Some((guild_id, event_id, record)) = managed_activity(ctx, &event_id).await? else {
        return Ok(());
    };
    if !can_manage(ctx, record.host_user_id.as_deref()).await? {
        ctx.say("Only the host or a member with Manage Events can update this activity.")
            .await?;
        return Ok(());
    }
    if name.is_none()
        && start_time.is_none()
        && description.is_none()
        && capacity.is_none()
        && status.is_none()
    {
        ctx.say("Provide at least one field to update.").await?;
        return Ok(());
    }
    if name
        .as_ref()
        .is_some_and(|value| !(1..=100).contains(&value.chars().count()))
        || description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 1_000)
        || capacity.is_some_and(|value| value <= 0)
    {
        ctx.say("Invalid name, description, or capacity.").await?;
        return Ok(());
    }
    let start = match start_time
        .as_deref()
        .map(serenity::Timestamp::parse)
        .transpose()
    {
        Ok(start) if start.is_none_or(|value| value > serenity::Timestamp::now()) => start,
        _ => {
            ctx.say("Use a future RFC 3339 start time with a UTC offset.")
                .await?;
            return Ok(());
        }
    };
    let event = match guild_id.scheduled_event(ctx.http(), event_id, false).await {
        Ok(event) => event,
        Err(error) if is_not_found(&error) => {
            crate::community::update_activity_extension(
                &ctx.data().db_pool,
                guild_id,
                event_id,
                None,
                Some("deleted"),
            )
            .await?;
            ctx.say("The native event is missing; local state was reconciled.")
                .await?;
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    let next_status = status.map(ActivityStatus::native);
    if next_status.is_some_and(|next| !valid_transition(event.status, next)) {
        ctx.say("That scheduled-event status transition is not allowed.")
            .await?;
        return Ok(());
    }
    if let Some(channel_id) = event.channel_id {
        let bot_id = ctx.cache().current_user().id;
        let missing = missing_channel_permissions(
            ctx,
            channel_id,
            bot_id,
            serenity::Permissions::CREATE_EVENTS | serenity::Permissions::VIEW_CHANNEL,
        )?;
        if !missing.is_empty() {
            ctx.say(format!("The bot is missing permissions: {missing}"))
                .await?;
            return Ok(());
        }
    }
    let has_native_update =
        name.is_some() || start.is_some() || description.is_some() || next_status.is_some();
    let mut builder = serenity::EditScheduledEvent::new();
    if let Some(name) = name {
        builder = builder.name(name);
    }
    if let Some(start) = start {
        builder = builder.start_time(start);
    }
    if let Some(description) = description {
        builder = builder.description(description);
    }
    if let Some(status) = next_status {
        builder = builder.status(status);
    }
    if has_native_update {
        guild_id
            .edit_scheduled_event(ctx.http(), event_id, builder)
            .await?;
    }
    crate::community::update_activity_extension(
        &ctx.data().db_pool,
        guild_id,
        event_id,
        None,
        next_status.map(activity_state),
    )
    .await?;
    if next_status == Some(serenity::ScheduledEventStatus::Completed) {
        finalize_local(ctx, guild_id, event_id).await?;
    }
    if capacity.is_some() {
        let promoted = crate::community::set_activity_capacity(
            &ctx.data().db_pool,
            guild_id,
            event_id,
            capacity,
        )
        .await?;
        crate::handlers::community::notify_promotions(
            ctx.serenity_context(),
            ctx.data(),
            guild_id,
            event_id,
            &promoted,
        )
        .await;
    }
    ctx.say(format!(
        "Activity updated: {}",
        event_url(guild_id, event_id)
    ))
    .await?;
    Ok(())
}

#[poise::command(slash_command, guild_only)]
pub async fn cancel(
    ctx: Context<'_>,
    #[description = "Discord Scheduled Event ID"] event_id: String,
) -> Result<(), Error> {
    let Some((guild_id, event_id, record)) = managed_activity(ctx, &event_id).await? else {
        return Ok(());
    };
    if !can_manage(ctx, record.host_user_id.as_deref()).await? {
        ctx.say("Only the host or a member with Manage Events can cancel this activity.")
            .await?;
        return Ok(());
    }
    if matches!(record.state.as_str(), "canceled" | "deleted" | "completed") {
        ctx.say("Activity is already closed.").await?;
        return Ok(());
    }
    let state = match guild_id.delete_scheduled_event(ctx.http(), event_id).await {
        Ok(()) => "canceled",
        Err(error) if is_not_found(&error) => "deleted",
        Err(error) => return Err(error.into()),
    };
    crate::community::update_activity_extension(
        &ctx.data().db_pool,
        guild_id,
        event_id,
        None,
        Some(state),
    )
    .await?;
    finalize_local(ctx, guild_id, event_id).await?;
    ctx.say("Activity canceled.").await?;
    Ok(())
}

async fn finalize_local(
    ctx: Context<'_>,
    guild_id: serenity::GuildId,
    event_id: serenity::ScheduledEventId,
) -> Result<(), Error> {
    let now = chrono::Utc::now().timestamp();
    crate::attendance::pause_session(&ctx.data().db_pool, guild_id, event_id, now).await?;
    crate::activity_aggregate::finalize_activity(&ctx.data().db_pool, guild_id, event_id, now)
        .await?;
    crate::handlers::rewards::reconcile(ctx.serenity_context(), ctx.data(), guild_id).await;
    crate::handlers::activity_presence::clear_session(ctx.data(), guild_id, event_id).await;
    Ok(())
}

async fn managed_activity(
    ctx: Context<'_>,
    raw_event_id: &str,
) -> Result<
    Option<(
        serenity::GuildId,
        serenity::ScheduledEventId,
        crate::community::ActivityRecord,
    )>,
    Error,
> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let Ok(raw_event_id) = raw_event_id.parse::<u64>() else {
        ctx.say("Use a valid Discord Scheduled Event ID.").await?;
        return Ok(None);
    };
    let event_id = serenity::ScheduledEventId::new(raw_event_id);
    let Some(record) = crate::community::activity(&ctx.data().db_pool, guild_id, event_id).await?
    else {
        ctx.say("That event is not a bot-managed activity.").await?;
        return Ok(None);
    };
    Ok(Some((guild_id, event_id, record)))
}

async fn can_manage(ctx: Context<'_>, host: Option<&str>) -> Result<bool, Error> {
    if host == Some(&ctx.author().id.to_string()) {
        return Ok(true);
    }
    let member = ctx
        .author_member()
        .await
        .ok_or_else(|| anyhow::anyhow!("Member unavailable"))?;
    let guild = ctx
        .guild()
        .ok_or_else(|| anyhow::anyhow!("Guild unavailable"))?;
    Ok(guild
        .member_permissions(&member)
        .contains(serenity::Permissions::MANAGE_EVENTS))
}

impl ActivityStatus {
    fn native(self) -> serenity::ScheduledEventStatus {
        match self {
            Self::Active => serenity::ScheduledEventStatus::Active,
            Self::Completed => serenity::ScheduledEventStatus::Completed,
        }
    }
}

fn valid_transition(
    from: serenity::ScheduledEventStatus,
    to: serenity::ScheduledEventStatus,
) -> bool {
    matches!(
        (from, to),
        (
            serenity::ScheduledEventStatus::Scheduled,
            serenity::ScheduledEventStatus::Active
        ) | (
            serenity::ScheduledEventStatus::Active,
            serenity::ScheduledEventStatus::Completed
        )
    )
}

fn activity_state(status: serenity::ScheduledEventStatus) -> &'static str {
    match status {
        serenity::ScheduledEventStatus::Active => "active",
        serenity::ScheduledEventStatus::Completed => "completed",
        serenity::ScheduledEventStatus::Canceled => "canceled",
        _ => "scheduled",
    }
}

fn is_not_found(error: &serenity::Error) -> bool {
    matches!(error, serenity::Error::Http(error) if error.status_code().is_some_and(|code| code.as_u16() == 404))
}

fn validate_create_input(
    guild_id: serenity::GuildId,
    channel: &serenity::GuildChannel,
    name: &str,
    start_time: &str,
    description: Option<&str>,
    capacity: Option<i64>,
) -> Result<serenity::Timestamp, &'static str> {
    if channel.guild_id != guild_id || channel.kind != serenity::ChannelType::Voice {
        return Err("Choose a voice channel from this server.");
    }
    validate_fields(
        name,
        start_time,
        description,
        capacity,
        serenity::Timestamp::now(),
    )
}

fn validate_fields(
    name: &str,
    start_time: &str,
    description: Option<&str>,
    capacity: Option<i64>,
    now: serenity::Timestamp,
) -> Result<serenity::Timestamp, &'static str> {
    if !(1..=100).contains(&name.chars().count()) {
        return Err("Activity names must contain 1 to 100 characters.");
    }
    if description.is_some_and(|value| value.chars().count() > 1_000) {
        return Err("Activity descriptions may contain at most 1,000 characters.");
    }
    if capacity.is_some_and(|value| value <= 0) {
        return Err("Capacity must be positive.");
    }
    let start = serenity::Timestamp::parse(start_time)
        .map_err(|_| "Use an RFC 3339 start time with a UTC offset.")?;
    if start <= now {
        return Err("Start time must be in the future.");
    }
    Ok(start)
}

fn event_url(guild_id: serenity::GuildId, event_id: serenity::ScheduledEventId) -> String {
    format!("https://discord.com/events/{guild_id}/{event_id}")
}

fn control_labels(language: Language) -> (&'static str, &'static str) {
    match language {
        Language::English => ("Join", "Leave"),
        Language::Vietnamese => ("Tham gia", "Rời đi"),
        Language::Japanese => ("参加", "退出"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        control_labels, event_url, format_duration, rank_members, valid_transition, validate_fields,
    };
    use crate::i18n::Language;
    use poise::serenity_prelude::{GuildId, ScheduledEventId, ScheduledEventStatus};
    use std::collections::HashSet;

    #[test]
    fn builds_native_link_and_localized_controls() {
        assert_eq!(
            event_url(GuildId::new(1), ScheduledEventId::new(2)),
            "https://discord.com/events/1/2"
        );
        assert_eq!(control_labels(Language::Vietnamese), ("Tham gia", "Rời đi"));
        assert_eq!(control_labels(Language::Japanese), ("参加", "退出"));
    }

    #[test]
    fn validates_discord_fields_before_creation() {
        let now = "2026-08-09T00:00:00Z".parse().unwrap();
        assert!(validate_fields("Game", "2026-08-10T00:00:00Z", None, Some(5), now).is_ok());
        assert!(validate_fields("", "2026-08-10T00:00:00Z", None, None, now).is_err());
        assert!(validate_fields("Game", "bad", None, None, now).is_err());
        assert!(validate_fields("Game", "2026-08-08T00:00:00Z", None, None, now).is_err());
        assert!(validate_fields("Game", "2026-08-10T00:00:00Z", None, Some(0), now).is_err());
        assert!(
            validate_fields(
                "Game",
                "2026-08-10T00:00:00Z",
                Some(&"x".repeat(1_001)),
                None,
                now
            )
            .is_err()
        );
    }

    #[test]
    fn permits_only_native_forward_transitions() {
        assert!(valid_transition(
            ScheduledEventStatus::Scheduled,
            ScheduledEventStatus::Active
        ));
        assert!(valid_transition(
            ScheduledEventStatus::Active,
            ScheduledEventStatus::Completed
        ));
        assert!(!valid_transition(
            ScheduledEventStatus::Scheduled,
            ScheduledEventStatus::Completed
        ));
        assert!(!valid_transition(
            ScheduledEventStatus::Completed,
            ScheduledEventStatus::Active
        ));
    }

    #[test]
    fn ranks_ties_without_credit_tiebreakers() {
        let ranked = rank_members(
            vec![(4, 60), (2, 120), (3, 120), (1, 180)],
            &HashSet::from([4]),
        );
        assert_eq!(
            ranked
                .iter()
                .map(|row| (row.user_id, row.rank))
                .collect::<Vec<_>>(),
            vec![(1, 1), (2, 2), (3, 2)]
        );
        assert_eq!(format_duration(125), "2h 05m");
    }
}
