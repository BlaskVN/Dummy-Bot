use crate::i18n::{TranslationKey, t, tf};
use crate::ui::{self, Tone};
use crate::valorant::{
    PlatformStatus, RiotRegion, StatusIncident, ValorantLeaderboardError, ValorantMatchesError,
    ValorantProfileError, ValorantStatusError,
};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// VALORANT player statistics and Guild leaderboard management.
#[poise::command(
    slash_command,
    subcommands("profile", "leaderboard", "matches", "status"),
    guild_only
)]
pub async fn valorant(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

async fn resolve_target_user(
    ctx: Context<'_>,
    member: Option<&serenity::Member>,
    guild_id: serenity::GuildId,
    lang: crate::i18n::Language,
) -> Result<Option<serenity::UserId>, Error> {
    match member {
        Some(m) => {
            if m.guild_id != guild_id {
                ui::reply(
                    ctx,
                    Tone::Error,
                    t(lang, TranslationKey::ValorantProfileNotGuildMember),
                )
                .await?;
                Ok(None)
            } else {
                Ok(Some(m.user.id))
            }
        }
        None => Ok(Some(ctx.author().id)),
    }
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

    let Some(target_user_id) = resolve_target_user(ctx, member.as_ref(), guild_id, lang).await?
    else {
        return Ok(());
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
        Err(ValorantProfileError::ApiUnauthorized) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::RiotApiUnauthorized),
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
        Err(ValorantProfileError::Database(err)) => return Err(err.into()),
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
        Err(ValorantLeaderboardError::ApiUnauthorized) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::RiotApiUnauthorized),
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
        Err(ValorantLeaderboardError::Database(err)) => return Err(err.into()),
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

/// View recent VALORANT matches for yourself or another member.
#[poise::command(slash_command, guild_only)]
pub async fn matches(
    ctx: Context<'_>,
    #[description = "Member whose VALORANT matches to view (defaults to yourself)"] member: Option<
        serenity::Member,
    >,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    let target_user_id = match resolve_target_user(ctx, member.as_ref(), guild_id, lang).await? {
        Some(id) => id,
        None => return Ok(()),
    };

    let data = match ctx
        .data()
        .valorant_service()
        .get_recent_matches(guild_id, ctx.author().id, target_user_id, 5)
        .await
    {
        Ok(d) => d,
        Err(ValorantMatchesError::NotLinkedSelf) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantMatchesNotLinkedSelf),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantMatchesError::NotLinkedOther) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantMatchesNotLinkedOther),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantMatchesError::HiddenOther) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantMatchesHiddenOther),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantMatchesError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantMatchesForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantMatchesError::ApiUnauthorized) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::RiotApiUnauthorized),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantMatchesError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantProfileApiError),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantMatchesError::Database(err)) => return Err(err.into()),
    };

    let player_display = format!("{}#{}", data.game_name, data.tag_line);
    let title = tf(
        lang,
        TranslationKey::ValorantMatchesTitle,
        &[&player_display],
    );

    if data.matches.is_empty() {
        let embed = ui::embed(ctx.data(), Tone::Primary)
            .title(title)
            .description(t(lang, TranslationKey::ValorantMatchesEmpty));
        ctx.send(ui::embed_reply(embed)).await?;
        return Ok(());
    }

    let mut match_blocks = Vec::new();
    for m in &data.matches {
        let outcome_badge = if m.won {
            format!("🟢 **{}**", t(lang, TranslationKey::ValorantMatchesWon))
        } else {
            format!("🔴 **{}**", t(lang, TranslationKey::ValorantMatchesLost))
        };
        let score_str = tf(
            lang,
            TranslationKey::ValorantMatchesScore,
            &[&m.rounds_won, &m.rounds_lost],
        );
        let kda_str = tf(
            lang,
            TranslationKey::ValorantMatchesKda,
            &[&m.kills, &m.deaths, &m.assists],
        );
        let time_str = if m.game_start_millis > 0 {
            format!(" • <t:{}:R>", m.game_start_millis / 1000)
        } else {
            String::new()
        };

        match_blocks.push(format!(
            "{outcome_badge} — **{map}** ({mode}) • **{agent}**\n> {score_str} • {kda_str}{time_str}",
            map = m.map_name,
            mode = m.game_mode,
            agent = m.character,
        ));
    }

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(title)
        .description(match_blocks.join("\n\n"));

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

fn pick_incident_title(incident: &StatusIncident, lang: crate::i18n::Language) -> &str {
    let target_locales: &[&str] = match lang {
        crate::i18n::Language::English => &["en_US", "en_GB", "en"],
        crate::i18n::Language::Vietnamese => &["vi_VN", "vi", "en_US", "en"],
        crate::i18n::Language::Japanese => &["ja_JP", "ja", "en_US", "en"],
    };
    for target in target_locales {
        if let Some(item) = incident
            .titles
            .iter()
            .find(|t| t.locale.eq_ignore_ascii_case(target))
        {
            return &item.content;
        }
    }
    incident
        .titles
        .first()
        .map(|t| t.content.as_str())
        .unwrap_or("(No title)")
}

