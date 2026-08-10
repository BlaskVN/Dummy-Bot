use crate::commands::donation::owned_qr_path;
use crate::database::{DonationConfig, load_donation_config};
use crate::i18n::{TranslationKey, t};
use crate::ui::{self, Tone};
use crate::{Context, Error};
use poise::serenity_prelude as serenity;

/// Show the server's configured donation information.
#[poise::command(slash_command, user_cooldown = 5)]
pub async fn donate(ctx: Context<'_>) -> Result<(), Error> {
    let lang = match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    };
    let Some(config) = load_donation_config(&ctx.data().db_pool).await? else {
        ui::reply(
            ctx,
            Tone::Warning,
            t(lang, TranslationKey::DonateNotConfigured),
        )
        .await?;
        return Ok(());
    };

    let mut reply = ui::reply_builder(ctx.data(), Tone::Primary, render(&config));
    if let Some(filename) = config.qr_filename.as_deref()
        && let Some(path) = owned_qr_path(&ctx.data().config.data_directory, filename)
    {
        reply = reply.attachment(serenity::CreateAttachment::path(path).await?);
    }
    ctx.send(reply).await?;
    Ok(())
}

fn render(config: &DonationConfig) -> String {
    [config.message.as_deref(), config.url.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::render;
    use crate::database::DonationConfig;

    #[test]
    fn renders_empty_text_and_qr_configurations() {
        assert_eq!(
            render(&DonationConfig {
                message: None,
                url: None,
                qr_filename: None
            }),
            ""
        );
        assert_eq!(
            render(&DonationConfig {
                message: Some("Support <@1>".into()),
                url: None,
                qr_filename: None
            }),
            "Support <@1>"
        );
        assert_eq!(
            render(&DonationConfig {
                message: Some("Thanks".into()),
                url: Some("https://example.com".into()),
                qr_filename: Some("donation-qr-1.png".into())
            }),
            "Thanks\nhttps://example.com"
        );
    }
}
