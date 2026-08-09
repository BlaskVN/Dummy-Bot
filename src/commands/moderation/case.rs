use crate::i18n::{Language, TranslationKey, t, tf};
use crate::moderation_cases::{ModerationCaseRecord, get_case, list_cases, void_case};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

const PAGE_SIZE: i64 = 10;

#[poise::command(
    rename = "case",
    slash_command,
    subcommands("view", "list", "void"),
    guild_only
)]
pub async fn cases(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, guild_only, required_permissions = "MODERATE_MEMBERS")]
pub async fn view(
    ctx: Context<'_>,
    #[description = "Case number"] number: i64,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    let Some(record) = get_case(&ctx.data().db_pool, guild_id, number).await? else {
        ctx.say(t(lang, TranslationKey::ModerationCaseNotFound))
            .await?;
        return Ok(());
    };
    send_safe(ctx, render_case(lang, &record)).await
}

#[poise::command(slash_command, guild_only, required_permissions = "MODERATE_MEMBERS")]
pub async fn list(
    ctx: Context<'_>,
    #[description = "Only cases for this member"] member: Option<serenity::Member>,
    #[description = "Page number"]
    #[min = 1]
    page: Option<u32>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    let page = page.unwrap_or(1);
    let records = list_cases(
        &ctx.data().db_pool,
        guild_id,
        member.map(|member| member.user.id),
        i64::from(page - 1) * PAGE_SIZE,
        PAGE_SIZE,
    )
    .await?;
    if records.is_empty() {
        ctx.say(t(lang, TranslationKey::ModerationCaseListEmpty))
            .await?;
        return Ok(());
    }
    let rows = records
        .iter()
        .map(|record| {
            tf(
                lang,
                TranslationKey::ModerationCaseListRow,
                &[
                    &record.case_number,
                    &action_name(lang, &record.action),
                    &record.target_user_id,
                    &status_name(lang, &record.status),
                ],
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let message = tf(lang, TranslationKey::ModerationCaseList, &[&page, &rows]);
    send_safe(ctx, message).await
}

#[poise::command(
    slash_command,
    guild_only,
    default_member_permissions = "MANAGE_GUILD",
    required_permissions = "MANAGE_GUILD"
)]
pub async fn void(
    ctx: Context<'_>,
    #[description = "Case number"] number: i64,
    #[description = "Why this case is invalid"] reason: String,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let lang = ctx.data().language(guild_id).await;
    if reason.trim().is_empty() {
        ctx.say(t(lang, TranslationKey::ModerationReasonRequired))
            .await?;
        return Ok(());
    }
    if get_case(&ctx.data().db_pool, guild_id, number)
        .await?
        .is_none()
    {
        ctx.say(t(lang, TranslationKey::ModerationCaseNotFound))
            .await?;
        return Ok(());
    }
    if !void_case(
        &ctx.data().db_pool,
        guild_id,
        number,
        ctx.author().id,
        &reason,
    )
    .await?
    {
        ctx.say(t(lang, TranslationKey::ModerationCaseAlreadyVoided))
            .await?;
        return Ok(());
    }
    let message = tf(lang, TranslationKey::ModerationCaseVoided, &[&number]);
    send_safe(ctx, message).await
}

fn action_name(language: Language, action: &str) -> &'static str {
    t(
        language,
        match action {
            "warn" => TranslationKey::ModerationActionWarn,
            "kick" => TranslationKey::ModerationActionKick,
            "ban" => TranslationKey::ModerationActionBan,
            "timeout" => TranslationKey::ModerationActionTimeout,
            _ => TranslationKey::ModerationActionUnknown,
        },
    )
}

fn status_name(language: Language, status: &str) -> &'static str {
    t(
        language,
        if status == "voided" {
            TranslationKey::ModerationCaseStatusVoided
        } else {
            TranslationKey::ModerationCaseStatusActive
        },
    )
}

fn render_case(language: Language, record: &ModerationCaseRecord) -> String {
    let evidence = record
        .evidence_url
        .as_deref()
        .unwrap_or_else(|| t(language, TranslationKey::SettingsNotConfigured));
    let mut text = tf(
        language,
        TranslationKey::ModerationCaseView,
        &[
            &record.case_number,
            &action_name(language, &record.action),
            &record.target_user_id,
            &record.moderator_user_id,
            &record.reason,
            &record.created_at,
            &evidence,
            &status_name(language, &record.status),
        ],
    );
    if let (Some(actor), Some(reason), Some(at)) = (
        record.void_actor_user_id.as_deref(),
        record.void_reason.as_deref(),
        record.voided_at.as_deref(),
    ) {
        text.push_str(&tf(
            language,
            TranslationKey::ModerationCaseVoidDetails,
            &[&actor, &reason, &at],
        ));
    }
    text
}

async fn send_safe(ctx: Context<'_>, content: String) -> Result<(), Error> {
    ctx.send(
        poise::CreateReply::default()
            .content(content)
            .allowed_mentions(serenity::CreateAllowedMentions::new()),
    )
    .await?;
    Ok(())
}
