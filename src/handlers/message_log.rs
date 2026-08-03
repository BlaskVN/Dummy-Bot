use crate::Data;
use crate::config::discord_limits;
use crate::database;
use crate::i18n::{TranslationKey, t, tf};
use chrono::DateTime;
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, Context, MessageId, MessageUpdateEvent};

/// Handle message deletion events.
///
/// Looks up the cached message, checks if logging is enabled for this guild,
/// and sends an embed with the deleted message's content and attachments to the log channel.
pub async fn handle_message_delete(
    ctx: &Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
    guild_id: Option<serenity::GuildId>,
    data: &Data,
) {
    // Only process guild messages (not DMs)
    let guild_id = match guild_id {
        Some(id) => id,
        None => return,
    };

    // Get language for this guild
    let lang = data.language(guild_id).await;

    // Try to fetch the message from cache
    let message = match ctx.cache.message(channel_id, deleted_message_id) {
        Some(msg) => msg.clone(),
        None => {
            tracing::debug!(
                "Message {} not in cache, cannot log deletion",
                deleted_message_id
            );
            return;
        }
    };

    // Skip bot messages to avoid spam
    if message.author.bot {
        return;
    }

    let log_channel_id = match database::message_log_channel(&data.db_pool, guild_id).await {
        Ok(Some(channel)) => channel,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("Failed to query message_log_config: {}", e);
            return;
        }
    };

    // Build the embed with fields (matching JavaScript format)
    let content_preview = if message.content.is_empty() {
        t(lang, TranslationKey::MessageMediaOnly).to_string()
    } else {
        let preview: String = message
            .content
            .chars()
            .take(data.config.message_preview_chars)
            .collect();
        preview
    };

    // Get author avatar URL
    let avatar_url = message.author.face();

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageDeleted))
        .thumbnail(avatar_url)
        .color(data.config.colors.error)
        .field(
            t(lang, TranslationKey::MessageAuthorLabel),
            format!("<@{}>", message.author.id),
            true,
        )
        .field(
            t(lang, TranslationKey::MessageId),
            message.author.id.to_string(),
            true,
        )
        .field(
            t(lang, TranslationKey::MessageChannelLabel),
            format!("<#{}>", channel_id),
            false,
        )
        .field(
            t(lang, TranslationKey::MessageContent),
            content_preview,
            false,
        )
        .field(
            t(lang, TranslationKey::MessageDeletedAt),
            format!("<t:{}:f>", message.timestamp.unix_timestamp()),
            false,
        );

    let builder = serenity::CreateMessage::new().embed(embed);

    if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
        tracing::error!("Failed to send deletion log: {}", e);
    }

    for attachment in &message.attachments {
        if let Err(error) = send_logged_attachment(ctx, log_channel_id, data, attachment).await {
            tracing::warn!(
                filename = %attachment.filename,
                %error,
                "Skipped unsafe or oversized attachment"
            );
        }
    }
}

/// Archive attachments fetched by `/purge` while their signed CDN URLs are still valid.
pub async fn archive_purge_attachments(
    ctx: &Context,
    guild_id: serenity::GuildId,
    messages: &[serenity::Message],
    data: &Data,
) {
    if !messages
        .iter()
        .any(|message| !message.author.bot && !message.attachments.is_empty())
    {
        return;
    }

    let log_channel_id = match database::message_log_channel(&data.db_pool, guild_id).await {
        Ok(Some(channel)) => channel,
        Ok(None) => return,
        Err(error) => {
            tracing::error!(%error, "Failed to load message log channel for purge");
            return;
        }
    };

    let mut archived_bytes = 0;
    for message in messages.iter().filter(|message| !message.author.bot) {
        for attachment in &message.attachments {
            let attachment_bytes = u64::from(attachment.size);
            if attachment_bytes > data.config.attachment_max_bytes {
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
                data.config.purge_attachment_max_total_bytes,
            ) {
                tracing::warn!(
                    archived_bytes,
                    limit = data.config.purge_attachment_max_total_bytes,
                    "Stopped archiving purge attachments at byte limit"
                );
                return;
            }
            archived_bytes += attachment_bytes;

            if let Err(error) = send_logged_attachment(ctx, log_channel_id, data, attachment).await
            {
                tracing::warn!(
                    message_id = %message.id,
                    filename = %attachment.filename,
                    %error,
                    "Failed to archive purged attachment"
                );
            }
        }
    }
}

