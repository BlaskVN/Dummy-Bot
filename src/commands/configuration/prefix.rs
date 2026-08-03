use crate::i18n::{TranslationKey, tf};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Set a custom command prefix for this server.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn setprefix(
    ctx: Context<'_>,
    #[description = "New prefix for the server"] new_prefix: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;

    let lang = ctx.data().language(guild_id).await;

    let length = new_prefix.chars().count();
    if length == 0 || length > ctx.data().config.prefix_max_chars {
        let message = tf(
            lang,
            TranslationKey::PrefixInvalidLength,
            &[&ctx.data().config.prefix_max_chars],
        );
        ctx.say(message).await?;
        return Ok(());
    }

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
        .color(ctx.data().config.colors.success);

    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
