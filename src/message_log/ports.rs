use anyhow::Result;
use poise::serenity_prelude as serenity;
use serenity::ChannelId;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Seam for delivering message log entries and attachments to Discord.
pub trait MessageLogOutbox: Send + Sync {
    fn send_message(
        &self,
        channel_id: ChannelId,
        builder: serenity::CreateMessage,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Seam for fetching remote message attachments.
pub trait AttachmentFetcher: Send + Sync {
    fn fetch_attachment(
        &self,
        attachment: &serenity::Attachment,
        max_bytes: u64,
    ) -> impl std::future::Future<Output = Result<serenity::CreateAttachment>> + Send;
}

/// Live Discord outbox utilizing Serenity's HTTP client.
pub struct DiscordOutbox<'a> {
    pub http: &'a serenity::Http,
}

impl<'a> DiscordOutbox<'a> {
    pub fn new(http: &'a serenity::Http) -> Self {
        Self { http }
    }
}

impl<'a> MessageLogOutbox for DiscordOutbox<'a> {
    async fn send_message(
        &self,
        channel_id: ChannelId,
        builder: serenity::CreateMessage,
    ) -> Result<()> {
        channel_id.send_message(self.http, builder).await?;
        Ok(())
    }
}

/// Live attachment fetcher with byte limits and CDN host validation.
pub struct HttpAttachmentFetcher {
    client: reqwest::Client,
    semaphore: Option<Arc<tokio::sync::Semaphore>>,
}

impl HttpAttachmentFetcher {
    pub fn new(client: reqwest::Client, semaphore: Option<Arc<tokio::sync::Semaphore>>) -> Self {
        Self { client, semaphore }
    }
}

impl AttachmentFetcher for HttpAttachmentFetcher {
    async fn fetch_attachment(
        &self,
        attachment: &serenity::Attachment,
        max_bytes: u64,
    ) -> Result<serenity::CreateAttachment> {
        let _permit = if let Some(sem) = &self.semaphore {
            Some(sem.acquire().await?)
        } else {
            None
        };

        anyhow::ensure!(
            u64::from(attachment.size) <= max_bytes,
            "attachment exceeds {} bytes",
            max_bytes
        );
        let url = reqwest::Url::parse(&attachment.url)?;
        anyhow::ensure!(is_discord_cdn(&url), "attachment URL is not Discord CDN");

        let mut response = self.client.get(url).send().await?.error_for_status()?;
        anyhow::ensure!(
            response.content_length().unwrap_or(0) <= max_bytes,
            "attachment response exceeds byte limit"
        );

        let mut bytes = Vec::with_capacity(attachment.size as usize);
        while let Some(chunk) = response.chunk().await? {
            anyhow::ensure!(
                (bytes.len() as u64).saturating_add(chunk.len() as u64) <= max_bytes,
                "attachment body exceeds byte limit"
            );
            bytes.extend_from_slice(&chunk);
        }

        Ok(serenity::CreateAttachment::bytes(
            bytes,
            attachment.filename.clone(),
        ))
    }
}

pub fn is_discord_cdn(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("cdn.discordapp.com" | "media.discordapp.net")
        )
}

#[derive(Clone)]
pub struct SentMessageRecord {
    pub channel_id: ChannelId,
    pub builder: serenity::CreateMessage,
}

#[derive(Default, Clone)]
pub struct InMemoryOutbox {
    pub messages: Arc<Mutex<Vec<SentMessageRecord>>>,
}

impl InMemoryOutbox {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn sent_count(&self) -> usize {
        self.messages.lock().await.len()
    }

    pub async fn get_messages(&self) -> Vec<SentMessageRecord> {
        self.messages.lock().await.clone()
    }

    pub async fn clear(&self) {
        self.messages.lock().await.clear();
    }
}

impl MessageLogOutbox for InMemoryOutbox {
    async fn send_message(
        &self,
        channel_id: ChannelId,
        builder: serenity::CreateMessage,
    ) -> Result<()> {
        self.messages.lock().await.push(SentMessageRecord {
            channel_id,
            builder,
        });
        Ok(())
    }
}

pub struct MockAttachmentFetcher {
    pub canned_bytes: Vec<u8>,
}

impl MockAttachmentFetcher {
    pub fn new(canned_bytes: Vec<u8>) -> Self {
        Self { canned_bytes }
    }
}

impl AttachmentFetcher for MockAttachmentFetcher {
    async fn fetch_attachment(
        &self,
        attachment: &serenity::Attachment,
        max_bytes: u64,
    ) -> Result<serenity::CreateAttachment> {
        anyhow::ensure!(
            (self.canned_bytes.len() as u64) <= max_bytes,
            "mock attachment exceeds limit"
        );
        Ok(serenity::CreateAttachment::bytes(
            self.canned_bytes.clone(),
            attachment.filename.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cdn_host_matching() {
        assert!(is_discord_cdn(
            &"https://cdn.discordapp.com/attachments/1/2/test.png"
                .parse()
                .unwrap()
        ));
        assert!(is_discord_cdn(
            &"https://media.discordapp.net/attachments/1/2/test.png"
                .parse()
                .unwrap()
        ));
        assert!(!is_discord_cdn(
            &"http://cdn.discordapp.com/test.png".parse().unwrap()
        ));
        assert!(!is_discord_cdn(
            &"https://evil.com/test.png".parse().unwrap()
        ));
    }

    #[tokio::test]
    async fn in_memory_outbox_records_messages() {
        let outbox = InMemoryOutbox::new();
        assert_eq!(outbox.sent_count().await, 0);

        outbox
            .send_message(
                ChannelId::new(42),
                serenity::CreateMessage::new().content("hello"),
            )
            .await
            .unwrap();

        assert_eq!(outbox.sent_count().await, 1);
        let records = outbox.get_messages().await;
        assert_eq!(records[0].channel_id, ChannelId::new(42));
    }

    #[tokio::test]
    async fn mock_fetcher_respects_byte_limit() {
        let fetcher = MockAttachmentFetcher::new(vec![0u8; 100]);
        let att: serenity::Attachment = serde_json::from_str(
            r#"{"id":"1","filename":"sample.txt","size":100,"url":"https://cdn.discordapp.com/sample.txt","proxy_url":""}"#,
        )
        .unwrap();

        let ok = fetcher.fetch_attachment(&att, 100).await;
        assert!(ok.is_ok());

        let overflow = fetcher.fetch_attachment(&att, 50).await;
        assert!(overflow.is_err());
    }
}
