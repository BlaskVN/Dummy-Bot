use anyhow::{Context, Result};
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, GuildId, MessageId, MessageUpdateEvent};
use sqlx::SqlitePool;

use crate::database;
use crate::i18n::{Language, TranslationKey, t};

use super::formatting::{
    build_bulk_delete_embeds, build_deleted_message_embed, build_edited_message_embed,
    build_metadata_embed, fits_byte_budget, reply_field,
};
use super::health::{self, current_health, mark_warning_sent, reconcile};
use super::models::{
    CachedMessageRecord, DeletedMessageView, EditedMessageView, MessageLogHealth,
    MessageLogOptions, PurgedMessageSummary,
};
use super::ports::{AttachmentFetcher, MessageLogOutbox};

pub struct MessageLogService<'a, O: MessageLogOutbox, F: AttachmentFetcher> {
    pub pool: &'a SqlitePool,
    pub outbox: O,
    pub fetcher: F,
    pub options: MessageLogOptions,
}

impl<'a, O: MessageLogOutbox, F: AttachmentFetcher> MessageLogService<'a, O, F> {
    pub fn new(pool: &'a SqlitePool, outbox: O, fetcher: F, options: MessageLogOptions) -> Self {
        Self {
            pool,
            outbox,
            fetcher,
            options,
        }
    }

    /// Prune stale cached messages past their time-to-live threshold.
    pub async fn prune_stale_cache(pool: &SqlitePool, ttl_seconds: i64) {
        if let Err(error) = database::prune_stale_cached_messages(pool, ttl_seconds).await {
            tracing::error!(%error, "Failed to prune stale cached messages");
        }
    }

    /// Persist incoming non-bot guild message to SQLite cache for restart resilience.
    pub async fn save_message(pool: &SqlitePool, message: &serenity::Message) {
        let Some(guild_id) = message.guild_id else {
            return;
        };
        if message.author.bot {
            return;
        }
        let attachments_json = match poise::serenity_prelude::json::to_string(&message.attachments)
        {
            Ok(json) => json,
            Err(_) => "[]".to_string(),
        };
        let record = CachedMessageRecord {
            message_id: message.id.to_string(),
            channel_id: message.channel_id.to_string(),
            guild_id: guild_id.to_string(),
            author_id: message.author.id.to_string(),
            author_name: message.author.name.clone(),
            author_avatar_url: message.author.face(),
            is_bot: false,
            content: message.content.clone(),
            created_at: message.timestamp.unix_timestamp(),
            attachments_json,
        };
        if let Err(error) = save_cached_message(pool, &record).await {
            tracing::warn!(%error, "Failed to persist cached message to DB");
        }
    }

    /// Prune stale cached messages and reconcile degraded health for all active guilds.
    pub async fn reconcile_all_health<L, Fut>(&self, get_lang: L)
    where
        L: Fn(GuildId) -> Fut,
        Fut: std::future::Future<Output = Language>,
    {
        Self::prune_stale_cache(self.pool, 3 * 86400).await;

        let enabled_guilds = match health::load_enabled_guilds(self.pool).await {
            Ok(guilds) => guilds,
            Err(error) => {
                tracing::error!(%error, "Failed to load Message Log configurations");
                return;
            }
        };

        for (guild_id, log_channel_id) in enabled_guilds {
            match reconcile(self.pool, guild_id, self.options.message_content_enabled).await {
                Ok((_, true)) => {
                    let language = get_lang(guild_id).await;
                    let embed = serenity::CreateEmbed::new()
                        .title(t(language, TranslationKey::MessageLogDegradedWarning))
                        .color(self.options.warning_color);
                    let builder = serenity::CreateMessage::new()
                        .embed(embed)
                        .allowed_mentions(serenity::CreateAllowedMentions::new());

                    if self
                        .outbox
                        .send_message(log_channel_id, builder)
                        .await
                        .is_ok()
                        && let Err(error) = mark_warning_sent(self.pool, guild_id).await
                    {
                        tracing::error!(%guild_id, %error, "Failed to persist Message Log warning state");
                    }
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::error!(%guild_id, %error, "Failed to reconcile Message Log health");
                }
            }
        }
    }

    async fn resolve_log_channel(&self, guild_id: GuildId) -> Option<ChannelId> {
        match health::get_log_channel(self.pool, guild_id).await {
            Ok(Some(channel)) => Some(channel),
            Ok(None) => None,
            Err(e) => {
                tracing::error!("Failed to query message_log_config: {}", e);
                None
            }
        }
    }

