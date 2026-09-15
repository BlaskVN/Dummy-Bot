use crate::i18n::{Language, TranslationKey, t, tf};
use crate::lol::{LolMasteryError, LolMatchesError, LolProfileError};
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// League of Legends player statistics and champion masteries.
#[poise::command(
    slash_command,
    subcommands("profile", "matches", "mastery"),
    guild_only
)]
pub async fn lol(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

async fn resolve_target_user(
    ctx: Context<'_>,
    member: Option<&serenity::Member>,
    guild_id: serenity::GuildId,
    lang: Language,
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

/// View a member's League of Legends profile and ranked stats (defaults to yourself).
#[poise::command(slash_command, guild_only)]
pub async fn profile(
    ctx: Context<'_>,
    #[description = "Member whose League of Legends profile to view (defaults to yourself)"]
    member: Option<serenity::Member>,
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
        .lol_service()
        .get_profile(guild_id, ctx.author().id, target_user_id)
        .await
    {
        Ok(p) => p,
        Err(LolProfileError::NotLinked { is_self }) => {
            let key = if is_self {
                TranslationKey::LolProfileNotLinkedSelf
            } else {
                TranslationKey::LolProfileNotLinkedOther
            };
            ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
            return Ok(());
        }
        Err(LolProfileError::HiddenOther) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolProfileHiddenOther),
            )
            .await?;
            return Ok(());
        }
        Err(LolProfileError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolProfileForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(LolProfileError::ApiNotFound) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolProfileNotFound),
            )
            .await?;
            return Ok(());
        }
        Err(LolProfileError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolProfileApiError),
            )
            .await?;
            return Ok(());
        }
        Err(LolProfileError::Database(err)) => return Err(err.into()),
    };

    let level_label = t(lang, TranslationKey::LolProfileSummonerLevel);
    let solo_label = t(lang, TranslationKey::LolProfileSoloDuo);
    let flex_label = t(lang, TranslationKey::LolProfileFlex);
    let unranked = t(lang, TranslationKey::LolProfileUnranked);

    let solo_line = match &profile.solo_entry {
        Some(entry) => {
            let rank_str = if entry.rank.is_empty() {
                entry.tier.name().to_string()
            } else {
                format!("{} {}", entry.tier.name(), entry.rank)
            };
            let wl_str = tf(
                lang,
                TranslationKey::LolProfileWinsLosses,
                &[&entry.wins, &entry.losses],
            );
            let wr_str = tf(
                lang,
                TranslationKey::LolProfileWinrate,
                &[&format!("{:.1}", entry.winrate())],
            );
            format!(
                "**{solo_label}:** {rank_str} ({} LP) • {wl_str} ({wr_str})",
                entry.league_points
            )
        }
        None => format!("**{solo_label}:** {unranked}"),
    };

    let flex_line = match &profile.flex_entry {
        Some(entry) => {
            let rank_str = if entry.rank.is_empty() {
                entry.tier.name().to_string()
            } else {
                format!("{} {}", entry.tier.name(), entry.rank)
            };
            let wl_str = tf(
                lang,
                TranslationKey::LolProfileWinsLosses,
                &[&entry.wins, &entry.losses],
            );
            let wr_str = tf(
                lang,
                TranslationKey::LolProfileWinrate,
                &[&format!("{:.1}", entry.winrate())],
            );
            format!(
                "**{flex_label}:** {rank_str} ({} LP) • {wl_str} ({wr_str})",
                entry.league_points
            )
        }
        None => format!("**{flex_label}:** {unranked}"),
    };

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
        "**Discord:** <@{target_user_id}>\n\
         **{level_label}:** {level}\n\n\
         {solo_line}\n\
         {flex_line}{privacy_note}",
        target_user_id = target_user_id,
        level_label = level_label,
        level = profile.summoner.summoner_level,
        solo_line = solo_line,
        flex_line = flex_line,
        privacy_note = privacy_note,
    );

    let icon_url = format!(
        "https://ddragon.leagueoflegends.com/cdn/14.18.1/img/profileicon/{}.png",
        profile.summoner.profile_icon_id
    );

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(format!(
            "{}#{}",
            profile.account.game_name, profile.account.tag_line
        ))
        .thumbnail(icon_url)
        .description(description);

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

