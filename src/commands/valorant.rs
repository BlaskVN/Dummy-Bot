use crate::i18n::{TranslationKey, t};
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// VALORANT Tracker Profile Link management.
#[poise::command(slash_command, subcommands("tracker"), guild_only)]
pub async fn valorant(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Manage and view VALORANT tracker.gg profile links.
#[poise::command(slash_command, subcommands("set", "view", "remove"), guild_only)]
pub async fn tracker(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Save your personal tracker.gg profile link.
#[poise::command(slash_command, guild_only)]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Your tracker.gg profile link"] url: String,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let normalized = match crate::valorant::tracker::parse_and_normalize_tracker_url(&url) {
        Ok(normalized) => normalized,
        Err(_) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantTrackerInvalidUrl),
            )
            .await?;
            return Ok(());
        }
    };

    crate::valorant::tracker::set_tracker_profile(
        &ctx.data().db_pool,
        ctx.author().id,
        &normalized,
    )
    .await?;

    let title = t(lang, TranslationKey::ValorantTrackerTitle);
    let success_text = t(lang, TranslationKey::ValorantTrackerSetSuccess);
    let link_label = t(lang, TranslationKey::ValorantTrackerLinkLabel);
    let notice_text = t(lang, TranslationKey::ValorantTrackerUnverifiedNotice);

    let embed = ui::embed(ctx.data(), Tone::Success)
        .title(title)
        .description(format!(
            "{success_text}\n\n**{link_label}:** <{normalized}>\n\n> ⚠️ *{notice_text}*"
        ));

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

/// View a member's tracker.gg profile link.
#[poise::command(slash_command, guild_only)]
pub async fn view(
    ctx: Context<'_>,
    #[description = "Member whose tracker profile to view (defaults to yourself)"] member: Option<
        serenity::Member,
    >,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    let target_user_id = match &member {
        Some(m) => {
            if m.guild_id != guild_id {
                ui::reply(
                    ctx,
                    Tone::Error,
                    t(lang, TranslationKey::ValorantTrackerNotGuildMember),
                )
                .await?;
                return Ok(());
            }
            m.user.id
        }
        None => ctx.author().id,
    };

    let profile =
        crate::valorant::tracker::get_tracker_profile(&ctx.data().db_pool, target_user_id).await?;

    match profile {
        Some(url) => {
            let title = t(lang, TranslationKey::ValorantTrackerTitle);
            let user_label = t(lang, TranslationKey::ValorantTrackerUserLabel);
            let link_label = t(lang, TranslationKey::ValorantTrackerLinkLabel);
            let notice_text = t(lang, TranslationKey::ValorantTrackerUnverifiedNotice);

            let embed = ui::embed(ctx.data(), Tone::Primary)
                .title(title)
                .description(format!("**{user_label}:** <@{target_user_id}>\n**{link_label}:** <{url}>\n\n> ⚠️ *{notice_text}*"));

            ctx.send(ui::embed_reply(embed)).await?;
        }
        None => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantTrackerNotFound),
            )
            .await?;
        }
    }

    Ok(())
}

/// Remove your saved tracker.gg profile link.
#[poise::command(slash_command, guild_only)]
pub async fn remove(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let removed =
        crate::valorant::tracker::remove_tracker_profile(&ctx.data().db_pool, ctx.author().id)
            .await?;

    if removed {
        ui::reply(
            ctx,
            Tone::Success,
            t(lang, TranslationKey::ValorantTrackerRemovedSuccess),
        )
        .await?;
    } else {
        ui::reply(
            ctx,
            Tone::Warning,
            t(lang, TranslationKey::ValorantTrackerNotFound),
        )
        .await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valorant_command_structure_is_valid() {
        let cmd = valorant();
        assert_eq!(cmd.name, "valorant");
        assert!(cmd.slash_action.is_some());
        assert_eq!(cmd.subcommands.len(), 1);

        let tracker_cmd = &cmd.subcommands[0];
        assert_eq!(tracker_cmd.name, "tracker");
        assert_eq!(tracker_cmd.subcommands.len(), 3);

        let tracker_subs: Vec<_> = tracker_cmd
            .subcommands
            .iter()
            .map(|s| s.name.as_str())
            .collect();
        assert!(tracker_subs.contains(&"set"));
        assert!(tracker_subs.contains(&"view"));
        assert!(tracker_subs.contains(&"remove"));
    }
}
