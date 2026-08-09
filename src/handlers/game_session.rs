use crate::community::{
    active_game_activity, create_game_activity, due_game_activities, finish_game_expiry,
    mirror_activity_state, next_game_expiry,
};
use crate::{Data, Error};
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::Notify;

const EXPIRY_BATCH: i64 = 100;

pub async fn handle_message(
    ctx: &serenity::Context,
    message: &serenity::Message,
    data: &Data,
) -> Result<(), Error> {
    let Some(guild_id) = message.guild_id else {
        return Ok(());
    };
    if message.author.bot || message.webhook_id.is_some() {
        return Ok(());
    }
    let Some((config, _)) = crate::game_config::game_config(&data.db_pool, guild_id).await? else {
        return Ok(());
    };
    if !qualifies_message(
        message.channel_id,
        config.game_channel()?,
        &message.mention_roles,
        config.role()?,
        message.author.bot,
        message.webhook_id.is_some(),
    ) {
        return Ok(());
    }
    let timezone: Option<String> =
        sqlx::query_scalar("SELECT iana_name FROM guild_timezone WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(&data.db_pool)
            .await?
            .flatten();
    let Some(timezone) = timezone.and_then(|name| crate::timezone::parse(&name)) else {
        return Ok(());
    };

    let _creation = data.game_session_creation.lock().await;
    if let Some(event_id) = active_game_activity(&data.db_pool, guild_id, &config.game_key).await? {
        match guild_id.scheduled_event(&ctx.http, event_id, false).await {
            Ok(_) => {
                send_session_link(ctx, message.channel_id, guild_id, event_id).await?;
                return Ok(());
            }
            Err(error) if is_not_found(&error) => {
                mirror_activity_state(&data.db_pool, guild_id, event_id, "deleted").await?;
            }
            Err(error) => {
                tracing::warn!(%guild_id, %event_id, %error, "Could not verify existing game session");
                return Ok(());
            }
        }
    }

    let primary = config.primary_voice_channel()?;
    let bot_id = ctx.cache.current_user().id;
    let missing = {
        let guild = ctx
            .cache
            .guild(guild_id)
            .ok_or_else(|| anyhow::anyhow!("Guild unavailable"))?;
        let channel = guild
            .channels
            .get(&primary)
            .ok_or_else(|| anyhow::anyhow!("Primary voice channel unavailable"))?;
        let member = guild
            .members
            .get(&bot_id)
            .ok_or_else(|| anyhow::anyhow!("Bot member unavailable"))?;
        let required = serenity::Permissions::CREATE_EVENTS
            | serenity::Permissions::VIEW_CHANNEL
            | serenity::Permissions::CONNECT;
        required - guild.user_permissions_in(channel, member)
    };
    if !missing.is_empty() {
        tracing::warn!(%guild_id, %missing, "Bot cannot create configured game session");
        return Ok(());
    }
    let start = serenity::Timestamp::from_unix_timestamp(chrono::Utc::now().timestamp() + 60)?;
    let event = guild_id
        .create_scheduled_event(
            &ctx.http,
            serenity::CreateScheduledEvent::new(
                serenity::ScheduledEventType::Voice,
                &config.display_name,
                start,
            )
            .channel_id(primary),
        )
        .await?;
    let expires_at = crate::timezone::next_five_am(chrono::Utc::now(), timezone)
        .ok_or_else(|| anyhow::anyhow!("Could not calculate game session expiry"))?
        .timestamp();
    if let Err(error) = create_game_activity(
        &data.db_pool,
        guild_id,
        event.id,
        event.kind,
        &config.game_key,
        expires_at,
    )
    .await
    {
        tracing::error!(%guild_id, event_id = %event.id, %error, "Could not persist game session");
        if let Err(delete_error) = guild_id.delete_scheduled_event(&ctx.http, event.id).await {
            tracing::error!(%guild_id, event_id = %event.id, %delete_error, "Could not remove orphaned game session");
        }
        return Ok(());
    }
    data.game_expiry_wakeup.notify_one();
    send_session_link(ctx, message.channel_id, guild_id, event.id).await?;
    Ok(())
}

pub fn spawn_expiry_worker(ctx: serenity::Context, pool: SqlitePool, wakeup: Arc<Notify>) {
    tokio::spawn(async move {
        loop {
            expire_due(&ctx, &pool).await;
            let next = match next_game_expiry(&pool).await {
                Ok(next) => next,
                Err(error) => {
                    tracing::error!(%error, "Could not load next game-session expiry");
                    Some(chrono::Utc::now().timestamp() + 60)
                }
            };
            match next {
                Some(timestamp) => {
                    let remaining = timestamp - chrono::Utc::now().timestamp();
                    let seconds = if remaining > 0 { remaining as u64 } else { 60 };
                    tokio::select! {
                        () = tokio::time::sleep(std::time::Duration::from_secs(seconds)) => {}
                        () = wakeup.notified() => {}
                    }
                }
                None => wakeup.notified().await,
            }
        }
    });
}

pub fn wake_expiry(data: &Data) {
    data.game_expiry_wakeup.notify_one();
}

async fn expire_due(ctx: &serenity::Context, pool: &SqlitePool) {
    let due = match due_game_activities(pool, chrono::Utc::now().timestamp(), EXPIRY_BATCH).await {
        Ok(due) => due,
        Err(error) => {
            tracing::error!(%error, "Could not load overdue game sessions");
            return;
        }
    };
    for activity in due {
        let (Ok(guild), Ok(event)) = (
            activity.guild_id.parse::<u64>(),
            activity.scheduled_event_id.parse::<u64>(),
        ) else {
            tracing::error!(
                guild_id = activity.guild_id,
                event_id = activity.scheduled_event_id,
                "Invalid stored game session identity"
            );
            continue;
        };
        let guild_id = serenity::GuildId::new(guild);
        let event_id = serenity::ScheduledEventId::new(event);
        let state = match guild_id.delete_scheduled_event(&ctx.http, event_id).await {
            Ok(()) => "canceled",
            Err(error) if is_not_found(&error) => "deleted",
            Err(error) => {
                tracing::warn!(%guild_id, %event_id, %error, "Could not expire native game session");
                continue;
            }
        };
        if let Err(error) = finish_game_expiry(pool, guild_id, event_id, state).await {
            tracing::error!(%guild_id, %event_id, %error, "Could not finalize game-session expiry");
        }
    }
}

fn is_not_found(error: &serenity::Error) -> bool {
    matches!(error, serenity::Error::Http(error) if error.status_code().is_some_and(|code| code.as_u16() == 404))
}

async fn send_session_link(
    ctx: &serenity::Context,
    channel_id: serenity::ChannelId,
    guild_id: serenity::GuildId,
    event_id: serenity::ScheduledEventId,
) -> Result<(), serenity::Error> {
    channel_id
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .content(format!(
                    "Game session: https://discord.com/events/{guild_id}/{event_id}"
                ))
                .allowed_mentions(serenity::CreateAllowedMentions::new()),
        )
        .await?;
    Ok(())
}

