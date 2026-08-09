use crate::community::create_activity;
use crate::i18n::Language;
use crate::permissions::missing_channel_permissions;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

const REQUIRED_CREATE_PERMISSIONS: serenity::Permissions = serenity::Permissions::CREATE_EVENTS
    .union(serenity::Permissions::VIEW_CHANNEL)
    .union(serenity::Permissions::CONNECT);

#[poise::command(slash_command, subcommands("create"), guild_only)]
pub async fn activity(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
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

    if let Err(error) = create_activity(
        &ctx.data().db_pool,
        guild_id,
        event.id,
        event.kind,
        Some(ctx.author().id),
        None,
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
    use super::{control_labels, event_url, validate_fields};
    use crate::i18n::Language;
    use poise::serenity_prelude::{GuildId, ScheduledEventId};

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
}