/// Handle message update (edit) events.
///
/// Compares the old (cached) content with the new content and logs the diff.
pub async fn handle_message_update(
    ctx: &Context,
    old_message: Option<&serenity::Message>,
    event: &MessageUpdateEvent,
    data: &Data,
) {
    // Only process guild messages
    let guild_id = match event.guild_id {
        Some(id) => id,
        None => return,
    };

    // Get language for this guild
    let lang = data.language(guild_id).await;

    // Serenity snapshots this before applying the update to its cache.
    let old_message = match old_message {
        Some(message) => message,
        None => return, // Not in cache, can't compare
    };

    // Skip bot messages
    if old_message.author.bot {
        return;
    }

    // Only log if content actually changed
    let new_content = match &event.content {
        Some(content) => content,
        None => return, // No content change
    };

    if old_message.content == *new_content {
        return; // Content didn't change
    }

    let log_channel_id = match database::message_log_channel(&data.db_pool, guild_id).await {
        Ok(Some(channel)) => channel,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("Failed to query message_log_config: {}", e);
            return;
        }
    };

    // Build embed showing before/after
    // Preview sizes are validated against Discord's embed limits at startup.
    let old_preview = code_block(&old_message.content, data.config.message_preview_chars);
    let new_preview = code_block(new_content, data.config.message_preview_chars);

    let author_text = tf(
        lang,
        TranslationKey::MessageAuthor,
        &[&old_message.author.id],
    );
    let channel_text = tf(lang, TranslationKey::MessageChannel, &[&event.channel_id]);
    let jump_url = format!(
        "https://discord.com/channels/{}/{}/{}",
        guild_id, event.channel_id, event.id
    );
    let jump_text = tf(lang, TranslationKey::MessageJumpTo, &[&jump_url]);

    let before_label = t(lang, TranslationKey::MessageBefore);
    let after_label = t(lang, TranslationKey::MessageAfter);

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageEditedTitle))
        .description(format!("{}\n{}\n{}", author_text, channel_text, jump_text))
        .field(before_label, old_preview, false)
        .field(after_label, new_preview, false)
        .color(data.config.colors.warning)
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new(tf(
            lang,
            TranslationKey::MessageIdValue,
            &[&event.id],
        )));

    let builder = serenity::CreateMessage::new().embed(embed);

    if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
        tracing::error!("Failed to send edit log: {}", e);
    }
}

