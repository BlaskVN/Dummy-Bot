use crate::game_config::{NewGameConfig, clear_game_config, save_game_config};
use crate::permissions::missing_channel_permissions;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::collections::HashSet;

#[poise::command(
    rename = "game-config",
    slash_command,
    subcommands("set", "clear"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn game_config(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[allow(clippy::too_many_arguments)]
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Role that opens the game session"] role: serenity::Role,
    #[description = "Stable game key, for example minecraft"] game_key: String,
    #[description = "Game display name"] display_name: String,
    #[description = "Text channel for game mentions"] game_channel: serenity::GuildChannel,
    #[description = "Primary session voice channel"] primary_voice: serenity::GuildChannel,
    #[description = "Comma-separated voice channel IDs, including the primary"] voice_pool: String,
    #[description = "Exact Discord activity name fallback"] activity_name: String,
    #[description = "Optional Discord activity application ID"] activity_application_id: Option<
        String,
    >,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let timezone: Option<String> =
        sqlx::query_scalar("SELECT iana_name FROM guild_timezone WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(&ctx.data().db_pool)
            .await?
            .flatten();
    if timezone.is_none() {
        ctx.say("Configure the server time zone before enabling game sessions.")
            .await?;
        return Ok(());
    }
    let Ok(pool) = parse_voice_pool(&voice_pool) else {
        ctx.say("Provide one or more unique voice channel IDs.")
            .await?;
        return Ok(());
    };
    let application_id = match activity_application_id
        .as_deref()
        .map(str::parse::<u64>)
        .transpose()
    {
        Ok(value) => value,
        Err(_) => {
            ctx.say("Use a valid Discord application ID.").await?;
            return Ok(());
        }
    };
    if role.guild_id != guild_id
        || game_channel.guild_id != guild_id
        || game_channel.kind != serenity::ChannelType::Text
        || primary_voice.guild_id != guild_id
        || primary_voice.kind != serenity::ChannelType::Voice
        || !pool.contains(&primary_voice.id)
        || !valid_names(&game_key, &display_name, &activity_name)
    {
        ctx.say("Invalid role, channel, voice pool, game name, or activity name.")
            .await?;
        return Ok(());
    }
    let invalid_pool_channel = {
        let guild = ctx
            .guild()
            .ok_or_else(|| anyhow::anyhow!("Guild unavailable"))?;
        pool.iter().any(|id| {
            guild
                .channels
                .get(id)
                .is_none_or(|channel| channel.kind != serenity::ChannelType::Voice)
        })
    };
    if invalid_pool_channel {
        ctx.say("Every voice-pool ID must be a voice channel in this server.")
            .await?;
        return Ok(());
    }
    let bot_id = ctx.cache().current_user().id;
    for channel_id in std::iter::once(game_channel.id).chain(pool.iter().copied()) {
        let missing = missing_channel_permissions(
            ctx,
            channel_id,
            bot_id,
            serenity::Permissions::VIEW_CHANNEL,
        )?;
        if !missing.is_empty() {
            ctx.say(format!("The bot cannot view <#{channel_id}>: {missing}"))
                .await?;
            return Ok(());
        }
    }
    save_game_config(
        &ctx.data().db_pool,
        guild_id,
        NewGameConfig {
            role_id: role.id,
            game_key: &game_key,
            display_name: &display_name,
            game_channel_id: game_channel.id,
            primary_voice_channel_id: primary_voice.id,
            voice_pool: &pool,
            activity_application_id: application_id,
            activity_name: &activity_name,
        },
    )
    .await?;
    ctx.say("Game configuration enabled.").await?;
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    clear_game_config(&ctx.data().db_pool, guild_id).await?;
    ctx.say("Game configuration cleared.").await?;
    Ok(())
}

fn parse_voice_pool(value: &str) -> Result<Vec<serenity::ChannelId>, ()> {
    let mut seen = HashSet::new();
    let channels = value
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(|part| {
            part.trim_matches(['<', '#', '>'])
                .parse::<u64>()
                .map(serenity::ChannelId::new)
                .map_err(|_| ())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if channels.is_empty() || channels.iter().any(|id| !seen.insert(*id)) {
        return Err(());
    }
    Ok(channels)
}

fn valid_names(key: &str, display_name: &str, activity_name: &str) -> bool {
    (1..=50).contains(&key.len())
        && key
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && (1..=100).contains(&display_name.chars().count())
        && (1..=128).contains(&activity_name.chars().count())
}

#[cfg(test)]
mod tests {
    use super::{parse_voice_pool, valid_names};

    #[test]
    fn validates_game_keys_and_unique_voice_pool() {
        assert!(valid_names("minecraft", "Minecraft", "Minecraft"));
        assert!(!valid_names("Minecraft!", "Minecraft", "Minecraft"));
        assert_eq!(parse_voice_pool("<#1>, 2").unwrap().len(), 2);
        assert!(parse_voice_pool("1,1").is_err());
        assert!(parse_voice_pool("").is_err());
    }
}
