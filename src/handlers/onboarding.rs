use crate::Data;
use crate::database::claim_guild_onboarding;
use crate::i18n::{TranslationKey, t};
use poise::serenity_prelude as serenity;

pub async fn handle_guild_create(
    ctx: &serenity::Context,
    guild: &serenity::Guild,
    is_new: Option<bool>,
    data: &Data,
) {
    if is_new != Some(true)
        || !claim_guild_onboarding(&data.db_pool, guild.id)
            .await
            .unwrap_or_else(|error| {
                tracing::error!(guild = %guild.id, %error, "Could not claim onboarding");
                false
            })
    {
        return;
    }

    let lang = data.language(guild.id).await;
    let message = t(lang, TranslationKey::OnboardingMessage);
    if let Some(installer) = installer(ctx, guild).await {
        if installer
            .dm(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(message)
                    .allowed_mentions(serenity::CreateAllowedMentions::new()),
            )
            .await
            .is_ok()
        {
            return;
        }
        tracing::warn!(guild = %guild.id, installer = %installer.id, "Could not DM onboarding installer");
    }

    let Some(channel_id) = guild.system_channel_id else {
        tracing::warn!(guild = %guild.id, "No onboarding delivery destination");
        return;
    };
    let bot_id = ctx.cache.current_user().id;
    let can_send = guild
        .channels
        .get(&channel_id)
        .zip(guild.members.get(&bot_id))
        .is_some_and(|(channel, member)| {
            guild.user_permissions_in(channel, member).contains(
                serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::SEND_MESSAGES,
            )
        });
    if !can_send
        || channel_id
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new()
                    .content(message)
                    .allowed_mentions(serenity::CreateAllowedMentions::new()),
            )
            .await
            .is_err()
    {
        tracing::warn!(guild = %guild.id, "Could not deliver onboarding to system channel");
    }
}

async fn installer(ctx: &serenity::Context, guild: &serenity::Guild) -> Option<serenity::User> {
    let bot_id = ctx.cache.current_user().id;
    let bot = guild.members.get(&bot_id)?;
    if !guild
        .member_permissions(bot)
        .contains(serenity::Permissions::VIEW_AUDIT_LOG)
    {
        return None;
    }
    let logs = guild
        .id
        .audit_logs(
            &ctx.http,
            Some(serenity::audit_log::Action::Member(
                serenity::audit_log::MemberAction::BotAdd,
            )),
            None,
            None,
            Some(10),
        )
        .await
        .ok()?;
    let entry = logs
        .entries
        .into_iter()
        .find(|entry| entry.target_id.is_some_and(|id| id.get() == bot_id.get()))?;
    entry.user_id.to_user(&ctx.http).await.ok()
}

#[cfg(test)]
mod tests {
    fn should_claim(is_new: Option<bool>, already_completed: bool) -> bool {
        is_new == Some(true) && !already_completed
    }

    #[test]
    fn only_new_unclaimed_guilds_attempt_onboarding() {
        assert!(should_claim(Some(true), false));
        assert!(!should_claim(Some(false), false)); // reconnect/cache hydration
        assert!(!should_claim(None, false)); // audit-log/cache unavailable state
        assert!(!should_claim(Some(true), true)); // DM or system delivery already failed
    }
}