/// View recent League of Legends matches for yourself or another member.
#[poise::command(slash_command, guild_only)]
pub async fn matches(
    ctx: Context<'_>,
    #[description = "Member whose League of Legends matches to view (defaults to yourself)"]
    member: Option<serenity::Member>,
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
        .lol_service()
        .get_matches(guild_id, ctx.author().id, target_user_id, 5)
        .await
    {
        Ok(d) => d,
        Err(LolMatchesError::NotLinked { is_self }) => {
            let key = if is_self {
                TranslationKey::LolMatchesNotLinkedSelf
            } else {
                TranslationKey::LolMatchesNotLinkedOther
            };
            ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
            return Ok(());
        }
        Err(LolMatchesError::HiddenOther) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolMatchesHiddenOther),
            )
            .await?;
            return Ok(());
        }
        Err(LolMatchesError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolMatchesForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(LolMatchesError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolMatchesApiError),
            )
            .await?;
            return Ok(());
        }
        Err(LolMatchesError::Database(err)) => return Err(err.into()),
    };

    let player_display = format!("{}#{}", data.account.game_name, data.account.tag_line);
    let title = tf(
        lang,
        TranslationKey::LolMatchesTitle,
        &[&player_display],
    );

    if data.matches.is_empty() {
        let embed = ui::embed(ctx.data(), Tone::Primary)
            .title(title)
            .description(t(lang, TranslationKey::LolMatchesEmpty));
        ctx.send(ui::embed_reply(embed)).await?;
        return Ok(());
    }

    let mut match_blocks = Vec::new();
    for m in &data.matches {
        let outcome_badge = if m.win {
            format!("🟢 **{}**", t(lang, TranslationKey::LolMatchesWon))
        } else {
            format!("🔴 **{}**", t(lang, TranslationKey::LolMatchesLost))
        };
        let champion_display = if m.champion_name.is_empty() {
            crate::lol::champion_name_by_id(m.champion_id)
        } else {
            m.champion_name.clone()
        };
        let kda_str = tf(
            lang,
            TranslationKey::LolMatchesKda,
            &[&m.kills, &m.deaths, &m.assists],
        );
        let cs_str = tf(lang, TranslationKey::LolMatchesCs, &[&m.cs]);
        let duration_mins = m.game_duration / 60;
        let duration_secs = m.game_duration % 60;
        let duration_str = tf(
            lang,
            TranslationKey::LolMatchesDuration,
            &[&duration_mins, &duration_secs],
        );
        let time_str = if m.game_start_millis > 0 {
            format!(" • <t:{}:R>", m.game_start_millis / 1000)
        } else {
            String::new()
        };

        let items_formatted: Vec<String> = m
            .items
            .iter()
            .filter(|&&id| id > 0)
            .map(|id| format!("`{id}`"))
            .collect();
        let items_line = if items_formatted.is_empty() {
            String::new()
        } else {
            format!("\n> Items: {}", items_formatted.join(" "))
        };

        match_blocks.push(format!(
            "{outcome_badge} — **{champ}** ({mode}) • {duration_str}{time_str}\n> {kda_str} • {cs_str}{items_line}",
            champ = champion_display,
            mode = m.game_mode,
        ));
    }

    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(title)
        .description(match_blocks.join("\n\n"));

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