fn qualifies_message(
    channel_id: serenity::ChannelId,
    game_channel_id: serenity::ChannelId,
    mentioned_roles: &[serenity::RoleId],
    game_role_id: serenity::RoleId,
    author_is_bot: bool,
    is_webhook: bool,
) -> bool {
    !author_is_bot
        && !is_webhook
        && channel_id == game_channel_id
        && mentioned_roles.contains(&game_role_id)
}

#[cfg(test)]
mod tests {
    use super::qualifies_message;
    use poise::serenity_prelude::{ChannelId, RoleId};

    #[test]
    fn requires_exact_parsed_role_in_configured_channel() {
        let roles = [RoleId::new(3)];
        assert!(qualifies_message(
            ChannelId::new(1),
            ChannelId::new(1),
            &roles,
            RoleId::new(3),
            false,
            false
        ));
        assert!(!qualifies_message(
            ChannelId::new(2),
            ChannelId::new(1),
            &roles,
            RoleId::new(3),
            false,
            false
        ));
        assert!(!qualifies_message(
            ChannelId::new(1),
            ChannelId::new(1),
            &[],
            RoleId::new(3),
            false,
            false
        ));
        assert!(!qualifies_message(
            ChannelId::new(1),
            ChannelId::new(1),
            &roles,
            RoleId::new(3),
            true,
            false
        ));
        assert!(!qualifies_message(
            ChannelId::new(1),
            ChannelId::new(1),
            &roles,
            RoleId::new(3),
            false,
            true
        ));
    }
}
