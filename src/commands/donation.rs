use crate::database::{clear_donation_config, save_donation_config};
use crate::i18n::{TranslationKey, t};
use crate::{Context, Error};
use anyhow::Context as _;
use poise::serenity_prelude as serenity;
use std::path::{Path, PathBuf};

#[poise::command(slash_command, subcommands("set", "clear"), owners_only, hide_in_help)]
pub async fn donation(_ctx: Context<'_>) -> Result<(), Error> {
    Ok(())
}

#[poise::command(slash_command, owners_only)]
pub async fn set(
    ctx: Context<'_>,
    #[description = "Donation message"] message: Option<String>,
    #[description = "HTTPS donation link"] url: Option<String>,
    #[description = "PNG or JPEG QR image"] qr_image: Option<serenity::Attachment>,
) -> Result<(), Error> {
    let lang = language(ctx).await;
    if !valid_update(message.as_deref(), url.as_deref(), qr_image.as_ref()) {
        ctx.say(t(lang, TranslationKey::DonationInvalidUpdate))
            .await?;
        return Ok(());
    }
    if let Some(url) = url.as_deref()
        && !valid_https_url(url)
    {
        ctx.say(t(lang, TranslationKey::DonationInvalidUrl)).await?;
        return Ok(());
    }

    let filename = match qr_image.as_ref() {
        Some(attachment) => match download_qr(ctx, attachment).await {
            Ok(filename) => Some(filename),
            Err(error) => {
                tracing::warn!(%error, "Rejected donation QR image");
                ctx.say(t(lang, TranslationKey::DonationInvalidImage))
                    .await?;
                return Ok(());
            }
        },
        None => None,
    };
    let old = match save_donation_config(
        &ctx.data().db_pool,
        message.as_deref(),
        url.as_deref(),
        filename.as_deref(),
    )
    .await
    {
        Ok(old) => old,
        Err(error) => {
            if let Some(filename) = filename.as_deref() {
                remove_owned_qr(&ctx.data().config.data_directory, filename).await;
            }
            return Err(error.into());
        }
    };
    if let Some(old) = old {
        remove_owned_qr(&ctx.data().config.data_directory, &old).await;
    }
    ctx.say(t(lang, TranslationKey::DonationSaved)).await?;
    Ok(())
}

#[poise::command(slash_command, owners_only)]
pub async fn clear(ctx: Context<'_>) -> Result<(), Error> {
    let lang = language(ctx).await;
    if let Some(filename) = clear_donation_config(&ctx.data().db_pool).await? {
        remove_owned_qr(&ctx.data().config.data_directory, &filename).await;
    }
    ctx.say(t(lang, TranslationKey::DonationCleared)).await?;
    Ok(())
}

async fn language(ctx: Context<'_>) -> crate::i18n::Language {
    match ctx.guild_id() {
        Some(guild_id) => ctx.data().language(guild_id).await,
        None => ctx.data().default_language(),
    }
}

fn valid_update(
    message: Option<&str>,
    url: Option<&str>,
    qr: Option<&serenity::Attachment>,
) -> bool {
    message.is_some_and(|value| !value.trim().is_empty()) || url.is_some() || qr.is_some()
}

fn valid_https_url(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| url.scheme() == "https" && url.host_str().is_some())
}

async fn download_qr(
    ctx: Context<'_>,
    attachment: &serenity::Attachment,
) -> anyhow::Result<String> {
    anyhow::ensure!(
        u64::from(attachment.size) <= ctx.data().config.attachment_max_bytes,
        "QR image exceeds byte limit"
    );
    anyhow::ensure!(
        matches!(
            attachment.content_type.as_deref(),
            Some("image/png" | "image/jpeg")
        ),
        "QR image MIME type is not PNG/JPEG"
    );
    let url = reqwest::Url::parse(&attachment.url)?;
    anyhow::ensure!(
        url.scheme() == "https"
            && matches!(
                url.host_str(),
                Some("cdn.discordapp.com" | "media.discordapp.net")
            ),
        "QR image URL is not Discord CDN"
    );
    let _permit = ctx.data().attachment_downloads.acquire().await?;
    let mut response = ctx
        .data()
        .attachment_client
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    anyhow::ensure!(
        response.content_length().unwrap_or(0) <= ctx.data().config.attachment_max_bytes,
        "QR image response exceeds byte limit"
    );
    let mut bytes = Vec::with_capacity(attachment.size as usize);
    while let Some(chunk) = response.chunk().await? {
        anyhow::ensure!(
            (bytes.len() as u64).saturating_add(chunk.len() as u64)
                <= ctx.data().config.attachment_max_bytes,
            "QR image body exceeds byte limit"
        );
        bytes.extend_from_slice(&chunk);
    }
    let extension = qr_extension(&bytes).context("QR image signature is not PNG/JPEG")?;
    let filename = format!(
        "donation-qr-{}.{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos(),
        extension
    );
    let directory = ctx.data().config.data_directory.join("donation");
    tokio::fs::create_dir_all(&directory).await?;
    let temporary = directory.join(format!(".{filename}.tmp"));
    tokio::fs::write(&temporary, bytes).await?;
    tokio::fs::rename(&temporary, directory.join(&filename)).await?;
    Ok(filename)
}

fn qr_extension(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some("jpg")
    } else {
        None
    }
}

pub(crate) fn owned_qr_path(data_directory: &Path, filename: &str) -> Option<PathBuf> {
    if filename.starts_with("donation-qr-")
        && matches!(
            Path::new(filename)
                .extension()
                .and_then(|part| part.to_str()),
            Some("png" | "jpg")
        )
        && Path::new(filename)
            .file_name()
            .is_some_and(|name| name == filename)
    {
        Some(data_directory.join("donation").join(filename))
    } else {
        None
    }
}

async fn remove_owned_qr(data_directory: &Path, filename: &str) {
    if let Some(path) = owned_qr_path(data_directory, filename)
        && let Err(error) = tokio::fs::remove_file(path).await
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%error, "Could not remove owned donation QR image");
    }
}

#[cfg(test)]
mod tests {
    use super::{qr_extension, valid_https_url, valid_update};

    #[test]
    fn validates_donation_inputs() {
        assert_eq!(qr_extension(b"\x89PNG\r\n\x1a\nrest"), Some("png"));
        assert_eq!(qr_extension(&[0xff, 0xd8, 0xff, 0xe0]), Some("jpg"));
        assert_eq!(qr_extension(b"not an image"), None);
        assert!(valid_https_url("https://example.com/donate"));
        assert!(!valid_https_url("http://example.com/donate"));
        assert!(!valid_update(Some("  "), None, None));
    }
}
