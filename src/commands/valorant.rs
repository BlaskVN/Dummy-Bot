use crate::i18n::{TranslationKey, t, tf};
use crate::ui::{self, Tone};
use crate::valorant::{RiotApiError, RiotRegion};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;
use std::sync::Arc;

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

    let (target_user_id, is_self) = match &member {
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
            (m.user.id, m.user.id == ctx.author().id)
        }
        None => (ctx.author().id, true),
    };

    let account =
        match crate::valorant::get_linked_account(&ctx.data().db_pool, target_user_id).await? {
            Some(acc) => acc,
            None => {
                let key = if is_self {
                    TranslationKey::ValorantProfileNotLinkedSelf
                } else {
                    TranslationKey::ValorantProfileNotLinkedOther
                };
                ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
                return Ok(());
            }
        };

    let is_visible =
        crate::valorant::get_guild_visibility(&ctx.data().db_pool, guild_id, target_user_id)
            .await?;

    if !is_self && !is_visible {
        ui::reply(
            ctx,
            Tone::Warning,
            t(lang, TranslationKey::ValorantProfileHiddenOther),
        )
        .await?;
        return Ok(());
    }

    let stats = match ctx
        .data()
        .riot_api
        .get_player_ranked(account.region, &account.puuid)
        .await
    {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(
                err = %err,
                puuid = %account.puuid,
                user_id = %target_user_id,
                "Failed to load player ranked data"
            );
            let key = match err.downcast_ref::<RiotApiError>() {
                Some(RiotApiError::Forbidden) => TranslationKey::ValorantProfileForbidden,
                Some(RiotApiError::NotFound) => TranslationKey::ValorantProfileUnranked,
                _ => TranslationKey::ValorantProfileApiError,
            };
            ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
            return Ok(());
        }
    };

    let title = t(lang, TranslationKey::ValorantProfileTitle);
    let player_label = t(lang, TranslationKey::ValorantProfilePlayerLabel);
    let rank_label = t(lang, TranslationKey::ValorantProfileRankLabel);
    let rr_label = t(lang, TranslationKey::ValorantProfileRRLabel);
    let wins_label = t(lang, TranslationKey::ValorantProfileWinsLabel);

    let privacy_note = if is_self {
        if is_visible {
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
        game_name = account.game_name,
        tag_line = account.tag_line,
        region = account.region.as_str().to_ascii_uppercase(),
        user_id = target_user_id,
        rank = stats.tier.name(),
        rr = stats.ranked_rating,
        wins = stats.number_of_wins,
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

    let visible_accounts =
        crate::valorant::list_guild_visible_accounts(&ctx.data().db_pool, guild_id).await?;

    if visible_accounts.is_empty() {
        ui::reply(
            ctx,
            Tone::Primary,
            t(lang, TranslationKey::ValorantLeaderboardEmpty),
        )
        .await?;
        return Ok(());
    }

    let mut join_set = tokio::task::JoinSet::new();
    let riot_api = Arc::clone(&ctx.data().riot_api);
    for acc in visible_accounts {
        let api = Arc::clone(&riot_api);
        join_set.spawn(async move {
            let res = api.get_player_ranked(acc.region, &acc.puuid).await;
            (acc, res)
        });
    }

    let mut ranked_entries = Vec::new();
    let mut had_forbidden = false;
    while let Some(res) = join_set.join_next().await {
        if let Ok((acc, res)) = res {
            match res {
                Ok(stats) => ranked_entries.push((acc, stats)),
                Err(err) => {
                    if let Some(RiotApiError::Forbidden) = err.downcast_ref::<RiotApiError>() {
                        had_forbidden = true;
                    }
                    tracing::warn!(%err, user_id = %acc.user_id, "Failed to load player ranked data for leaderboard");
                }
            }
        }
    }

    if ranked_entries.is_empty() {
        let key = if had_forbidden {
            TranslationKey::ValorantProfileForbidden
        } else {
            TranslationKey::ValorantLeaderboardApiError
        };
        ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
        return Ok(());
    }

    ranked_entries.sort_by(|a, b| {
        b.1.tier
            .cmp(&a.1.tier)
            .then_with(|| b.1.ranked_rating.cmp(&a.1.ranked_rating))
            .then_with(|| b.1.number_of_wins.cmp(&a.1.number_of_wins))
    });

    let mut lines = Vec::new();
    for (idx, (acc, stats)) in ranked_entries.iter().enumerate().take(25) {
        let rank_pos = idx + 1;
        let medal = match rank_pos {
            1 => "🥇 ",
            2 => "🥈 ",
            3 => "🥉 ",
            _ => "",
        };

        lines.push(format!(
            "**#{rank_pos}** {medal}**{game_name}#{tag}** — **{tier}** ({rr} RR, {wins}W) • <@{user_id}>",
            game_name = acc.game_name,
            tag = acc.tag_line,
            tier = stats.tier.name(),
            rr = stats.ranked_rating,
            wins = stats.number_of_wins,
            user_id = acc.user_id
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

    let account = crate::valorant::get_linked_account(&ctx.data().db_pool, ctx.author().id).await?;
    if account.is_none() {
        ui::reply(
            ctx,
            Tone::Warning,
            t(lang, TranslationKey::ValorantVisibilityNotLinked),
        )
        .await?;
        return Ok(());
    }

    crate::valorant::set_guild_visibility(&ctx.data().db_pool, guild_id, ctx.author().id, true)
        .await?;

    ui::reply(
        ctx,
        Tone::Success,
        t(lang, TranslationKey::ValorantVisibilityEnabled),
    )
    .await?;
    Ok(())
}

/// Disable Guild Profile Visibility for this server.
#[poise::command(slash_command, guild_only)]
pub async fn disable(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    crate::valorant::set_guild_visibility(&ctx.data().db_pool, guild_id, ctx.author().id, false)
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

    let is_visible =
        crate::valorant::get_guild_visibility(&ctx.data().db_pool, guild_id, ctx.author().id)
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

    let trimmed = riot_id.trim();
    let Some((game_name, tag_line)) = trimmed.split_once('#') else {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::ValorantLinkInvalidFormat),
        )
        .await?;
        return Ok(());
    };

    let game_name = game_name.trim();
    let tag_line = tag_line.trim();
    if game_name.is_empty() || tag_line.is_empty() {
        ui::reply(
            ctx,
            Tone::Error,
            t(lang, TranslationKey::ValorantLinkInvalidFormat),
        )
        .await?;
        return Ok(());
    }

    let riot_region = match region {
        Some(r) => match RiotRegion::try_parse(&r) {
            Some(parsed) => parsed,
            None => {
                ui::reply(
                    ctx,
                    Tone::Error,
                    t(lang, TranslationKey::ValorantLinkInvalidRegion),
                )
                .await?;
                return Ok(());
            }
        },
        None => RiotRegion::Ap,
    };

    let linked_account = match ctx
        .data()
        .riot_api
        .get_account_by_riot_id(riot_region, game_name, tag_line)
        .await
    {
        Ok(acc) => acc,
        Err(err) => {
            if let Some(RiotApiError::NotFound) = err.downcast_ref::<RiotApiError>() {
                let full_id = format!("{game_name}#{tag_line}");
                let msg = tf(lang, TranslationKey::ValorantLinkNotFound, &[&full_id]);
                ui::reply(ctx, Tone::Error, msg).await?;
                return Ok(());
            }

            tracing::warn!(
                %err,
                %game_name,
                %tag_line,
                "Failed to verify Riot account with Riot API"
            );
            ui::reply(
                ctx,
                Tone::Error,
                t(lang, TranslationKey::ValorantLinkApiError),
            )
            .await?;
            return Ok(());
        }
    };

    let linked = crate::valorant::set_linked_account(
        &ctx.data().db_pool,
        ctx.author().id,
        &linked_account.puuid,
        &linked_account.game_name,
        &linked_account.tag_line,
        riot_region,
    )
    .await?;

    let full_id = format!("{}#{}", linked.game_name, linked.tag_line);
    let msg = tf(lang, TranslationKey::ValorantLinkSuccess, &[&full_id]);

    ui::reply(ctx, Tone::Success, msg).await?;
    Ok(())
}

/// Unlink your Riot account from your Discord profile.
#[poise::command(slash_command, guild_only)]
pub async fn unlink(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };

    let removed =
        crate::valorant::remove_linked_account(&ctx.data().db_pool, ctx.author().id).await?;

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