    /// Handle message deletion events.
    pub async fn handle_message_delete(
        &self,
        lang: Language,
        channel_id: ChannelId,
        deleted_message_id: MessageId,
        guild_id: GuildId,
        ram_message: Option<serenity::Message>,
    ) {
        let Some(log_channel_id) = self.resolve_log_channel(guild_id).await else {
            return;
        };

        let serenity_msg;
        let (is_bot, author_id, author_face, content, sent_at_unix, attachments) =
            if let Some(message) = ram_message {
                let _ = database::delete_cached_message(self.pool, &deleted_message_id.to_string())
                    .await;
                let is_bot = message.author.bot;
                let author_id = message.author.id.to_string();
                let author_face = message.author.face();
                let content = message.content.clone();
                let sent_at_unix = message.timestamp.unix_timestamp();
                let attachments = message.attachments.clone();
                serenity_msg = Some(message);
                (
                    is_bot,
                    author_id,
                    author_face,
                    content,
                    sent_at_unix,
                    attachments,
                )
            } else if let Ok(Some(db_msg)) =
                load_cached_message(self.pool, &deleted_message_id.to_string()).await
            {
                let _ = database::delete_cached_message(self.pool, &deleted_message_id.to_string())
                    .await;
                let attachments: Vec<serenity::Attachment> =
                    poise::serenity_prelude::json::from_str(&db_msg.attachments_json)
                        .unwrap_or_default();
                serenity_msg = None;
                (
                    db_msg.is_bot,
                    db_msg.author_id,
                    db_msg.author_avatar_url,
                    db_msg.content,
                    db_msg.created_at,
                    attachments,
                )
            } else {
                let embed = build_metadata_embed(
                    lang,
                    channel_id,
                    TranslationKey::MessageDeleted,
                    self.options.warning_color,
                );
                let builder = serenity::CreateMessage::new()
                    .embed(embed)
                    .allowed_mentions(serenity::CreateAllowedMentions::new());
                let _ = self.outbox.send_message(log_channel_id, builder).await;
                return;
            };

        if is_bot {
            return;
        }

        let reply = serenity_msg
            .as_ref()
            .and_then(|msg| reply_field(lang, guild_id, msg));

        let view = DeletedMessageView {
            channel_id,
            author_id: &author_id,
            author_face: &author_face,
            content: &content,
            sent_at_unix,
            reply_info: reply,
        };

        let embed = build_deleted_message_embed(
            lang,
            &view,
            self.options.preview_chars,
            self.options.error_color,
        );

        let builder = serenity::CreateMessage::new()
            .embed(embed)
            .allowed_mentions(serenity::CreateAllowedMentions::new());

        if let Err(e) = self.outbox.send_message(log_channel_id, builder).await {
            tracing::error!("Failed to send deletion log: {}", e);
        }

        for attachment in &attachments {
            match self
                .fetcher
                .fetch_attachment(attachment, self.options.attachment_max_bytes)
                .await
            {
                Ok(file) => {
                    if let Err(e) = self.outbox.send_attachment(log_channel_id, file).await {
                        tracing::warn!(filename = %attachment.filename, %e, "Failed to send logged attachment");
                    }
                }
                Err(error) => {
                    tracing::warn!(
                        filename = %attachment.filename,
                        %error,
                        "Skipped unsafe or oversized attachment"
                    );
                    let notice = format!(
                        "⚠️ `[Attachment skipped: {} ({})]`",
                        attachment.filename, error
                    );
                    let notice_msg = serenity::CreateMessage::new()
                        .content(notice)
                        .allowed_mentions(serenity::CreateAllowedMentions::new());
                    let _ = self.outbox.send_message(log_channel_id, notice_msg).await;
                }
            }
        }
    }