/// Handle bulk message deletion events (purge/prune).
///
/// Logs when multiple messages are deleted at once, typically from mod commands.
pub async fn handle_message_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    deleted_message_ids: &[MessageId],
    guild_id: Option<serenity::GuildId>,
    data: &Data,
) {
    // Only process guild messages
    let guild_id = match guild_id {
        Some(id) => id,
        None => return,
    };

    // Get language for this guild
    let lang = data.language(guild_id).await;

    let log_channel_id = match database::message_log_channel(&data.db_pool, guild_id).await {
        Ok(Some(channel)) => channel,
        Ok(None) => return,
        Err(e) => {
            tracing::error!("Failed to query message_log_config: {}", e);
            return;
        }
    };

    // Try to fetch cached messages and build a summary
    let mut cached_count = 0;
    let mut bot_count = 0;
    let mut user_messages: Vec<(String, String, i64)> = Vec::new();

    for &msg_id in deleted_message_ids {
        if let Some(msg) = ctx.cache.message(channel_id, msg_id) {
            cached_count += 1;
            if msg.author.bot {
                bot_count += 1;
            } else {
                // Store user message info with unix timestamp for sorting and display
                let unix_ts = msg.timestamp.unix_timestamp();
                user_messages.push((msg.author.name.clone(), msg.content.clone(), unix_ts));
            }
        }
    }

    // Sort messages chronologically (oldest first)
    user_messages.sort_by_key(|(_, _, ts)| *ts);

    let total_count = deleted_message_ids.len();
    let user_count = cached_count - bot_count;

    // Build all formatted lines for user messages with timestamps
    let media_only = t(lang, TranslationKey::MessageMediaOnly);
    let mut all_lines: Vec<String> = Vec::new();

    for (author, content, unix_ts) in &user_messages {
        let ts_str = DateTime::from_timestamp(*unix_ts, 0)
            .map(|dt| dt.format(&data.config.message_timestamp_format).to_string())
            .unwrap_or_else(|| t(lang, TranslationKey::MessageUnknownTimestamp).to_string());

        let content_preview: String = content
            .chars()
            .take(data.config.message_preview_chars)
            .collect();
        let preview = if content_preview.is_empty() {
            media_only.to_string()
        } else {
            content_preview
        };
        all_lines.push(
            escape_code_fences(&format!("[{}] {}: {}", ts_str, author, preview))
                .chars()
                .take(data.config.message_log_chunk_chars)
                .collect(),
        );
    }

    // Split lines into chunks that fit within field value limit
    // Field value limit: 1024 chars, ``` ``` overhead: 6 chars → 1018 usable
    let mut chunks: Vec<String> = Vec::new();
    let mut current_chunk = String::new();

    for line in &all_lines {
        let needed = if current_chunk.is_empty() {
            line.chars().count()
        } else {
            line.chars().count() + 1 // +1 for \n separator
        };

        if !current_chunk.is_empty()
            && current_chunk.chars().count() + needed > data.config.message_log_chunk_chars
        {
            chunks.push(current_chunk);
            current_chunk = String::new();
        }

        if !current_chunk.is_empty() {
            current_chunk.push('\n');
        }
        current_chunk.push_str(line);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    // Build summary texts
    let channel_text = tf(lang, TranslationKey::MessageChannel, &[&channel_id]);
    let total_text = tf(lang, TranslationKey::MessageTotalDeleted, &[&total_count]);
    let cached_text = tf(
        lang,
        TranslationKey::MessageCached,
        &[&cached_count, &user_count, &bot_count],
    );

    let description = format!("{}\n{}\n{}", channel_text, total_text, cached_text);

    let deleted_messages_label = t(lang, TranslationKey::MessageDeletedMessages);
    let footer_text = tf(lang, TranslationKey::MessagePurged, &[&total_count]);
    let total_chunks = chunks.len();

    // Build embeds: first embed has full summary, subsequent embeds are continuation pages
    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();

    if chunks.is_empty() {
        // No cached messages to display
        let embed = serenity::CreateEmbed::new()
            .title(t(lang, TranslationKey::MessageBulkDeleteTitle))
            .description(description)
            .field(
                deleted_messages_label,
                t(lang, TranslationKey::MessageNoCached),
                false,
            )
            .color(data.config.colors.warning)
            .timestamp(serenity::Timestamp::now())
            .footer(serenity::CreateEmbedFooter::new(footer_text));
        embeds.push(embed);
    } else {
        for (idx, chunk) in chunks.iter().enumerate() {
            let field_value = format!("```{}```", chunk);

            if idx == 0 {
                // Main embed with summary info
                let field_name = if total_chunks > 1 {
                    format!("{} [{}/{}]", deleted_messages_label, idx + 1, total_chunks)
                } else {
                    deleted_messages_label.to_string()
                };

                let embed = serenity::CreateEmbed::new()
                    .title(t(lang, TranslationKey::MessageBulkDeleteTitle))
                    .description(&description)
                    .field(field_name, field_value, false)
                    .color(data.config.colors.warning)
                    .timestamp(serenity::Timestamp::now())
                    .footer(serenity::CreateEmbedFooter::new(&footer_text));
                embeds.push(embed);
            } else {
                // Continuation embed — lightweight, just the message chunk
                let field_name =
                    format!("{} [{}/{}]", deleted_messages_label, idx + 1, total_chunks);

                let embed = serenity::CreateEmbed::new()
                    .field(field_name, field_value, false)
                    .color(data.config.colors.warning);
                embeds.push(embed);
            }
        }
    }

    let mut remaining = embeds;

    while !remaining.is_empty() {
        let batch_size = remaining.len().min(discord_limits::EMBEDS_PER_MESSAGE);
        let batch: Vec<serenity::CreateEmbed> = remaining.drain(..batch_size).collect();

        let mut builder = serenity::CreateMessage::new();
        for embed in batch {
            builder = builder.embed(embed);
        }

        if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
            tracing::error!("Failed to send bulk delete log: {}", e);
            break;
        }
    }
}

