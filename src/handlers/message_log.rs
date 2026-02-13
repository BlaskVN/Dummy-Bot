use crate::i18n::{get_guild_language, t, tf, TranslationKey};
use crate::Data;
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
    let lang = get_guild_language(&data.db_pool, guild_id).await;

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

    // Check if logging is enabled for this guild
    let config = match sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
        .bind(guild_id.to_string())
        .fetch_optional(&data.db_pool)
        .await
    {
        Ok(Some((channel, enabled))) if enabled == 1 => channel,
        Ok(_) => return, // Logging disabled or not configured
        Err(e) => {
            tracing::error!("Failed to query message_log_config: {}", e);
            return;
        }
    };

    let log_channel_id = match config.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => return,
    };

    // Build the embed
    // Description limit is 4096 chars, reserve space for labels and formatting
    // author_text (~40) + channel_text (~40) + content_label (~15) + newlines (~3) = ~98 chars
    // Available for content: 4096 - 98 - 6 (``` ```) = ~3992 chars
    // But we'll be more conservative to ensure total embed stays under 6000 chars
    const MAX_CONTENT_CHARS: usize = 1900;

    let content_preview = if message.content.is_empty() {
        t(lang, TranslationKey::MessageMediaOnly).to_string()
    } else {
        let preview: String = message.content.chars().take(MAX_CONTENT_CHARS).collect();
        format!("```{}```", preview)
    };

    let author_text = tf(lang, TranslationKey::MessageAuthor, &[&message.author.id]);
    let channel_text = tf(lang, TranslationKey::MessageChannel, &[&channel_id]);
    let content_label = t(lang, TranslationKey::MessageContent);

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageDeleted))
        .description(format!(
            "{}\n{}\n{}\n{}",
            author_text, channel_text, content_label, content_preview
        ))
        .color(0xe74c3c) // Red
        .timestamp(message.timestamp)
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Message ID: {}",
            message.id
        )));

    // Download and re-attach media files
    let mut attachments = Vec::new();
    for attachment in &message.attachments {
        match download_attachment(&data.http_client, &attachment.url, &attachment.filename).await {
            Ok(file) => attachments.push(file),
            Err(e) => {
                tracing::warn!(
                    "Failed to download attachment {}: {}",
                    attachment.filename,
                    e
                );
            }
        }
    }

    // Send log message (embed first)
    let builder = serenity::CreateMessage::new().embed(embed);

    if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
        tracing::error!("Failed to send deletion log: {}", e);
        return;
    }

    // Send media files separately after the message info (matching old bot behavior)
    if !attachments.is_empty() {
        let media_builder = serenity::CreateMessage::new().files(attachments);
        if let Err(e) = log_channel_id.send_message(&ctx.http, media_builder).await {
            tracing::error!("Failed to send media attachments: {}", e);
        }
    }
}

