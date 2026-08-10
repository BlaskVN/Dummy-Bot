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

    let description = render(&config);
    let reply = if let Some(filename) = config.qr_filename.as_deref()
        && let Some(path) = owned_qr_path(&ctx.data().config.data_directory, filename)
    {
        ui::embed_reply(
            ui::embed(ctx.data(), Tone::Primary)
                .description(description)
                .image(attachment_url(filename)),
        )
        .attachment(serenity::CreateAttachment::path(path).await?)
    } else {
        ui::reply_builder(ctx.data(), Tone::Primary, description)
    };
    ctx.send(reply).await?;
    Ok(())
}

fn attachment_url(filename: &str) -> String {
    format!("attachment://{filename}")
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
    use super::{attachment_url, render};
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

    #[test]
    fn donation_qr_uses_the_embed_attachment_url() {
        assert_eq!(
            attachment_url("donation-qr-1.png"),
            "attachment://donation-qr-1.png"
        );
    }
}
