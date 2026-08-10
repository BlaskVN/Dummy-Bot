use crate::i18n::Language;
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

#[poise::command(slash_command, subcommands("tracker"), guild_only)]
pub async fn valorant(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(
    slash_command,
    subcommands("tracker_set", "tracker_remove", "tracker_view"),
    guild_only
)]
pub async fn tracker(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(rename = "set", slash_command, guild_only)]
pub async fn tracker_set(
    ctx: Context<'_>,
    #[description = "Your Tracker Network VALORANT profile URL"] url: String,
) -> Result<(), Error> {
    let language = language(ctx).await?;
    let response = match crate::valorant_tracker::set_profile(
        &ctx.data().db_pool,
        ctx.author().id,
        &url,
    )
    .await
    {
        Ok(_) => text(language, Text::Saved),
        Err(_) => text(language, Text::Invalid),
    };
    send_private(ctx, response).await?;
    Ok(())
}

#[poise::command(rename = "remove", slash_command, guild_only)]
pub async fn tracker_remove(ctx: Context<'_>) -> Result<(), Error> {
    let language = language(ctx).await?;
    let removed =
        crate::valorant_tracker::remove_profile(&ctx.data().db_pool, ctx.author().id).await?;
    send_private(
        ctx,
        text(
            language,
            if removed {
                Text::Removed
            } else {
                Text::Missing
            },
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(rename = "view", slash_command, guild_only)]
pub async fn tracker_view(
    ctx: Context<'_>,
    #[description = "A current member of this server"] member: Option<serenity::User>,
) -> Result<(), Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    let language = ctx.data().language(guild_id).await;
    let target = member.as_ref().unwrap_or_else(|| ctx.author());
    if target.id != ctx.author().id && guild_id.member(ctx.http(), target.id).await.is_err() {
        send_private(ctx, text(language, Text::NotMember)).await?;
        return Ok(());
    }
    let Some(url) = crate::valorant_tracker::profile(&ctx.data().db_pool, target.id).await? else {
        send_private(ctx, text(language, Text::Missing)).await?;
        return Ok(());
    };
    let embed = serenity::CreateEmbed::new()
        .title(format!("{} — VALORANT Tracker", target.name))
        .description(text(language, Text::Unverified))
        .url(&url);
    ctx.send(
        poise::CreateReply::default()
            .embed(embed)
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new_link(url).label(text(language, Text::Open)),
            ])])
            .allowed_mentions(serenity::CreateAllowedMentions::new()),
    )
    .await?;
    Ok(())
}

async fn language(ctx: Context<'_>) -> Result<Language, Error> {
    let guild_id = ctx
        .guild_id()
        .ok_or_else(|| anyhow::anyhow!("Not in a guild"))?;
    Ok(ctx.data().language(guild_id).await)
}

async fn send_private(ctx: Context<'_>, content: &'static str) -> Result<(), serenity::Error> {
    ctx.send(
        poise::CreateReply::default()
            .content(content)
            .ephemeral(true)
            .allowed_mentions(serenity::CreateAllowedMentions::new()),
    )
    .await?;
    Ok(())
}

#[derive(Clone, Copy)]
#[repr(usize)]
enum Text {
    Saved,
    Invalid,
    Removed,
    Missing,
    NotMember,
    Unverified,
    Open,
}

#[cfg(test)]
const TEXT_KEYS: [Text; 7] = [
    Text::Saved,
    Text::Invalid,
    Text::Removed,
    Text::Missing,
    Text::NotMember,
    Text::Unverified,
    Text::Open,
];

fn text(language: Language, key: Text) -> &'static str {
    const EN: [&str; 7] = [
        "Your global Tracker profile link was saved.",
        "Use a valid HTTPS tracker.gg VALORANT Riot profile overview URL.",
        "Your global Tracker profile link was removed.",
        "No Tracker profile link is saved for that member.",
        "That user is not a current member of this server.",
        "User-provided external link — not verified by Riot or this bot",
        "Open Tracker profile",
    ];
    const VI: [&str; 7] = [
        "Đã lưu liên kết hồ sơ Tracker toàn cục của bạn.",
        "Hãy dùng URL tổng quan hồ sơ Riot VALORANT tracker.gg hợp lệ qua HTTPS.",
        "Đã xóa liên kết hồ sơ Tracker toàn cục của bạn.",
        "Thành viên này chưa lưu liên kết hồ sơ Tracker.",
        "Người dùng đó hiện không phải thành viên của server này.",
        "Liên kết ngoài do người dùng cung cấp — Riot và bot này không xác minh",
        "Mở hồ sơ Tracker",
    ];
    const JA: [&str; 7] = [
        "グローバルTrackerプロフィールリンクを保存しました。",
        "有効なHTTPSのtracker.gg VALORANT Riotプロフィール概要URLを使用してください。",
        "グローバルTrackerプロフィールリンクを削除しました。",
        "そのメンバーのTrackerプロフィールリンクは保存されていません。",
        "そのユーザーは現在このサーバーのメンバーではありません。",
        "ユーザー提供の外部リンク — Riotおよびこのボットは未確認",
        "Trackerプロフィールを開く",
    ];
    match language {
        Language::English => EN[key as usize],
        Language::Vietnamese => VI[key as usize],
        Language::Japanese => JA[key as usize],
    }
}

#[cfg(test)]
mod tests {
    use super::{TEXT_KEYS, Text, text};
    use crate::i18n::Language;

    #[test]
    fn labels_are_complete_and_disclose_unverified_external_links() {
        for language in [Language::English, Language::Vietnamese, Language::Japanese] {
            for key in TEXT_KEYS {
                assert!(!text(language, key).is_empty());
            }
        }
        assert_eq!(
            text(Language::English, Text::Unverified),
            "User-provided external link — not verified by Riot or this bot"
        );
    }
}
