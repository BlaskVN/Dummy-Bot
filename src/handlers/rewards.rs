use crate::Data;
use crate::reward_roles::{
    RewardConfig, RewardRoleDenial, claim_degraded_notification, mark_reward_health, reward_config,
    validate_reward_role,
};
use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

pub async fn replace_and_reconcile(
    ctx: &serenity::Context,
    data: &Data,
    guild_id: serenity::GuildId,
    old: Option<RewardConfig>,
) {
    let Some(new) = reward_config(&data.db_pool, guild_id).await.ok().flatten() else {
        return;
    };
    if old.as_ref().is_some_and(|old| old.role_id != new.role_id)
        && let Some(old) = old
    {
        remove_tracked(ctx, &data.db_pool, guild_id, &old.role_id).await;
        let remaining: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM activity_reward_grant WHERE guild_id = ? AND role_id = ?",
        )
        .bind(guild_id.to_string())
        .bind(&old.role_id)
        .fetch_one(&data.db_pool)
        .await
        .unwrap_or(1);
        if old.ownership == "bot_owned"
            && remaining == 0
            && let Ok(role) = old.role_id.parse::<u64>()
            && let Err(error) = guild_id
                .delete_role(&ctx.http, serenity::RoleId::new(role))
                .await
        {
            tracing::warn!(%guild_id, %error, "Could not delete replaced bot-owned reward role");
        }
    }
    reconcile(ctx, data, guild_id).await;
}

pub async fn reconcile(ctx: &serenity::Context, data: &Data, guild_id: serenity::GuildId) {
    reconcile_pool(ctx, &data.db_pool, guild_id).await;
}

pub async fn reconcile_all(ctx: &serenity::Context, data: &Data) {
    let guilds: Vec<String> = sqlx::query_scalar(
        "SELECT guild_id FROM activity_reward_config ORDER BY guild_id LIMIT 500",
    )
    .fetch_all(&data.db_pool)
    .await
    .unwrap_or_default();
    for guild_id in guilds.into_iter().filter_map(|id| id.parse::<u64>().ok()) {
        reconcile(ctx, data, serenity::GuildId::new(guild_id)).await;
    }
}

pub async fn reconcile_pool(
    ctx: &serenity::Context,
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
) {
    let Some(config) = reward_config(pool, guild_id).await.ok().flatten() else {
        return;
    };
    let role_id = match config.role_id.parse::<u64>() {
        Ok(role) => serenity::RoleId::new(role),
        Err(_) => {
            degrade(ctx, pool, guild_id, RewardRoleDenial::Missing).await;
            return;
        }
    };
    let bot_id = ctx.cache.current_user().id;
    let validation = ctx
        .cache
        .guild(guild_id)
        .map_or(Err(RewardRoleDenial::Missing), |guild| {
            validate_reward_role(&guild, bot_id, role_id)
        });
    if let Err(denial) = validation {
        remove_tracked(ctx, pool, guild_id, &config.role_id).await;
        degrade(ctx, pool, guild_id, denial).await;
        return;
    }
    if let Err(error) = mark_reward_health(pool, guild_id, None).await {
        tracing::error!(%guild_id, %error, "Could not mark reward role safe");
    }
    let eligible: Vec<(String, i64)> = sqlx::query_as("SELECT a.user_id, a.play_minutes FROM activity_member_aggregate a LEFT JOIN activity_opt_out o ON o.guild_id = a.guild_id AND o.user_id = a.user_id WHERE a.guild_id = ? AND o.user_id IS NULL ORDER BY a.user_id LIMIT 1000")
        .bind(guild_id.to_string()).fetch_all(pool).await.unwrap_or_default();
    let eligible = eligible
        .into_iter()
        .filter(|(_, minutes)| {
            crate::activity_aggregate::activity_level(*minutes) >= config.level_threshold as u64
        })
        .filter_map(|(user, _)| user.parse::<u64>().ok().map(serenity::UserId::new))
        .collect::<std::collections::HashSet<_>>();
    let tracked: Vec<String> = sqlx::query_scalar(
        "SELECT user_id FROM activity_reward_grant WHERE guild_id = ? AND role_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(&config.role_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for user in tracked {
        let Ok(raw_user) = user.parse::<u64>() else {
            continue;
        };
        let user_id = serenity::UserId::new(raw_user);
        if !eligible.contains(&user_id) {
            remove_one(ctx, pool, guild_id, user_id, role_id).await;
        }
    }
    for user_id in eligible {
        let tracked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM activity_reward_grant WHERE guild_id = ? AND user_id = ? AND role_id = ?)")
            .bind(guild_id.to_string()).bind(user_id.to_string()).bind(role_id.to_string())
            .fetch_one(pool).await.unwrap_or(false);
        if tracked {
            continue;
        }
        let member = match guild_id.member(&ctx.http, user_id).await {
            Ok(member) => member,
            Err(error) => {
                tracing::debug!(%guild_id, %user_id, %error, "Reward-eligible member unavailable");
                continue;
            }
        };
        if member.user.bot || member.roles.contains(&role_id) {
            continue;
        }
        if member.add_role(&ctx.http, role_id).await.is_ok()
            && let Err(error) = sqlx::query("INSERT INTO activity_reward_grant (guild_id, user_id, role_id) VALUES (?, ?, ?) ON CONFLICT DO NOTHING")
                .bind(guild_id.to_string()).bind(user_id.to_string()).bind(role_id.to_string())
                .execute(pool).await
        {
            tracing::error!(%guild_id, %user_id, %error, "Could not track reward grant");
        }
    }
}