    /// Handle message update (edit) events.
    pub async fn handle_message_update(
        &self,
        lang: Language,
        old_message: Option<&serenity::Message>,
        event: &MessageUpdateEvent,
    ) {
        let guild_id = match event.guild_id {
            Some(id) => id,
            None => return,
        };

        let serenity_msg;
        let (is_bot, author_id, author_face, old_content, sent_at_unix) =
            if let Some(message) = old_message {
                serenity_msg = Some(message);
                (
                    message.author.bot,
                    message.author.id.to_string(),
                    message.author.face(),
                    message.content.clone(),
                    message.timestamp.unix_timestamp(),
                )
            } else if let Ok(Some(db_msg)) =
                load_cached_message(self.pool, &event.id.to_string()).await
            {
                serenity_msg = None;
                (
                    db_msg.is_bot,
                    db_msg.author_id,
                    db_msg.author_avatar_url,
                    db_msg.content,
                    db_msg.created_at,
                )
            } else {
                // When Healthy, Discord sends update events for non-content edits (pins, embeds).
                // Without cached original content, diffing is impossible, so logging would trigger
                // false-positive edit notices. When Degraded, the bot has no content intent and falls
                // back to metadata-only notice.
                self.send_degraded_edit_fallback(lang, guild_id, event.channel_id)
                    .await;
                return;
            };

        if is_bot {
            return;
        }

        let new_content = match &event.content {
            Some(content) => content,
            None => {
                self.send_degraded_edit_fallback(lang, guild_id, event.channel_id)
                    .await;
                return;
            }
        };

        if old_content == *new_content {
            return;
        }

        if let Ok(Some(mut db_msg)) =
            load_cached_message(self.pool, &event.id.to_string()).await
        {
            db_msg.content = new_content.clone();
            let _ = save_cached_message(self.pool, &db_msg).await;
        }

        let Some(log_channel_id) = self.resolve_log_channel(guild_id).await else {
            return;
        };

        let reply = serenity_msg.and_then(|msg| reply_field(lang, guild_id, msg));

        let view = EditedMessageView {
            channel_id: event.channel_id,
            author_id: &author_id,
            author_face: &author_face,
            old_content: &old_content,
            new_content,
            sent_at_unix,
            reply_info: reply,
        };

        let embed = build_edited_message_embed(
            lang,
            &view,
            self.options.preview_chars,
            self.options.warning_color,
        );

        let builder = serenity::CreateMessage::new()
            .embed(embed)
            .allowed_mentions(serenity::CreateAllowedMentions::new());

        if let Err(e) = self.outbox.send_message(log_channel_id, builder).await {
            tracing::error!("Failed to send edit log: {}", e);
        }
    }

    /// Handle bulk message deletion events (purge/prune).
    pub async fn handle_message_delete_bulk<R>(
        &self,
        lang: Language,
        channel_id: ChannelId,
        deleted_message_ids: &[MessageId],
        guild_id: GuildId,
        get_ram_message: R,
    ) where
        R: Fn(ChannelId, MessageId) -> Option<serenity::Message>,
    {
        let Some(log_channel_id) = self.resolve_log_channel(guild_id).await else {
            return;
        };

        let mut cached_count = 0;
        let mut bot_count = 0;
        let mut user_messages: Vec<PurgedMessageSummary> = Vec::new();

        for &msg_id in deleted_message_ids {
            if let Some(msg) = get_ram_message(channel_id, msg_id) {
                cached_count += 1;
                let _ = database::delete_cached_message(self.pool, &msg_id.to_string()).await;
                if msg.author.bot {
                    bot_count += 1;
                } else {
                    let unix_ts = msg.timestamp.unix_timestamp();
                    user_messages.push(PurgedMessageSummary {
                        author_name: msg.author.name.clone(),
                        content: msg.content.clone(),
                        created_at: unix_ts,
                    });
                }
            } else if let Ok(Some(db_msg)) =
                load_cached_message(self.pool, &msg_id.to_string()).await
            {
                cached_count += 1;
                let _ = database::delete_cached_message(self.pool, &msg_id.to_string()).await;
                if db_msg.is_bot {
                    bot_count += 1;
                } else {
                    user_messages.push(PurgedMessageSummary {
                        author_name: db_msg.author_name,
                        content: db_msg.content,
                        created_at: db_msg.created_at,
                    });
                }
            }
        }

        let total_count = deleted_message_ids.len();

        let message_batches = build_bulk_delete_embeds(
            lang,
            channel_id,
            total_count,
            cached_count,
            bot_count,
            user_messages,
            &self.options.timestamp_format,
            self.options.preview_chars,
            self.options.chunk_chars,
            self.options.warning_color,
        );

        for builder in message_batches {
            if let Err(e) = self.outbox.send_message(log_channel_id, builder).await {
                tracing::error!("Failed to send bulk delete log: {}", e);
                break;
            }
        }
    }

