use crate::i18n::{TranslationKey, t, tf};
use crate::timezone;
use crate::ui::{self, Tone};
use crate::{Context, Error};

/// Configure the time zone used for this server.
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

/// Set this server's IANA time zone.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn set(
    ctx: Context<'_>,
    #[description = "IANA time zone, e.g. Asia/Ho_Chi_Minh"]
    #[autocomplete = "autocomplete_timezone"]
    iana_name: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    if timezone::set_timezone(&ctx.data().db_pool, guild_id, &iana_name)
        .await
        .is_err()
    {
        ui::reply(ctx, Tone::Error, t(lang, TranslationKey::TimezoneInvalid)).await?;
        return Ok(());
    }

    let message = tf(lang, TranslationKey::TimezoneSet, &[&iana_name]);
    ui::reply(ctx, Tone::Success, message).await?;
    Ok(())
}

async fn autocomplete_timezone(_ctx: Context<'_>, partial: &str) -> Vec<String> {
    use chrono::Offset as _;

    let partial = partial.trim().to_lowercase();
    let now = chrono::Utc::now();

    let get_offset =
        |tz: chrono_tz::Tz| -> i32 { now.with_timezone(&tz).offset().fix().local_minus_utc() };

    if partial.is_empty() {
        let mut results = vec!["Asia/Ho_Chi_Minh".to_string()];

        let mut list: Vec<(i32, &'static str)> = chrono_tz::TZ_VARIANTS
            .iter()
            .map(|&tz| (get_offset(tz), tz.name()))
            .collect();

        // Sort by UTC offset descending (latest time to earliest time), then alphabetically
        list.sort_by(|(offset_a, name_a), (offset_b, name_b)| {
            offset_b.cmp(offset_a).then_with(|| name_a.cmp(name_b))
        });

        for (_, name) in list {
            if !results.iter().any(|r| r == name) {
                results.push(name.to_string());
            }
            if results.len() >= 25 {
                break;
            }
        }
        return results;
    }

    let mut matches: Vec<(i32, bool, &'static str)> = chrono_tz::TZ_VARIANTS
        .iter()
        .filter_map(|&tz| {
            let name = tz.name();
            let name_lower = name.to_lowercase();
            if name_lower.contains(&partial) {
                let starts = name_lower.starts_with(&partial);
                Some((get_offset(tz), starts, name))
            } else {
                None
            }
        })
        .collect();

    matches.sort_by(
        |(offset_a, starts_a, name_a), (offset_b, starts_b, name_b)| {
            starts_b
                .cmp(starts_a)
                .then_with(|| offset_b.cmp(offset_a))
                .then_with(|| name_a.cmp(name_b))
        },
    );

    matches
        .into_iter()
        .take(25)
        .map(|(_, _, name)| name.to_string())
        .collect()
}

/// Show this server's configured time zone.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn show(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    let value = timezone::get_timezone_name(&ctx.data().db_pool, guild_id).await?;
    ui::reply(
        ctx,
        Tone::Neutral,
        match value {
            Some(value) => tf(lang, TranslationKey::TimezoneCurrent, &[&value]),
            None => t(lang, TranslationKey::TimezoneNotConfigured).to_string(),
        },
    )
    .await?;
    Ok(())
}

/// Reset this server to the default time zone.
#[poise::command(slash_command, guild_only, required_permissions = "MANAGE_GUILD")]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    timezone::clear_timezone(&ctx.data().db_pool, guild_id).await?;
    ui::reply(ctx, Tone::Success, t(lang, TranslationKey::TimezoneCleared)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::Offset as _;

    #[test]
    fn utc_offset_sorting_places_latest_times_first() {
        let now = chrono::Utc::now();
        let get_offset =
            |tz: chrono_tz::Tz| -> i32 { now.with_timezone(&tz).offset().fix().local_minus_utc() };
        let tokyo_offset = get_offset(chrono_tz::Asia::Tokyo);
        let hcm_offset = get_offset(chrono_tz::Asia::Ho_Chi_Minh);
        let utc_offset = get_offset(chrono_tz::UTC);
        let ny_offset = get_offset(chrono_tz::America::New_York);

        assert!(tokyo_offset >= hcm_offset);
        assert!(hcm_offset > utc_offset);
        assert!(utc_offset > ny_offset);
    }
}
