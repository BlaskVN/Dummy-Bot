use poise::serenity_prelude as serenity;

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
                (overwrite.kind == serenity::PermissionOverwriteType::Role(role_id))
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
    use super::{RewardRoleDenial, validate_properties};
    use poise::serenity_prelude::{ChannelId, Permissions, RoleId};

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
}
