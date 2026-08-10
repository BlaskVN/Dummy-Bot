use poise::serenity_prelude as serenity;
use sqlx::SqlitePool;

/// Administrative/moderation authority that Activity progress must never grant.
pub const UNSAFE_REWARD_PERMISSIONS: serenity::Permissions = serenity::Permissions::ADMINISTRATOR
    .union(serenity::Permissions::VIEW_AUDIT_LOG)
    .union(serenity::Permissions::KICK_MEMBERS)
    .union(serenity::Permissions::BAN_MEMBERS)
    .union(serenity::Permissions::MANAGE_CHANNELS)
    .union(serenity::Permissions::MANAGE_GUILD)
    .union(serenity::Permissions::MANAGE_MESSAGES)
    .union(serenity::Permissions::MUTE_MEMBERS)
    .union(serenity::Permissions::DEAFEN_MEMBERS)
    .union(serenity::Permissions::MOVE_MEMBERS)
    .union(serenity::Permissions::MANAGE_NICKNAMES)
    .union(serenity::Permissions::MANAGE_ROLES)
    .union(serenity::Permissions::MANAGE_WEBHOOKS)
    .union(serenity::Permissions::MANAGE_GUILD_EXPRESSIONS)
    .union(serenity::Permissions::MANAGE_EVENTS)
    .union(serenity::Permissions::MANAGE_THREADS)
    .union(serenity::Permissions::MODERATE_MEMBERS);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewardRoleDenial {
    Everyone,
    Managed,
    Hierarchy,
    BotCannotManageRoles,
    UnsafeBase(serenity::Permissions),
    UnsafeOverwrite(serenity::ChannelId, serenity::Permissions),
    Missing,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RewardConfig {
    pub role_id: String,
    pub level_threshold: i64,
    pub ownership: String,
    pub health: String,
    pub degraded_reason: Option<String>,
    pub notification_sent: i64,
}

pub async fn reward_config(
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
) -> anyhow::Result<Option<RewardConfig>> {
    Ok(sqlx::query_as("SELECT role_id, level_threshold, ownership, health, degraded_reason, notification_sent FROM activity_reward_config WHERE guild_id = ?")
        .bind(guild_id.to_string()).fetch_optional(pool).await?)
}

pub async fn save_reward_config(
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
    role_id: serenity::RoleId,
    level: i64,
    ownership: &str,
) -> anyhow::Result<Option<RewardConfig>> {
    anyhow::ensure!(level > 0, "Reward level must be positive");
    anyhow::ensure!(
        matches!(ownership, "guild_owned" | "bot_owned"),
        "Invalid role ownership"
    );
    let old = reward_config(pool, guild_id).await?;
    sqlx::query("INSERT INTO activity_reward_config (guild_id, role_id, level_threshold, ownership) VALUES (?, ?, ?, ?) ON CONFLICT(guild_id) DO UPDATE SET role_id = excluded.role_id, level_threshold = excluded.level_threshold, ownership = excluded.ownership, health = 'safe', degraded_reason = NULL, notification_sent = 0, updated_at = CURRENT_TIMESTAMP")
        .bind(guild_id.to_string()).bind(role_id.to_string()).bind(level).bind(ownership)
        .execute(pool).await?;
    Ok(old)
}

pub async fn mark_reward_health(
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
    denial: Option<&RewardRoleDenial>,
) -> anyhow::Result<bool> {
    let (health, reason) = denial.map_or(("safe", None), |denial| {
        ("degraded", Some(denial.to_string()))
    });
    let changed = sqlx::query("UPDATE activity_reward_config SET health = ?, degraded_reason = ?, notification_sent = CASE WHEN ? = 'safe' THEN 0 ELSE notification_sent END, updated_at = CURRENT_TIMESTAMP WHERE guild_id = ?")
        .bind(health).bind(reason).bind(health).bind(guild_id.to_string()).execute(pool).await?.rows_affected() == 1;
    Ok(changed)
}

pub async fn claim_degraded_notification(
    pool: &SqlitePool,
    guild_id: serenity::GuildId,
) -> anyhow::Result<bool> {
    Ok(sqlx::query("UPDATE activity_reward_config SET notification_sent = 1 WHERE guild_id = ? AND health = 'degraded' AND notification_sent = 0")
        .bind(guild_id.to_string()).execute(pool).await?.rows_affected() == 1)
}

impl std::fmt::Display for RewardRoleDenial {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Everyone => formatter.write_str("@everyone cannot be an Activity Reward Role"),
            Self::Managed => formatter.write_str("managed/integration roles cannot be assigned"),
            Self::Hierarchy => formatter.write_str("the role must be below the bot's highest role"),
            Self::BotCannotManageRoles => formatter.write_str("the bot needs Manage Roles"),
            Self::UnsafeBase(permissions) => {
                write!(formatter, "unsafe base permissions: {permissions}")
            }
            Self::UnsafeOverwrite(channel, permissions) => {
                write!(
                    formatter,
                    "unsafe permissions granted in <#{channel}>: {permissions}"
                )
            }
            Self::Missing => formatter.write_str("the role is no longer present"),
        }
    }
}

pub fn validate_reward_role(
    guild: &serenity::Guild,
    bot_id: serenity::UserId,
    role_id: serenity::RoleId,
) -> Result<(), RewardRoleDenial> {
    let role = guild.roles.get(&role_id).ok_or(RewardRoleDenial::Missing)?;
    validate_reward_role_data(guild, bot_id, role)
}