fn escape_code_fences(value: &str) -> String {
    value.replace("```", "`\u{200b}``")
}

async fn download_attachment(
    data: &Data,
    attachment: &serenity::Attachment,
) -> anyhow::Result<serenity::CreateAttachment> {
    anyhow::ensure!(
        u64::from(attachment.size) <= data.config.attachment_max_bytes,
        "attachment exceeds {} bytes",
        data.config.attachment_max_bytes
    );
    let url = reqwest::Url::parse(&attachment.url)?;
    anyhow::ensure!(is_discord_cdn(&url), "attachment URL is not Discord CDN");

    let mut response = data
        .attachment_client
        .get(url)
        .send()
        .await?
        .error_for_status()?;
    anyhow::ensure!(
        response.content_length().unwrap_or(0) <= data.config.attachment_max_bytes,
        "attachment response exceeds byte limit"
    );

    let mut bytes = Vec::with_capacity(attachment.size as usize);
    while let Some(chunk) = response.chunk().await? {
        anyhow::ensure!(
            (bytes.len() as u64).saturating_add(chunk.len() as u64)
                <= data.config.attachment_max_bytes,
            "attachment body exceeds byte limit"
        );
        bytes.extend_from_slice(&chunk);
    }

    Ok(serenity::CreateAttachment::bytes(
        bytes,
        attachment.filename.clone(),
    ))
}

fn fits_byte_budget(used: u64, next: u64, limit: u64) -> bool {
    used.checked_add(next).is_some_and(|total| total <= limit)
}

async fn send_logged_attachment(
    ctx: &Context,
    log_channel_id: ChannelId,
    data: &Data,
    attachment: &serenity::Attachment,
) -> anyhow::Result<()> {
    // Keep the permit through upload so buffered files stay within the concurrency ceiling.
    let _permit = data.attachment_downloads.acquire().await?;
    let file = download_attachment(data, attachment).await?;
    log_channel_id
        .send_message(&ctx.http, serenity::CreateMessage::new().add_file(file))
        .await?;
    Ok(())
}

fn is_discord_cdn(url: &reqwest::Url) -> bool {
    url.scheme() == "https"
        && matches!(
            url.host_str(),
            Some("cdn.discordapp.com" | "media.discordapp.net")
        )
}

fn code_block(value: &str, max_chars: usize) -> String {
    let escaped = escape_code_fences(value);
    format!(
        "```{}```",
        escaped.chars().take(max_chars).collect::<String>()
    )
}

#[cfg(test)]
mod tests {
    use super::{code_block, fits_byte_budget, is_discord_cdn};

    #[test]
    fn user_content_cannot_close_code_block() {
        let block = code_block("before```fake log", 100);
        assert_eq!(block.matches("```").count(), 2);
    }

    #[test]
    fn attachments_only_use_discord_cdn() {
        assert!(is_discord_cdn(
            &"https://cdn.discordapp.com/attachments/1/2/file.png"
                .parse()
                .unwrap()
        ));
        assert!(!is_discord_cdn(&"https://127.0.0.1/file".parse().unwrap()));
    }

    #[test]
    fn purge_attachment_budget_includes_boundary_and_rejects_overflow() {
        assert!(fits_byte_budget(6, 4, 10));
        assert!(!fits_byte_budget(7, 4, 10));
        assert!(!fits_byte_budget(u64::MAX, 1, u64::MAX));
    }
}
