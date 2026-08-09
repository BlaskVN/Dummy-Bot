use crate::community::{
    MembershipState, claim_promotion_notification, finish_promotion_notification, join_activity,
    leave_activity,
};
use crate::i18n::Language;
use crate::{Data, Error};
use poise::serenity_prelude as serenity;

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
    use super::parse_custom_id;

    #[test]
    fn accepts_only_owned_component_ids() {
        assert_eq!(parse_custom_id("activity:join:42").unwrap().1.get(), 42);
        assert!(parse_custom_id("activity:join:42:extra").is_none());
        assert!(parse_custom_id("other:join:42").is_none());
        assert!(parse_custom_id("activity:delete:42").is_none());
    }
}