pub fn validate_reward_role_data(
    guild: &serenity::Guild,
    bot_id: serenity::UserId,
    role: &serenity::Role,
) -> Result<(), RewardRoleDenial> {
    let bot = guild
        .members
        .get(&bot_id)
        .ok_or(RewardRoleDenial::Hierarchy)?;
    let bot_role = guild
        .member_highest_role(bot)
        .ok_or(RewardRoleDenial::Hierarchy)?;
    let bot_can_manage = guild
        .member_permissions(bot)
        .contains(serenity::Permissions::MANAGE_ROLES);
    let overwrites = guild.channels.values().flat_map(|channel| {
        channel
            .permission_overwrites
            .iter()
            .filter_map(|overwrite| {
                (overwrite.kind == serenity::PermissionOverwriteType::Role(role.id))
                    .then_some((channel.id, overwrite.allow))
            })
    });
    validate_properties(
        role.id,
        guild.id.everyone_role(),
        role.managed,
        role.position,
        bot_role.position,
        bot_can_manage,
        role.permissions,
        overwrites,
    )
}

#[allow(clippy::too_many_arguments)]
fn validate_properties(
    role_id: serenity::RoleId,
    everyone_id: serenity::RoleId,
    managed: bool,
    role_position: u16,
    bot_position: u16,
    bot_can_manage: bool,
    base_permissions: serenity::Permissions,
    overwrites: impl IntoIterator<Item = (serenity::ChannelId, serenity::Permissions)>,
) -> Result<(), RewardRoleDenial> {
    if role_id == everyone_id {
        return Err(RewardRoleDenial::Everyone);
    }
    if managed {
        return Err(RewardRoleDenial::Managed);
    }
    if role_position >= bot_position {
        return Err(RewardRoleDenial::Hierarchy);
    }
    if !bot_can_manage {
        return Err(RewardRoleDenial::BotCannotManageRoles);
    }
    let unsafe_base = base_permissions & UNSAFE_REWARD_PERMISSIONS;
    if !unsafe_base.is_empty() {
        return Err(RewardRoleDenial::UnsafeBase(unsafe_base));
    }
    for (channel, allow) in overwrites {
        let unsafe_allow = allow & UNSAFE_REWARD_PERMISSIONS;
        if !unsafe_allow.is_empty() {
            return Err(RewardRoleDenial::UnsafeOverwrite(channel, unsafe_allow));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        RewardRoleDenial, claim_degraded_notification, mark_reward_health, reward_config,
        save_reward_config, validate_properties,
    };
    use poise::serenity_prelude::{ChannelId, GuildId, Permissions, RoleId};

    #[test]
    fn rejects_authority_managed_roles_and_hierarchy() {
        let safe = || {
            validate_properties(
                RoleId::new(2),
                RoleId::new(1),
                false,
                2,
                3,
                true,
                Permissions::empty(),
                [],
            )
        };
        assert!(safe().is_ok());
        assert_eq!(
            validate_properties(
                RoleId::new(1),
                RoleId::new(1),
                false,
                2,
                3,
                true,
                Permissions::empty(),
                []
            ),
            Err(RewardRoleDenial::Everyone)
        );
        assert_eq!(
            validate_properties(
                RoleId::new(2),
                RoleId::new(1),
                true,
                2,
                3,
                true,
                Permissions::empty(),
                []
            ),
            Err(RewardRoleDenial::Managed)
        );
        assert_eq!(
            validate_properties(
                RoleId::new(2),
                RoleId::new(1),
                false,
                3,
                3,
                true,
                Permissions::empty(),
                []
            ),
            Err(RewardRoleDenial::Hierarchy)
        );
        assert!(matches!(
            validate_properties(
                RoleId::new(2),
                RoleId::new(1),
                false,
                2,
                3,
                true,
                Permissions::ADMINISTRATOR,
                []
            ),
            Err(RewardRoleDenial::UnsafeBase(_))
        ));
        assert_eq!(
            validate_properties(
                RoleId::new(2),
                RoleId::new(1),
                false,
                2,
                3,
                true,
                Permissions::empty(),
                [(ChannelId::new(4), Permissions::MANAGE_MESSAGES)]
            ),
            Err(RewardRoleDenial::UnsafeOverwrite(
                ChannelId::new(4),
                Permissions::MANAGE_MESSAGES
            ))
        );
    }

    #[tokio::test]
    async fn replaces_config_and_notifies_once_per_degradation() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        sqlx::migrate!().run(&pool).await.unwrap();
        let guild = GuildId::new(1);
        assert!(
            save_reward_config(&pool, guild, RoleId::new(10), 2, "bot_owned")
                .await
                .unwrap()
                .is_none()
        );
        let old = save_reward_config(&pool, guild, RoleId::new(11), 3, "guild_owned")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            (old.role_id.as_str(), old.ownership.as_str()),
            ("10", "bot_owned")
        );

        mark_reward_health(&pool, guild, Some(&RewardRoleDenial::Missing))
            .await
            .unwrap();
        let current = reward_config(&pool, guild).await.unwrap().unwrap();
        assert_eq!(
            (
                current.role_id.as_str(),
                current.level_threshold,
                current.health.as_str()
            ),
            ("11", 3, "degraded")
        );
        assert!(claim_degraded_notification(&pool, guild).await.unwrap());
        assert!(!claim_degraded_notification(&pool, guild).await.unwrap());
        assert!(
            reward_config(&pool, GuildId::new(2))
                .await
                .unwrap()
                .is_none()
        );
    }
}
