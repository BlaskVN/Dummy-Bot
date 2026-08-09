use crate::community::{
    MembershipState, claim_deleted_activity, claim_promotion_notification,
    finish_promotion_notification, join_activity, leave_activity, mirror_activity_state,
    nonterminal_activities,
};
use crate::i18n::Language;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;

const RECONCILE_LIMIT: i64 = 500;

pub async fn handle_native_update(data: &Data, event: &serenity::ScheduledEvent) {
    let Some(state) = native_state(event.status) else {
        return;
    };
    if let Err(error) = mirror_activity_state(&data.db_pool, event.guild_id, event.id, state).await
    {
        tracing::error!(guild_id = %event.guild_id, event_id = %event.id, %error, "Could not mirror scheduled event state");
    }
    if matches!(state, "completed" | "canceled") {
        if let Err(error) = crate::attendance::pause_session(
            &data.db_pool,
            event.guild_id,
            event.id,
            chrono::Utc::now().timestamp(),
        )
        .await
        {
            tracing::error!(guild_id = %event.guild_id, event_id = %event.id, %error, "Could not pause terminal activity attendance");
        }
        if let Err(error) = crate::activity_aggregate::finalize_activity(
            &data.db_pool,
            event.guild_id,
            event.id,
            chrono::Utc::now().timestamp(),
        )
        .await
        {
            tracing::error!(guild_id = %event.guild_id, event_id = %event.id, %error, "Could not finalize terminal activity");
        }
        super::activity_presence::clear_session(data, event.guild_id, event.id).await;
    }
}

pub async fn handle_native_delete(
    ctx: &serenity::Context,
    data: &Data,
    event: &serenity::ScheduledEvent,
) {
    if let Err(error) = crate::attendance::pause_session(
        &data.db_pool,
        event.guild_id,
        event.id,
        chrono::Utc::now().timestamp(),
    )
    .await
    {
        tracing::error!(guild_id = %event.guild_id, event_id = %event.id, %error, "Could not pause deleted activity attendance");
    }
    super::activity_presence::clear_session(data, event.guild_id, event.id).await;
    notify_deleted(ctx, data, event.guild_id, event.id).await;
    if let Err(error) = crate::activity_aggregate::finalize_activity(
        &data.db_pool,
        event.guild_id,
        event.id,
        chrono::Utc::now().timestamp(),
    )
    .await
    {
        tracing::error!(guild_id = %event.guild_id, event_id = %event.id, %error, "Could not finalize deleted activity");
    }
}

pub async fn reconcile_all(ctx: &serenity::Context, data: &Data) {
    let activities = match nonterminal_activities(&data.db_pool, RECONCILE_LIMIT).await {
        Ok(activities) => activities,
        Err(error) => {
            tracing::error!(%error, "Could not load activities for reconciliation");
            return;
        }
    };
    for activity in activities {
        let (Ok(guild), Ok(event)) = (
            activity.guild_id.parse::<u64>(),
            activity.scheduled_event_id.parse::<u64>(),
        ) else {
            tracing::error!(
                guild_id = activity.guild_id,
                event_id = activity.scheduled_event_id,
                "Invalid stored activity identity"
            );
            continue;
        };
        let guild_id = serenity::GuildId::new(guild);
        if guild_id.shard_id(&ctx.cache) != ctx.shard_id.get() {
            continue;
        }
        let event_id = serenity::ScheduledEventId::new(event);
        match guild_id.scheduled_event(&ctx.http, event_id, false).await {
            Ok(event) => handle_native_update(data, &event).await,
            Err(error) if is_not_found(&error) => {
                notify_deleted(ctx, data, guild_id, event_id).await
            }
            Err(error) => {
                tracing::warn!(%guild_id, %event_id, %error, "Scheduled event reconciliation failed")
            }
        }
    }
}

async fn notify_deleted(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: serenity::GuildId,
    event_id: serenity::ScheduledEventId,
) {
    let users = match claim_deleted_activity(&data.db_pool, guild_id, event_id).await {
        Ok(Some(users)) => users,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(%guild_id, %event_id, %error, "Could not reconcile deleted activity");
            return;
        }
    };
    for user_id in users {
        if let Err(error) = user_id
            .direct_message(
                ctx,
                serenity::CreateMessage::new()
                    .content("A Community Activity you joined was canceled or deleted."),
            )
            .await
        {
            tracing::debug!(%guild_id, %event_id, %user_id, %error, "Could not notify activity member");
        }
    }
}

fn native_state(status: serenity::ScheduledEventStatus) -> Option<&'static str> {
    match status {
        serenity::ScheduledEventStatus::Scheduled => Some("scheduled"),
        serenity::ScheduledEventStatus::Active => Some("active"),
        serenity::ScheduledEventStatus::Completed => Some("completed"),
        serenity::ScheduledEventStatus::Canceled => Some("canceled"),
        _ => None,
    }
}

fn is_not_found(error: &serenity::Error) -> bool {
    matches!(error, serenity::Error::Http(error) if error.status_code().is_some_and(|code| code.as_u16() == 404))
}

