pub mod formatting;
pub mod health;
pub mod models;
pub mod ports;
pub mod service;

pub use formatting::{
    build_bulk_delete_embeds, build_deleted_message_embed, build_edited_message_embed,
    build_metadata_embed, escape_markdown, fits_byte_budget, fits_embed_batch, markdown_message,
    markdown_quote, message_url, reply_field, truncate_text,
};
pub use health::{
    current_health, disable, enable, enable_with_outbox, format_status_description, get_config,
    get_log_channel, load_enabled_guilds, mark_warning_sent, reconcile, status,
};
pub use models::{
    CachedMessageRecord, DeletedMessageView, MessageLogConfig, MessageLogHealth,
    MessageLogOptions, PurgedMessageSummary,
};
pub use ports::{
    AttachmentFetcher, DiscordOutbox, HttpAttachmentFetcher, InMemoryOutbox, MessageLogOutbox,
    MockAttachmentFetcher, SentMessageRecord, is_discord_cdn,
};
pub use service::{MessageLogService, load_cached_message, save_cached_message};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::{self, init_db};
    use crate::i18n::Language;
    use poise::serenity_prelude as serenity;
    use serenity::{ChannelId, GuildId, MessageId};

    #[tokio::test]
    async fn full_lifecycle_in_memory_pipeline() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-ml-service-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        let guild_id = GuildId::new(10);
        let channel_id = ChannelId::new(20);
        let log_channel_id = ChannelId::new(30);

        // Enable logging
        enable(&pool, guild_id, log_channel_id, true).await.unwrap();

        let outbox = InMemoryOutbox::new();
        let fetcher = MockAttachmentFetcher::new(vec![1, 2, 3]);

        let service = MessageLogService::new(
            &pool,
            outbox.clone(),
            fetcher.clone(),
            MessageLogOptions::default(),
        );

        // 1. Simulate incoming message
        let record = CachedMessageRecord {
            message_id: "1001".to_string(),
            channel_id: channel_id.to_string(),
            guild_id: guild_id.to_string(),
            author_id: "555".to_string(),
            author_name: "Alice".to_string(),
            author_avatar_url: "https://cdn.discordapp.com/avatar.png".to_string(),
            is_bot: false,
            content: "Original text".to_string(),
            created_at: 1700000000,
            attachments_json: "[]".to_string(),
        };
        save_cached_message(&pool, &record).await.unwrap();

        // 2. Simulate edit
        let update_event: serenity::MessageUpdateEvent = serde_json::from_str(
            r#"{"id":"1001","channel_id":"20","guild_id":"10","content":"Edited text"}"#,
        )
        .unwrap();

        service
            .handle_message_update(Language::English, None, &update_event)
            .await;

        assert_eq!(outbox.sent_count().await, 1);
        let messages = outbox.get_messages().await;
        assert_eq!(messages[0].channel_id, log_channel_id);

        // 3. Simulate delete
        service
            .handle_message_delete(
                Language::English,
                channel_id,
                MessageId::new(1001),
                guild_id,
                None,
            )
            .await;

        assert_eq!(outbox.sent_count().await, 2);

        // Verify message deleted from DB cache
        let cached = load_cached_message(&pool, "1001").await.unwrap();
        assert!(cached.is_none());
    }

    #[tokio::test]
    async fn uncached_message_delete_sends_metadata_log() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-ml-metadata-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        let guild_id = GuildId::new(11);
        let channel_id = ChannelId::new(21);
        let log_channel_id = ChannelId::new(31);

        enable(&pool, guild_id, log_channel_id, true).await.unwrap();

        let outbox = InMemoryOutbox::new();
        let fetcher = MockAttachmentFetcher::new(vec![]);

        let service = MessageLogService::new(
            &pool,
            outbox.clone(),
            fetcher.clone(),
            MessageLogOptions::default(),
        );

        service
            .handle_message_delete(
                Language::English,
                channel_id,
                MessageId::new(9999),
                guild_id,
                None,
            )
            .await;

        assert_eq!(outbox.sent_count().await, 1);
        let messages = outbox.get_messages().await;
        assert_eq!(messages[0].channel_id, log_channel_id);
    }

    #[tokio::test]
    async fn bulk_delete_filters_bots_and_sends_summary() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-ml-bulk-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        let guild_id = GuildId::new(12);
        let channel_id = ChannelId::new(22);
        let log_channel_id = ChannelId::new(32);

        enable(&pool, guild_id, log_channel_id, true).await.unwrap();

        let outbox = InMemoryOutbox::new();
        let fetcher = MockAttachmentFetcher::new(vec![]);

        let service = MessageLogService::new(
            &pool,
            outbox.clone(),
            fetcher.clone(),
            MessageLogOptions::default(),
        );

        // User message
        save_cached_message(
            &pool,
            &CachedMessageRecord {
                message_id: "201".to_string(),
                channel_id: channel_id.to_string(),
                guild_id: guild_id.to_string(),
                author_id: "1".to_string(),
                author_name: "Bob".to_string(),
                author_avatar_url: "".to_string(),
                is_bot: false,
                content: "First message".to_string(),
                created_at: 100,
                attachments_json: "[]".to_string(),
            },
        )
        .await
        .unwrap();

        // Bot message
        save_cached_message(
            &pool,
            &CachedMessageRecord {
                message_id: "202".to_string(),
                channel_id: channel_id.to_string(),
                guild_id: guild_id.to_string(),
                author_id: "2".to_string(),
                author_name: "Bot".to_string(),
                author_avatar_url: "".to_string(),
                is_bot: true,
                content: "Spam".to_string(),
                created_at: 101,
                attachments_json: "[]".to_string(),
            },
        )
        .await
        .unwrap();

        let deleted_ids = vec![
            MessageId::new(201),
            MessageId::new(202),
            MessageId::new(203),
        ];

        service
            .handle_message_delete_bulk(
                Language::English,
                channel_id,
                &deleted_ids,
                guild_id,
                |_, _| None,
            )
            .await;

        assert_eq!(outbox.sent_count().await, 1);
        let messages = outbox.get_messages().await;
        assert_eq!(messages[0].channel_id, log_channel_id);
    }

    #[tokio::test]
    async fn purge_attachments_archived_within_limits() {
        let directory =
            std::env::temp_dir().join(format!("dummy-bot-ml-purge-test-{}", std::process::id()));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        let guild_id = GuildId::new(13);
        let log_channel_id = ChannelId::new(33);

        enable(&pool, guild_id, log_channel_id, true).await.unwrap();

        let outbox = InMemoryOutbox::new();
        let fetcher = MockAttachmentFetcher::new(vec![0u8; 100]);

        let service = MessageLogService::new(
            &pool,
            outbox.clone(),
            fetcher.clone(),
            MessageLogOptions {
                attachment_max_bytes: 500,
                purge_attachment_max_total_bytes: 2000,
                ..Default::default()
            },
        );

        let mut msg = serenity::Message::default();
        msg.author.bot = false;
        let att: serenity::Attachment = serde_json::from_str(
            r#"{"id":"1","filename":"img.png","size":100,"url":"https://cdn.discordapp.com/attachments/1/2/img.png","proxy_url":""}"#,
        )
        .unwrap();
        msg.attachments = vec![att];

        service.archive_purge_attachments(guild_id, &[msg]).await;

        assert_eq!(outbox.sent_count().await, 1);
        let messages = outbox.get_messages().await;
        assert_eq!(messages[0].channel_id, log_channel_id);
    }

    #[tokio::test]
    async fn skipped_attachment_notifies_log_channel() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-ml-skipped-att-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        let guild_id = GuildId::new(14);
        let channel_id = ChannelId::new(24);
        let log_channel_id = ChannelId::new(34);

        enable(&pool, guild_id, log_channel_id, true).await.unwrap();

        let outbox = InMemoryOutbox::new();
        // Fetcher with byte limit 50 bytes, but attachment is 500 bytes -> will fail
        let fetcher = MockAttachmentFetcher::new(vec![0u8; 500]);

        let service = MessageLogService::new(
            &pool,
            outbox.clone(),
            fetcher.clone(),
            MessageLogOptions {
                attachment_max_bytes: 50,
                purge_attachment_max_total_bytes: 1000,
                ..Default::default()
            },
        );

        let record = CachedMessageRecord {
            message_id: "7001".to_string(),
            channel_id: channel_id.to_string(),
            guild_id: guild_id.to_string(),
            author_id: "555".to_string(),
            author_name: "Alice".to_string(),
            author_avatar_url: "".to_string(),
            is_bot: false,
            content: "Message with large attachment".to_string(),
            created_at: 1700000000,
            attachments_json: serde_json::to_string(&vec![serde_json::json!({
                "id": "1",
                "filename": "heavy.zip",
                "size": 500,
                "url": "https://cdn.discordapp.com/heavy.zip",
                "proxy_url": ""
            })])
            .unwrap(),
        };
        save_cached_message(&pool, &record).await.unwrap();

        service
            .handle_message_delete(
                Language::English,
                channel_id,
                MessageId::new(7001),
                guild_id,
                None,
            )
            .await;

        // Expect 2 messages: 1 for the deleted message embed, and 1 warning notice for the skipped attachment
        assert_eq!(outbox.sent_count().await, 2);
    }

    #[tokio::test]
    async fn cached_message_crud_and_ttl_pruning() {
        let directory = std::env::temp_dir().join(format!(
            "dummy-bot-cached-message-test-{}",
            std::process::id()
        ));
        let pool = init_db(
            &format!("sqlite:{}/bot.db?mode=rwc", directory.display()),
            &directory,
        )
        .await
        .unwrap();

        let record = CachedMessageRecord {
            message_id: "100".into(),
            channel_id: "200".into(),
            guild_id: "300".into(),
            author_id: "400".into(),
            author_name: "Alice".into(),
            author_avatar_url: "https://cdn.discordapp.com/avatar.png".into(),
            is_bot: false,
            content: "Hello world".into(),
            created_at: 1000,
            attachments_json: "[]".into(),
        };

        save_cached_message(&pool, &record).await.unwrap();
        let loaded = load_cached_message(&pool, "100").await.unwrap();
        assert_eq!(loaded, Some(record));

        let pruned = database::prune_stale_cached_messages(&pool, 0).await.unwrap();
        assert_eq!(pruned, 1);
        assert!(load_cached_message(&pool, "100").await.unwrap().is_none());

        pool.close().await;
        std::fs::remove_dir_all(directory).unwrap();
    }
}