/// View a member's top League of Legends champion masteries and total score.
#[poise::command(slash_command, guild_only)]
pub async fn mastery(
    ctx: Context<'_>,
    #[description = "Member whose champion masteries to view (defaults to yourself)"]
    member: Option<serenity::Member>,
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
        .lol_service()
        .get_mastery(guild_id, ctx.author().id, target_user_id, 5)
        .await
    {
        Ok(d) => d,
        Err(LolMasteryError::NotLinked { is_self }) => {
            let key = if is_self {
                TranslationKey::LolMasteryNotLinkedSelf
            } else {
                TranslationKey::LolMasteryNotLinkedOther
            };
            ui::reply(ctx, Tone::Warning, t(lang, key)).await?;
            return Ok(());
        }
        Err(LolMasteryError::HiddenOther) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolMasteryHiddenOther),
            )
            .await?;
            return Ok(());
        }
        Err(LolMasteryError::ApiForbidden) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolMasteryForbidden),
            )
            .await?;
            return Ok(());
        }
        Err(LolMasteryError::ApiError(_)) => {
            ui::reply(
                ctx,
                Tone::Warning,
                t(lang, TranslationKey::LolMasteryApiError),
            )
            .await?;
            return Ok(());
        }
        Err(LolMasteryError::Database(err)) => return Err(err.into()),
    };

    let player_display = format!("{}#{}", data.account.game_name, data.account.tag_line);
    let title = tf(
        lang,
        TranslationKey::LolMasteryTitle,
        &[&player_display],
    );

    let total_score_line = tf(
        lang,
        TranslationKey::LolMasteryTotalScore,
        &[&data.total_score],
    );

    if data.top_masteries.is_empty() {
        let embed = ui::embed(ctx.data(), Tone::Primary)
            .title(title)
            .description(format!(
                "**{total_score_line}**\n\n{}",
                t(lang, TranslationKey::LolMasteryEmpty)
            ));
        ctx.send(ui::embed_reply(embed)).await?;
        return Ok(());
    }

    let mut lines = Vec::new();
    for (idx, m) in data.top_masteries.iter().enumerate() {
        let rank_pos = idx + 1;
        let medal = match rank_pos {
            1 => "🥇 ",
            2 => "🥈 ",
            3 => "🥉 ",
            _ => "",
        };
        let champion_display = if m.champion_name.is_empty() {
            crate::lol::champion_name_by_id(m.champion_id)
        } else {
            m.champion_name.clone()
        };
        let level_str = tf(lang, TranslationKey::LolMasteryLevel, &[&m.champion_level]);
        let points_str = tf(lang, TranslationKey::LolMasteryPoints, &[&m.champion_points]);
        let time_str = if m.last_play_time > 0 {
            format!(" • <t:{}:R>", m.last_play_time / 1000)
        } else {
            String::new()
        };

        lines.push(format!(
            "**#{rank_pos}** {medal}**{name}** — {level_str} ({points_str}){time_str}",
            name = champion_display,
        ));
    }

    let description = format!("**{total_score_line}**\n\n{}", lines.join("\n"));
    let embed = ui::embed(ctx.data(), Tone::Primary)
        .title(title)
        .description(description);

    ctx.send(ui::embed_reply(embed)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::i18n::Language;

    #[test]
    fn lol_command_structure_and_subcommands() {
        let cmd = lol();
        assert_eq!(cmd.name, "lol");
        assert!(cmd.slash_action.is_some());
        assert_eq!(cmd.subcommands.len(), 3);

        let sub_names: Vec<&str> = cmd.subcommands.iter().map(|s| s.name.as_str()).collect();
        assert!(sub_names.contains(&"profile"));
        assert!(sub_names.contains(&"matches"));
        assert!(sub_names.contains(&"mastery"));

        for sub in &cmd.subcommands {
            assert!(sub.slash_action.is_some());
            assert_eq!(sub.parameters.len(), 1);
            let param = &sub.parameters[0];
            assert_eq!(param.name, "member");
            assert!(!param.required);
            assert!(
                !param.description.as_deref().unwrap_or("").is_empty(),
                "parameter description must not be empty"
            );
        }
    }

    #[test]
    fn lol_subcommand_descriptions_are_populated() {
        let cmd = lol();
        for sub in &cmd.subcommands {
            let desc = sub.description.as_deref().unwrap_or("");
            assert!(!desc.is_empty(), "subcommand {} missing description", sub.name);
        }
    }

    #[test]
    fn lol_translation_keys_resolve_in_all_languages() {
        let keys = [
            TranslationKey::LolProfileTitle,
            TranslationKey::LolProfileSummonerLevel,
            TranslationKey::LolProfileSoloDuo,
            TranslationKey::LolProfileFlex,
            TranslationKey::LolProfileUnranked,
            TranslationKey::LolProfileWinrate,
            TranslationKey::LolProfileWinsLosses,
            TranslationKey::LolProfileNotLinkedSelf,
            TranslationKey::LolProfileNotLinkedOther,
            TranslationKey::LolProfileHiddenOther,
            TranslationKey::LolProfileForbidden,
            TranslationKey::LolProfileNotFound,
            TranslationKey::LolProfileApiError,
            TranslationKey::LolMatchesTitle,
            TranslationKey::LolMatchesEmpty,
            TranslationKey::LolMatchesWon,
            TranslationKey::LolMatchesLost,
            TranslationKey::LolMatchesKda,
            TranslationKey::LolMatchesCs,
            TranslationKey::LolMatchesDuration,
            TranslationKey::LolMatchesNotLinkedSelf,
            TranslationKey::LolMatchesNotLinkedOther,
            TranslationKey::LolMatchesHiddenOther,
            TranslationKey::LolMatchesForbidden,
            TranslationKey::LolMatchesApiError,
            TranslationKey::LolMasteryTitle,
            TranslationKey::LolMasteryTotalScore,
            TranslationKey::LolMasteryLevel,
            TranslationKey::LolMasteryPoints,
            TranslationKey::LolMasteryEmpty,
            TranslationKey::LolMasteryNotLinkedSelf,
            TranslationKey::LolMasteryNotLinkedOther,
            TranslationKey::LolMasteryHiddenOther,
            TranslationKey::LolMasteryForbidden,
            TranslationKey::LolMasteryApiError,
        ];

        let languages = [Language::English, Language::Vietnamese, Language::Japanese];

        for lang in languages {
            for key in keys {
                let text = t(lang, key);
                assert_ne!(
                    text, "Translation missing",
                    "Missing translation for {:?} in {:?}",
                    key, lang
                );
                assert!(
                    !text.is_empty(),
                    "Empty translation for {:?} in {:?}",
                    key, lang
                );
            }
        }
    }

    #[test]
    fn translation_key_try_from_str_roundtrips_lol_keys() {
        let key_names = [
            "LolProfileTitle",
            "LolProfileSummonerLevel",
            "LolProfileSoloDuo",
            "LolProfileFlex",
            "LolProfileUnranked",
            "LolProfileWinrate",
            "LolProfileWinsLosses",
            "LolProfileNotLinkedSelf",
            "LolProfileNotLinkedOther",
            "LolProfileHiddenOther",
            "LolProfileForbidden",
            "LolProfileNotFound",
            "LolProfileApiError",
            "LolMatchesTitle",
            "LolMatchesEmpty",
            "LolMatchesWon",
            "LolMatchesLost",
            "LolMatchesKda",
            "LolMatchesCs",
            "LolMatchesDuration",
            "LolMatchesNotLinkedSelf",
            "LolMatchesNotLinkedOther",
            "LolMatchesHiddenOther",
            "LolMatchesForbidden",
            "LolMatchesApiError",
            "LolMasteryTitle",
            "LolMasteryTotalScore",
            "LolMasteryLevel",
            "LolMasteryPoints",
            "LolMasteryEmpty",
            "LolMasteryNotLinkedSelf",
            "LolMasteryNotLinkedOther",
            "LolMasteryHiddenOther",
            "LolMasteryForbidden",
            "LolMasteryApiError",
        ];

        for name in key_names {
            assert!(
                TranslationKey::try_from_str(name).is_some(),
                "try_from_str failed for {}",
                name
            );
        }
    }
}