pub async fn audit(ctx: &serenity::Context, data: &Data, guild_id: serenity::GuildId) {
    let Some(config) = reward_config(&data.db_pool, guild_id).await.ok().flatten() else {
        return;
    };
    let denial = config
        .role_id
        .parse::<u64>()
        .ok()
        .map(serenity::RoleId::new)
        .and_then(|role_id| {
            ctx.cache.guild(guild_id).map(|guild| {
                validate_reward_role(&guild, ctx.cache.current_user().id, role_id).err()
            })
        })
        .flatten()
        .unwrap_or(RewardRoleDenial::Missing);
    if ctx.cache.guild(guild_id).is_some_and(|guild| {
        config.role_id.parse::<u64>().ok().is_some_and(|role| {
            validate_reward_role(
                &guild,
                ctx.cache.current_user().id,
                serenity::RoleId::new(role),
            )
            .is_ok()
        })
    }) {
        let _ = mark_reward_health(&data.db_pool, guild_id, None).await;
    } else {
        remove_tracked(ctx, &data.db_pool, guild_id, &config.role_id).await;
        degrade(ctx, &data.db_pool, guild_id, denial).await;
    }
}

async fn remove_tracked(
    ctx: &serenity::Context,
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
    role: &str,
) {
    let Ok(role) = role.parse::<u64>().map(serenity::RoleId::new) else {
        return;
    };
    let users: Vec<String> = sqlx::query_scalar(
        "SELECT user_id FROM activity_reward_grant WHERE guild_id = ? AND role_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(role.to_string())
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    for user in users {
        if let Ok(user) = user.parse::<u64>() {
            remove_one(ctx, pool, guild_id, serenity::UserId::new(user), role).await;
        }
    }
}

async fn remove_one(
    ctx: &serenity::Context,
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
    user_id: serenity::UserId,
    role_id: serenity::RoleId,
) {
    match guild_id.member(&ctx.http, user_id).await {
        Ok(member) => {
            if let Err(error) = member.remove_role(&ctx.http, role_id).await
                && !is_not_found(&error)
            {
                return;
            }
        }
        Err(error) if !is_not_found(&error) => return,
        _ => {}
    }
    let _ = sqlx::query(
        "DELETE FROM activity_reward_grant WHERE guild_id = ? AND user_id = ? AND role_id = ?",
    )
    .bind(guild_id.to_string())
    .bind(user_id.to_string())
    .bind(role_id.to_string())
    .execute(pool)
    .await;
}

async fn degrade(
    ctx: &serenity::Context,
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
    denial: RewardRoleDenial,
) {
    if mark_reward_health(pool, guild_id, Some(&denial))
        .await
        .is_err()
        || !claim_degraded_notification(pool, guild_id)
            .await
            .unwrap_or(false)
    {
        return;
    }
    let message = format!("Activity Reward Role disabled: {denial}");
    let channel: Option<String> =
        sqlx::query_scalar("SELECT channel_id FROM moderation_channel_config WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    if let Some(channel) = channel.and_then(|channel| channel.parse::<u64>().ok()) {
        let _ = serenity::ChannelId::new(channel)
            .send_message(
                ctx,
                serenity::CreateMessage::new()
                    .content(message)
                    .allowed_mentions(serenity::CreateAllowedMentions::new()),
            )
            .await;
    } else if let Some(owner) = ctx.cache.guild(guild_id).map(|guild| guild.owner_id) {
        let _ = owner
            .direct_message(ctx, serenity::CreateMessage::new().content(message))
            .await;
    }
}

fn is_not_found(error: &serenity::Error) -> bool {
    matches!(error, serenity::Error::Http(error) if error.status_code().is_some_and(|code| code.as_u16() == 404))
}