    /// Archive attachments fetched by `/purge` while their signed CDN URLs are still valid.
    pub async fn archive_purge_attachments(
        &self,
        guild_id: GuildId,
        messages: &[serenity::Message],
    ) {
        if messages.is_empty() {
            return;
        }

        let Some(log_channel_id) = self.resolve_log_channel(guild_id).await else {
            return;
        };

        let mut archived_bytes = 0;
        for message in messages.iter().filter(|message| !message.author.bot) {
            for attachment in &message.attachments {
                let attachment_bytes = u64::from(attachment.size);
                if attachment_bytes > self.options.attachment_max_bytes {
                    tracing::warn!(
                        message_id = %message.id,
                        filename = %attachment.filename,
                        "Skipped oversized purged attachment"
                    );
                    continue;
                }
                if !fits_byte_budget(
                    archived_bytes,
                    attachment_bytes,
                    self.options.purge_attachment_max_total_bytes,
                ) {
                    tracing::warn!(
                        archived_bytes,
                        limit = self.options.purge_attachment_max_total_bytes,
                        "Stopped archiving purge attachments at byte limit"
                    );
                    return;
                }
                archived_bytes += attachment_bytes;

                match self
                    .fetcher
                    .fetch_attachment(attachment, self.options.attachment_max_bytes)
                    .await
                {
                    Ok(file) => {
                        if let Err(error) = self.outbox.send_attachment(log_channel_id, file).await
                        {
                            tracing::warn!(
                                message_id = %message.id,
                                filename = %attachment.filename,
                                %error,
                                "Failed to send purge attachment"
                            );
                        }
                    }
                    Err(error) => {
                        tracing::warn!(
                            message_id = %message.id,
                            filename = %attachment.filename,
                            %error,
                            "Failed to fetch purged attachment"
                        );
                    }
                }
            }
        }
    }

    async fn send_degraded_edit_fallback(
        &self,
        lang: Language,
        guild_id: GuildId,
        channel_id: ChannelId,
    ) {
        if current_health(self.pool, guild_id).await.ok() == Some(MessageLogHealth::Degraded)
            && let Some(log_channel) = self.resolve_log_channel(guild_id).await
        {
            let embed = build_metadata_embed(
                lang,
                channel_id,
                TranslationKey::MessageEditedTitle,
                self.options.warning_color,
            );
            let builder = serenity::CreateMessage::new()
                .embed(embed)
                .allowed_mentions(serenity::CreateAllowedMentions::new());
            let _ = self.outbox.send_message(log_channel, builder).await;
        }
    }
}

pub async fn save_cached_message(pool: &SqlitePool, record: &CachedMessageRecord) -> Result<()> {
    sqlx::query(
        "INSERT INTO cached_message (
            message_id, channel_id, guild_id, author_id, author_name, author_avatar_url, is_bot, content, created_at, attachments_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(message_id) DO UPDATE SET
            content = excluded.content,
            attachments_json = excluded.attachments_json",
    )
    .bind(&record.message_id)
    .bind(&record.channel_id)
    .bind(&record.guild_id)
    .bind(&record.author_id)
    .bind(&record.author_name)
    .bind(&record.author_avatar_url)
    .bind(if record.is_bot { 1 } else { 0 })
    .bind(&record.content)
    .bind(record.created_at)
    .bind(&record.attachments_json)
    .execute(pool)
    .await
    .context("Failed to save cached message")?;
    Ok(())
}

pub async fn load_cached_message(
    pool: &SqlitePool,
    message_id: &str,
) -> Result<Option<CachedMessageRecord>> {
    let row = sqlx::query_as::<_, (String, String, String, String, String, String, i64, String, i64, String)>(
        "SELECT message_id, channel_id, guild_id, author_id, author_name, author_avatar_url, is_bot, content, created_at, attachments_json FROM cached_message WHERE message_id = ?",
    )
    .bind(message_id)
    .fetch_optional(pool)
    .await
    .context("Failed to load cached message")?;

    Ok(row.map(
        |(
            message_id,
            channel_id,
            guild_id,
            author_id,
            author_name,
            author_avatar_url,
            is_bot,
            content,
            created_at,
            attachments_json,
        )| CachedMessageRecord {
            message_id,
            channel_id,
            guild_id,
            author_id,
            author_name,
            author_avatar_url,
            is_bot: is_bot != 0,
            content,
            created_at,
            attachments_json,
        },
    ))
}

