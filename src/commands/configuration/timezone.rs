use crate::i18n::{TranslationKey, t, tf};
use crate::timezone;
use crate::{Context, Error};

#[poise::command(
    slash_command,
    subcommands("set", "show", "clear"),
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn timezone(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn set(
    ctx: Context<'_>,
    #[description = "IANA time zone, e.g. Asia/Bangkok"] iana_name: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    if timezone::parse(&iana_name).is_none() {
        ctx.say(t(lang, TranslationKey::TimezoneInvalid)).await?;
        return Ok(());
    }

    sqlx::query(
        "INSERT INTO guild_timezone (guild_id, iana_name) VALUES (?, ?)\n         ON CONFLICT(guild_id) DO UPDATE SET iana_name = excluded.iana_name, updated_at = CURRENT_TIMESTAMP",
    )
    .bind(guild_id.to_string())
    .bind(&iana_name)
    .execute(&ctx.data().db_pool)
    .await?;
    let message = tf(lang, TranslationKey::TimezoneSet, &[&iana_name]);
    ctx.say(message).await?;
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn show(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    let value = sqlx::query_scalar::<_, Option<String>>(
        "SELECT iana_name FROM guild_timezone WHERE guild_id = ?",
    )
    .bind(guild_id.to_string())
    .fetch_optional(&ctx.data().db_pool)
    .await?
    .flatten();
    ctx.say(match value {
        Some(value) => tf(lang, TranslationKey::TimezoneCurrent, &[&value]),
        None => t(lang, TranslationKey::TimezoneNotConfigured).to_string(),
    })
    .await?;
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    sqlx::query(
        "INSERT INTO guild_timezone (guild_id, iana_name) VALUES (?, NULL)\n         ON CONFLICT(guild_id) DO UPDATE SET iana_name = NULL, updated_at = CURRENT_TIMESTAMP",
    )
        .bind(guild_id.to_string())
        .execute(&ctx.data().db_pool)
        .await?;
    ctx.say(t(lang, TranslationKey::TimezoneCleared)).await?;
    Ok(())
}
