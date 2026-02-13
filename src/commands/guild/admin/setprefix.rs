use crate::i18n::{get_guild_language, tf, TranslationKey};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Set a custom command prefix for this server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    required_permissions = "MANAGE_GUILD"
)]
pub async fn setprefix(
    ctx: Context<'_>,
    #[description = "New prefix for the server"]
    #[min_length = 1]
    #[max_length = 5]
    new_prefix: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = get_guild_language(&ctx.data().db_pool, guild_id).await;

    sqlx::query(
        "INSERT INTO guild_config (guild_id, prefix, updated_at)
         VALUES (?, ?, CURRENT_TIMESTAMP)
         ON CONFLICT(guild_id) DO UPDATE SET prefix = excluded.prefix, updated_at = CURRENT_TIMESTAMP",
    )
        .bind(guild_id.to_string())
        .bind(&new_prefix)
        .execute(&ctx.data().db_pool)
        .await?;

    tracing::info!(
        guild = %guild_id,
        new_prefix = %new_prefix,
        admin = %ctx.author().name,
        "Prefix updated"
    );

    let message = tf(lang, TranslationKey::PrefixChanged, &[&new_prefix]);

    let embed = serenity::CreateEmbed::new()
        .description(message)
        .color(0x2ecc71); // Green

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
