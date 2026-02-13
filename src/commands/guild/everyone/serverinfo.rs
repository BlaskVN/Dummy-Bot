use crate::i18n::{get_guild_language, tf, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Display current server information.
#[poise::command(slash_command, prefix_command, guild_only)]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => get_guild_language(&ctx.data().db_pool, guild_id).await,
        None => crate::i18n::Language::English,
    };

    let guild = ctx
        .guild()
        .ok_or_else(|| anyhow::anyhow!("Could not fetch guild info"))?
        .clone();

    let member_count = guild.member_count;
    let name = &guild.name;
    let channel_count = guild.channels.len();
    let role_count = guild.roles.len();
    let created_at = guild.id.created_at();

    let name_text = tf(lang, TranslationKey::ServerInfoName, &[&name]);
    let members_text = tf(lang, TranslationKey::ServerInfoMembers, &[&member_count]);
    let channels_text = tf(lang, TranslationKey::ServerInfoChannels, &[&channel_count]);
    let roles_text = tf(lang, TranslationKey::ServerInfoRoles, &[&role_count]);
    let created_text = tf(lang, TranslationKey::ServerInfoCreated, &[&created_at.unix_timestamp()]);

    let description = format!(
        "├ {}\n├ {}\n├ {}\n├ {}\n└ {}",
        name_text,
        members_text,
        channels_text,
        roles_text,
        created_text
    );

    let embed = serenity::CreateEmbed::new()
        .title(crate::i18n::t(lang, TranslationKey::ServerInfoTitle))
        .description(description)
        .color(0x9b59b6); // Purple

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