pub fn format_platform_status_content(
    status: &PlatformStatus,
    region: RiotRegion,
    lang: crate::i18n::Language,
) -> (Tone, String, String) {
    let title = tf(lang, TranslationKey::ValorantStatusTitle, &[&status.name]);
    let region_badge = region.as_str().to_ascii_uppercase();

    if status.maintenances.is_empty() && status.incidents.is_empty() {
        let desc = format!(
            "🟢 **{}**\n\n> **Region:** `{region_badge}`",
            t(lang, TranslationKey::ValorantStatusOperational)
        );
        (Tone::Success, title, desc)
    } else {
        let mut sections = Vec::new();
        sections.push(format!("> **Region:** `{region_badge}`"));

        if !status.incidents.is_empty() {
            let label = t(lang, TranslationKey::ValorantStatusIncidents);
            let mut inc_lines = vec![format!("**{label}**")];
            for inc in &status.incidents {
                let sev = inc.incident_severity.as_deref().unwrap_or("info");
                let incident_title = pick_incident_title(inc, lang);
                inc_lines.push(format!("⚠️ **[{sev}]** {incident_title}"));
            }
            sections.push(inc_lines.join("\n"));
        }

        if !status.maintenances.is_empty() {
            let label = t(lang, TranslationKey::ValorantStatusMaintenances);
            let mut maint_lines = vec![format!("**{label}**")];
            for m in &status.maintenances {
                let st = m.maintenance_status.as_deref().unwrap_or("scheduled");
                let maint_title = pick_incident_title(m, lang);
                maint_lines.push(format!("🛠️ **[{st}]** {maint_title}"));
            }
            sections.push(maint_lines.join("\n"));
        }

        (Tone::Warning, title, sections.join("\n\n"))
    }
}

/// Check real-time VALORANT server status and maintenance incidents.
#[poise::command(slash_command, guild_only)]
pub async fn status(
    ctx: Context<'_>,
    #[description = "Region to check (defaults to bot default)"] region: Option<RiotRegion>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;

    let riot_region = region.unwrap_or_else(|| {
        RiotRegion::try_parse(&ctx.data().config.riot_default_region).unwrap_or(RiotRegion::Ap)
    });

    let status_data = match ctx
        .data()
        .valorant_service()
        .get_platform_status(riot_region)
        .await
    {
        Ok(s) => s,
        Err(ValorantStatusError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantStatusForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantStatusError::ApiUnauthorized) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::RiotApiUnauthorized),
            )
            .await?;
            return Ok(());
        }
        Err(ValorantStatusError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::ValorantStatusApiError),
            )
            .await?;
            return Ok(());
        }
    };

    let (tone, title, description) =
        format_platform_status_content(&status_data, riot_region, lang);
    let embed = ui::embed(ctx.data(), tone)
        .title(title)
        .description(description);

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::valorant::riot_api::LocalizedContent;

    #[test]
    fn valorant_command_structure_is_valid() {
        let cmd = valorant();
        assert_eq!(cmd.name, "valorant");
        assert!(cmd.slash_action.is_some());
        assert_eq!(cmd.subcommands.len(), 4);

        let sub_names: Vec<_> = cmd.subcommands.iter().map(|s| s.name.as_str()).collect();
        assert!(sub_names.contains(&"profile"));
        assert!(sub_names.contains(&"leaderboard"));
        assert!(sub_names.contains(&"matches"));
        assert!(sub_names.contains(&"status"));

        let status_cmd = cmd.subcommands.iter().find(|s| s.name == "status").unwrap();
        let status_region = status_cmd
            .parameters
            .iter()
            .find(|p| p.name == "region")
            .unwrap();
        assert!(!status_region.required);
        assert_eq!(status_region.choices.len(), 6);
    }

    #[test]
    fn format_platform_status_content_operational() {
        let status = PlatformStatus {
            id: "VALORANT".to_string(),
            name: "VALORANT (AP)".to_string(),
            locales: vec!["en_US".to_string()],
            maintenances: vec![],
            incidents: vec![],
        };

        let (tone, title, desc) =
            format_platform_status_content(&status, RiotRegion::Ap, crate::i18n::Language::English);
        assert!(matches!(tone, Tone::Success));
        assert!(title.contains("VALORANT (AP)"));
        assert!(desc.contains("All systems operational"));
        assert!(desc.contains("AP"));
    }

    #[test]
    fn format_platform_status_content_with_incidents_and_maintenances() {
        let status = PlatformStatus {
            id: "VALORANT".to_string(),
            name: "VALORANT (EU)".to_string(),
            locales: vec!["en_US".to_string(), "vi_VN".to_string()],
            maintenances: vec![StatusIncident {
                id: 1,
                maintenance_status: Some("in_progress".to_string()),
                incident_severity: None,
                titles: vec![LocalizedContent {
                    locale: "en_US".to_string(),
                    content: "Scheduled server maintenance".to_string(),
                }],
            }],
            incidents: vec![StatusIncident {
                id: 2,
                maintenance_status: None,
                incident_severity: Some("critical".to_string()),
                titles: vec![
                    LocalizedContent {
                        locale: "en_US".to_string(),
                        content: "Ranked queue disabled".to_string(),
                    },
                    LocalizedContent {
                        locale: "vi_VN".to_string(),
                        content: "Hàng chờ xếp hạng tạm đóng".to_string(),
                    },
                ],
            }],
        };

        // English check
        let (tone, title, desc) =
            format_platform_status_content(&status, RiotRegion::Eu, crate::i18n::Language::English);
        assert!(matches!(tone, Tone::Warning));
        assert!(title.contains("VALORANT (EU)"));
        assert!(desc.contains("critical"));
        assert!(desc.contains("Ranked queue disabled"));
        assert!(desc.contains("in_progress"));
        assert!(desc.contains("Scheduled server maintenance"));

        // Vietnamese localization check
        let (_tone_vi, _title_vi, desc_vi) = format_platform_status_content(
            &status,
            RiotRegion::Eu,
            crate::i18n::Language::Vietnamese,
        );
        assert!(desc_vi.contains("Hàng chờ xếp hạng tạm đóng"));
    }
}