pub async fn handle_component(
    ctx: &serenity::Context,
    interaction: &serenity::ComponentInteraction,
    data: &Data,
) -> Result<(), Error> {
    let Some((action, event_id)) = parse_custom_id(&interaction.data.custom_id) else {
        return Ok(());
    };
    let Some(guild_id) = interaction.guild_id else {
        return Ok(());
    };
    let language = data.language(guild_id).await;
    let message = match action {
        "join" => match join_activity(
            &data.db_pool,
            guild_id,
            event_id,
            interaction.user.id,
            interaction.user.bot,
        )
        .await
        {
            Ok(MembershipState::Participant) => response(language, Response::Joined),
            Ok(MembershipState::Waitlisted) => response(language, Response::Waitlisted),
            Ok(MembershipState::Closed) => response(language, Response::Closed),
            Err(error) => {
                tracing::warn!(%guild_id, %event_id, %error, "Activity join rejected");
                response(language, Response::Closed)
            }
        },
        "leave" => {
            let result =
                leave_activity(&data.db_pool, guild_id, event_id, interaction.user.id).await?;
            notify_promotions(ctx, data, guild_id, event_id, &result.promoted).await;
            response(
                language,
                if result.left {
                    Response::Left
                } else {
                    Response::NotJoined
                },
            )
        }
        _ => return Ok(()),
    };
    interaction
        .create_response(
            ctx,
            serenity::CreateInteractionResponse::Message(
                serenity::CreateInteractionResponseMessage::new()
                    .content(message)
                    .ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

pub async fn notify_promotions(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: serenity::GuildId,
    event_id: serenity::ScheduledEventId,
    users: &[serenity::UserId],
) {
    for &user_id in users {
        let Ok(true) =
            claim_promotion_notification(&data.db_pool, guild_id, event_id, user_id).await
        else {
            continue;
        };
        let delivered = user_id
            .direct_message(
                ctx,
                serenity::CreateMessage::new().content(format!(
                    "A place opened and you joined the activity: https://discord.com/events/{guild_id}/{event_id}"
                )),
            )
            .await
            .is_ok();
        if let Err(error) =
            finish_promotion_notification(&data.db_pool, guild_id, event_id, user_id, delivered)
                .await
        {
            tracing::error!(%guild_id, %event_id, %user_id, %error, "Could not persist promotion notification result");
        }
    }
}

fn parse_custom_id(value: &str) -> Option<(&str, serenity::ScheduledEventId)> {
    let mut parts = value.split(':');
    if parts.next()? != "activity" {
        return None;
    }
    let action = parts.next()?;
    let event_id = serenity::ScheduledEventId::new(parts.next()?.parse().ok()?);
    (parts.next().is_none() && matches!(action, "join" | "leave")).then_some((action, event_id))
}

enum Response {
    Joined,
    Waitlisted,
    Closed,
    Left,
    NotJoined,
}

fn response(language: Language, response: Response) -> &'static str {
    match (language, response) {
        (Language::English, Response::Joined) => "You joined the activity.",
        (Language::English, Response::Waitlisted) => {
            "The activity is full; you joined the waitlist."
        }
        (Language::English, Response::Closed) => "This activity is closed or missing.",
        (Language::English, Response::Left) => "You left the activity.",
        (Language::English, Response::NotJoined) => "You are not in this activity.",
        (Language::Vietnamese, Response::Joined) => "Bạn đã tham gia hoạt động.",
        (Language::Vietnamese, Response::Waitlisted) => {
            "Hoạt động đã đầy; bạn đã vào danh sách chờ."
        }
        (Language::Vietnamese, Response::Closed) => "Hoạt động này đã đóng hoặc không còn tồn tại.",
        (Language::Vietnamese, Response::Left) => "Bạn đã rời hoạt động.",
        (Language::Vietnamese, Response::NotJoined) => "Bạn chưa tham gia hoạt động này.",
        (Language::Japanese, Response::Joined) => "アクティビティに参加しました。",
        (Language::Japanese, Response::Waitlisted) => "満員のため、キャンセル待ちに登録しました。",
        (Language::Japanese, Response::Closed) => "このアクティビティは終了済みか存在しません。",
        (Language::Japanese, Response::Left) => "アクティビティから退出しました。",
        (Language::Japanese, Response::NotJoined) => "このアクティビティには参加していません。",
    }
}

#[cfg(test)]
mod tests {
    use super::{native_state, parse_custom_id};
    use poise::serenity_prelude::ScheduledEventStatus;

    #[test]
    fn accepts_only_owned_component_ids() {
        assert_eq!(parse_custom_id("activity:join:42").unwrap().1.get(), 42);
        assert!(parse_custom_id("activity:join:42:extra").is_none());
        assert!(parse_custom_id("other:join:42").is_none());
        assert!(parse_custom_id("activity:delete:42").is_none());
    }

    #[test]
    fn maps_only_known_native_states() {
        assert_eq!(
            native_state(ScheduledEventStatus::Scheduled),
            Some("scheduled")
        );
        assert_eq!(
            native_state(ScheduledEventStatus::Completed),
            Some("completed")
        );
    }
}
