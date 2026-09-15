use crate::i18n::{TranslationKey, t, tf};
use crate::ui::{self, Tone};
use crate::valorant::{ValorantLinkError, ValorantVisibilityError};
use crate::{Context, Error};

/// Riot Games account linking and cross-game privacy settings (VALORANT & League of Legends).
#[poise::command(slash_command, subcommands("link", "unlink", "visibility"), guild_only)]
pub async fn riot(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Manage Guild Profile Visibility consent for your Linked Riot Account.
#[poise::command(slash_command, subcommands("enable", "disable", "status"), guild_only)]
pub async fn visibility(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enable Guild Profile Visibility for your Linked Riot Account in this Guild.
#[poise::command(slash_command, guild_only)]
pub async fn enable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    match ctx
        .data()
        .valorant_service()
        .enable_visibility(guild_id, ctx.author().id)
        .await
    {
        Ok(()) => {
            ui::reply(
                ctx,
                Tone::Success,
                t(lang, TranslationKey::ValorantVisibilityEnabled),
            )
            .await?;
        }
        Err(ValorantVisibilityError::NotLinked) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantVisibilityNotLinked),
            )
            .await?;
        }
        Err(ValorantVisibilityError::Database(err)) => return Err(err.into()),
    }

    Ok(())
}

/// Disable Guild Profile Visibility for your Linked Riot Account in this Guild.
#[poise::command(slash_command, guild_only)]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    ctx.data()
        .valorant_service()
        .disable_visibility(guild_id, ctx.author().id)
        .await?;

    ui::reply(
        ctx,
        Tone::Success,
        t(lang, TranslationKey::ValorantVisibilityDisabled),
    )
    .await?;
    Ok(())
}

/// Check your current Guild Profile Visibility status in this Guild.
#[poise::command(slash_command, guild_only)]
pub async fn status(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    let is_visible = ctx
        .data()
        .valorant_service()
        .get_visibility_status(guild_id, ctx.author().id)
        .await?;

    let key = if is_visible {
        TranslationKey::ValorantVisibilityStatusEnabled
    } else {
        TranslationKey::ValorantVisibilityStatusDisabled
    };

    ui::reply(ctx, Tone::Primary, t(lang, key)).await?;
    Ok(())
}

/// Link a Riot account to your Discord profile for VALORANT and League of Legends.
#[poise::command(slash_command, guild_only)]
pub async fn link(
    ctx: Context<'_>,
    #[description = "Your Riot ID in GameName#TAG format (e.g. TenZ#0001, Faker#KR1)"]
    riot_id: String,
    #[description = "Your account region (defaults to ap)"] region: Option<
        crate::valorant::RiotRegion,
    >,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    match ctx
        .data()
        .valorant_service()
        .link_account(ctx.author().id, &riot_id, region.map(|r| r.as_str()))
        .await
    {
        Ok(linked) => {
            let full_id = format!("{}#{}", linked.game_name, linked.tag_line);
            let msg = tf(lang, TranslationKey::ValorantLinkSuccess, &[&full_id]);
            ui::reply(ctx, Tone::Success, msg).await?;
        }
        Err(ValorantLinkError::InvalidFormat) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantLinkInvalidFormat),
            )
            .await?;
        }
        Err(ValorantLinkError::InvalidRegion) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantLinkInvalidRegion),
            )
            .await?;
        }
        Err(ValorantLinkError::AccountNotFound(full_id)) => {
            let msg = tf(lang, TranslationKey::ValorantLinkNotFound, &[&full_id]);
            ui::reply(ctx, Tone::Error, msg).await?;
        }
        Err(ValorantLinkError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantProfileForbidden),
            )
            .await?;
        }
        Err(ValorantLinkError::ApiUnauthorized) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::RiotApiUnauthorized),
            )
            .await?;
        }
        Err(ValorantLinkError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantLinkApiError),
            )
            .await?;
        }
        Err(ValorantLinkError::Database(err)) => return Err(err.into()),
    }

    Ok(())
}

/// Unlink your Riot account from your Discord profile.
#[poise::command(slash_command, guild_only)]
pub async fn unlink(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let removed = ctx
        .data()
        .valorant_service()
        .unlink_account(ctx.author().id)
        .await?;

    if removed {
        ui::reply(
            ctx,
            Tone::Success,
            t(lang, TranslationKey::ValorantUnlinkSuccess),
        )
        .await?;
    } else {
        ui::reply(
            ctx,
            Tone::Warning,
            t(lang, TranslationKey::ValorantUnlinkNotFound),
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riot_command_structure_is_valid() {
        let cmd = riot();
        assert_eq!(cmd.name, "riot");
        assert!(cmd.slash_action.is_some());
        assert_eq!(cmd.subcommands.len(), 3);

        let sub_names: Vec<_> = cmd.subcommands.iter().map(|s| s.name.as_str()).collect();
        assert!(sub_names.contains(&"link"));
        assert!(sub_names.contains(&"unlink"));
        assert!(sub_names.contains(&"visibility"));

        let vis_cmd = cmd
            .subcommands
            .iter()
            .find(|s| s.name == "visibility")
            .unwrap();
        assert_eq!(vis_cmd.subcommands.len(), 3);
        let vis_subs: Vec<_> = vis_cmd
            .subcommands
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(vis_subs.contains(&"enable"));
        assert!(vis_subs.contains(&"disable"));
        assert!(vis_subs.contains(&"status"));

        let link_cmd = cmd.subcommands.iter().find(|s| s.name == "link").unwrap();
        let region_param = link_cmd
            .parameters
            .iter()
            .find(|p| p.name == "region")
            .unwrap();
        assert!(!region_param.required);
        assert_eq!(region_param.choices.len(), 6);
    }
}
