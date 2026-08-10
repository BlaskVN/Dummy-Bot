use crate::i18n::{TranslationKey, tf};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Display current server information.
#[poise::command(slash_command, guild_only, user_cooldown = 5)]
pub async fn serverinfo(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
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
    let created_text = tf(
        lang,
        TranslationKey::ServerInfoCreated,
        &[&created_at.unix_timestamp()],
    );

    let description = format!(
        "├ {}\n├ {}\n├ {}\n├ {}\n└ {}",
        name_text, members_text, channels_text, roles_text, created_text
    );

    let embed = serenity::CreateEmbed::new()
        .title(crate::i18n::t(lang, TranslationKey::ServerInfoTitle))
        .description(description)
        .color(ctx.data().config.colors.server_info);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
