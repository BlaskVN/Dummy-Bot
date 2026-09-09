use crate::i18n::{TranslationKey, t, tf};
use crate::ui::{self, Tone};
use crate::valorant::{
    ValorantLeaderboardError, ValorantLinkError, ValorantProfileError, ValorantVisibilityError,
};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// VALORANT player statistics and Guild leaderboard management.
#[poise::command(
    slash_command,
    subcommands("profile", "leaderboard", "visibility", "link", "unlink"),
    guild_only
)]
pub async fn valorant(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// View a member's VALORANT rank and stats (defaults to yourself).
#[poise::command(slash_command, guild_only)]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "Member whose VALORANT profile to view (defaults to yourself)"] member: Option<
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
                    t(lang, TranslationKey::ValorantProfileNotGuildMember),
                )
                .await?;
                return Ok(());
            }
            m.user.id
        }
        None => ctx.author().id,
    };

    let profile = match ctx
        .data()
        .valorant_service()
        .get_profile(guild_id, ctx.author().id, target_user_id)
        .await
    {
        Ok(p) => p,
        Err(ValorantProfileError::NotLinked { is_self }) => {
            let key = if is_self {
                TranslationKey::ValorantProfileNotLinkedSelf
            } else {
                TranslationKey::ValorantProfileNotLinkedOther
            };
            ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
            return Ok(());
        }
        Err(ValorantProfileError::HiddenOther) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantProfileHiddenOther),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantProfileError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantProfileForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantProfileError::ApiUnranked) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantProfileUnranked),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantProfileError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantProfileApiError),
            )
            .await?;
            return Ok(());
        }
    };

    let title = t(lang, TranslationKey::ValorantProfileTitle);
    let player_label = t(lang, TranslationKey::ValorantProfilePlayerLabel);
    let rank_label = t(lang, TranslationKey::ValorantProfileRankLabel);
    let rr_label = t(lang, TranslationKey::ValorantProfileRRLabel);
    let wins_label = t(lang, TranslationKey::ValorantProfileWinsLabel);

    let privacy_note = if profile.is_self {
        if profile.is_visible {
            format!(
                "\n\n> {}",
                t(lang, TranslationKey::ValorantProfileVisibilityNoteVisible)
            )
        } else {
            format!(
                "\n\n> {}",
                t(lang, TranslationKey::ValorantProfileVisibilityNoteHidden)
            )
        }
    } else {
        String::new()
    };

    let description = format!(
        "**{player_label}:** {game_name}#{tag_line} (`{region}`)\n\
         **Discord:** <@{user_id}>\n\
         **{rank_label}:** {rank}\n\
         **{rr_label}:** {rr} RR\n\
         **{wins_label}:** {wins}{privacy_note}",
        game_name = profile.account.game_name,
        tag_line = profile.account.tag_line,
        region = profile.account.region.as_str().to_ascii_uppercase(),
        user_id = target_user_id,
        rank = profile.stats.tier.name(),
        rr = profile.stats.ranked_rating,
        wins = profile.stats.number_of_wins,
    );

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(title)
        .description(description);

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

/// View the Guild VALORANT leaderboard of visible members.
#[poise::command(slash_command, guild_only)]
pub async fn leaderboard(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    let ranked_entries = match ctx
        .data()
        .valorant_service()
        .get_guild_leaderboard(guild_id)
        .await
    {
        Ok(entries) => entries,
        Err(ValorantLeaderboardError::Empty) => {
            ui::reply(
                ctx,
                Tone::Primary,
                t(lang, TranslationKey::ValorantLeaderboardEmpty),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantLeaderboardError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantProfileForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantLeaderboardError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantLeaderboardApiError),
            )
            .await?;
            return Ok(());
        }
    };

    let mut lines = Vec::new();
    for (idx, entry) in ranked_entries.iter().enumerate().take(25) {
        let rank_pos = idx + 1;
        let medal = match rank_pos {
            1 => "🥇 ",
            2 => "🥈 ",
            3 => "🥉 ",
            _ => "",
        };

        lines.push(format!(
            "**#{rank_pos}** {medal}**{game_name}#{tag}** — **{tier}** ({rr} RR, {wins}W) • <@{user_id}>",
            game_name = entry.account.game_name,
            tag = entry.account.tag_line,
            tier = entry.stats.tier.name(),
            rr = entry.stats.ranked_rating,
            wins = entry.stats.number_of_wins,
            user_id = entry.account.user_id
        ));
    }

    let title = t(lang, TranslationKey::ValorantLeaderboardTitle);
    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(title)
        .description(lines.join("\n"));

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

/// Manage Guild Profile Visibility consent for this server.
#[poise::command(slash_command, subcommands("enable", "disable", "status"), guild_only)]
pub async fn visibility(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

/// Enable Guild Profile Visibility for this server.
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

/// Disable Guild Profile Visibility for this server.
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

/// Check your current Guild Profile Visibility status in this server.
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

/// Link a Riot account to your Discord profile.
#[poise::command(slash_command, guild_only)]
pub async fn link(
    ctx: Context<'_>,
    #[description = "Your Riot ID in GameName#TAG format (e.g. TenZ#0001)"] riot_id: String,
    #[description = "Your account region (ap, na, eu, kr, latam, br)"] region: Option<String>,
) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    match ctx
        .data()
        .valorant_service()
        .link_account(ctx.author().id, &riot_id, region.as_deref())
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
        Err(ValorantLinkError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantLinkApiError),
            )
            .await?;
        }
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
    fn valorant_command_structure_is_valid() {
        let cmd = valorant();
        assert_eq!(cmd.name, "valorant");
        assert!(cmd.slash_action.is_some());
        assert_eq!(cmd.subcommands.len(), 5);

        let sub_names: Vec<_> = cmd.subcommands.iter().map(|s| s.name.as_str()).collect();
        assert!(sub_names.contains(&"profile"));
        assert!(sub_names.contains(&"leaderboard"));
        assert!(sub_names.contains(&"visibility"));
        assert!(sub_names.contains(&"link"));
        assert!(sub_names.contains(&"unlink"));

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
    }
}