/// Handle message update (edit) events.
///
/// Compares the old (cached) content with the new content and logs the diff.
pub async fn handle_message_update(ctx: &Context, event: &MessageUpdateEvent, data: &Data) {
    // Only process guild messages
    let guild_id = match event.guild_id {
        Some(id) => id,
        None => return,
    };

    // Get language for this guild
    let lang = get_guild_language(&data.db_pool, guild_id).await;

    // Get old message from cache
    let old_message = match ctx.cache.message(event.channel_id, event.id) {
        Some(msg) => msg.clone(),
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

    // Check if logging is enabled
    let config = match sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
        .bind(guild_id.to_string())
        .fetch_optional(&data.db_pool)
        .await
    {
        Ok(Some((channel, enabled))) if enabled == 1 => channel,
        Ok(_) => return,
        Err(e) => {
            tracing::error!("Failed to query message_log_config: {}", e);
            return;
        }
    };

    let log_channel_id = match config.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => return,
    };

    // Build embed showing before/after
    // Field value limit is 1024 chars, with ``` ``` overhead (6 chars) = 1018 chars available
    // We'll use 900 chars to leave some margin and ensure total embed stays under 6000
    const MAX_FIELD_CONTENT_CHARS: usize = 900;

    let old_preview: String = old_message.content.chars().take(MAX_FIELD_CONTENT_CHARS).collect();
    let new_preview: String = new_content.chars().take(MAX_FIELD_CONTENT_CHARS).collect();

    let author_text = tf(lang, TranslationKey::MessageAuthor, &[&old_message.author.id]);
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
        .description(format!(
            "{}\n{}\n{}",
            author_text,
            channel_text,
            jump_text
        ))
        .field(before_label, format!("```{}```", old_preview), false)
        .field(after_label, format!("```{}```", new_preview), false)
        .color(0xf39c12) // Orange
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new(format!(
            "Message ID: {}",
            event.id
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
    let lang = get_guild_language(&data.db_pool, guild_id).await;

    // Check if logging is enabled for this guild
    let config = match sqlx::query_as::<_, (String, i64)>(
        "SELECT log_channel_id, enabled FROM message_log_config WHERE guild_id = ?",
    )
        .bind(guild_id.to_string())
        .fetch_optional(&data.db_pool)
        .await
    {
        Ok(Some((channel, enabled))) if enabled == 1 => channel,
        Ok(_) => return, // Logging disabled or not configured
        Err(e) => {
            tracing::error!("Failed to query message_log_config: {}", e);
            return;
        }
    };

    let log_channel_id = match config.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => return,
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
            .map(|dt| dt.format("%d/%m %H:%M").to_string())
            .unwrap_or_else(|| "??/?? ??:??".to_string());

        let content_preview: String = content.chars().take(45).collect();
        let preview = if content_preview.is_empty() {
            media_only.to_string()
        } else {
            content_preview
        };
        all_lines.push(format!("[{}] {}: {}", ts_str, author, preview));
    }

    // Split lines into chunks that fit within field value limit
    // Field value limit: 1024 chars, ``` ``` overhead: 6 chars → 1018 usable
    const MAX_CHUNK_CHARS: usize = 1000;

    let mut chunks: Vec<String> = Vec::new();
    let mut current_chunk = String::new();

    for line in &all_lines {
        let needed = if current_chunk.is_empty() {
            line.len()
        } else {
            line.len() + 1 // +1 for \n separator
        };

        if !current_chunk.is_empty() && current_chunk.len() + needed > MAX_CHUNK_CHARS {
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
    let cached_text = tf(lang, TranslationKey::MessageCached, &[&cached_count, &user_count, &bot_count]);

    let description = format!(
        "{}\n{}\n{}",
        channel_text, total_text, cached_text
    );

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
            .field(deleted_messages_label, "*No cached messages to display*", false)
            .color(0xe67e22)
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
                    .color(0xe67e22)
                    .timestamp(serenity::Timestamp::now())
                    .footer(serenity::CreateEmbedFooter::new(&footer_text));
                embeds.push(embed);
            } else {
                // Continuation embed — lightweight, just the message chunk
                let field_name = format!(
                    "{} [{}/{}]",
                    deleted_messages_label,
                    idx + 1,
                    total_chunks
                );

                let embed = serenity::CreateEmbed::new()
                    .field(field_name, field_value, false)
                    .color(0xe67e22);
                embeds.push(embed);
            }
        }
    }

    // Send embeds in batches of 10 (Discord limit per message)
    const MAX_EMBEDS_PER_MESSAGE: usize = 10;
    let mut remaining = embeds;

    while !remaining.is_empty() {
        let batch_size = remaining.len().min(MAX_EMBEDS_PER_MESSAGE);
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

/// Download an attachment from Discord CDN and return it as a CreateAttachment.
async fn download_attachment(
    client: &reqwest::Client,
    url: &str,
    filename: &str,
) -> Result<serenity::CreateAttachment, Box<dyn std::error::Error + Send + Sync>> {
    let response = client.get(url).send().await?;
    let bytes = response.bytes().await?;

    Ok(serenity::CreateAttachment::bytes(bytes.to_vec(), filename))
}
