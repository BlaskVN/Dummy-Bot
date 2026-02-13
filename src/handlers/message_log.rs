use crate::i18n::{get_guild_language, t, tf, TranslationKey};
use crate::Data;
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
    let content_preview = if message.content.is_empty() {
        t(lang, TranslationKey::MessageMediaOnly).to_string()
    } else {
        message.content.chars().take(1000).collect()
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
    let old_preview: String = old_message.content.chars().take(500).collect();
    let new_preview: String = new_content.chars().take(500).collect();

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
    let mut user_messages = Vec::new();

    for &msg_id in deleted_message_ids {
        if let Some(msg) = ctx.cache.message(channel_id, msg_id) {
            cached_count += 1;
            if msg.author.bot {
                bot_count += 1;
            } else {
                // Store user message info for detailed logging
                user_messages.push((msg.author.name.clone(), msg.content.clone()));
            }
        }
    }

    let total_count = deleted_message_ids.len();
    let user_count = cached_count - bot_count;

    // Build detailed message list (limited to first 10 to avoid spam)
    let media_only = t(lang, TranslationKey::MessageMediaOnly);
    let mut message_list = String::new();
    for (i, (author, content)) in user_messages.iter().take(10).enumerate() {
        let content_preview: String = content.chars().take(100).collect();
        let preview = if content_preview.is_empty() {
            media_only
        } else {
            &content_preview
        };
        message_list.push_str(&format!("{}. **{}**: {}\n", i + 1, author, preview));
    }

    if user_messages.len() > 10 {
        message_list.push_str(&format!("\n*...and {} more messages*", user_messages.len() - 10));
    }

    if message_list.is_empty() {
        message_list = "*No cached messages to display*".to_string();
    }

    // Build the embed
    let channel_text = tf(lang, TranslationKey::MessageChannel, &[&channel_id]);
    let total_text = tf(lang, TranslationKey::MessageTotalDeleted, &[&total_count]);
    let cached_text = tf(lang, TranslationKey::MessageCached, &[&cached_count, &user_count, &bot_count]);

    let description = format!(
        "{}\n{}\n{}",
        channel_text, total_text, cached_text
    );

    let deleted_messages_label = t(lang, TranslationKey::MessageDeletedMessages);
    let footer_text = tf(lang, TranslationKey::MessagePurged, &[&total_count]);

    let embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageBulkDeleteTitle))
        .description(description)
        .field(deleted_messages_label, message_list, false)
        .color(0xe67e22) // Orange
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new(footer_text));

    let builder = serenity::CreateMessage::new().embed(embed);

    if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
        tracing::error!("Failed to send bulk delete log: {}", e);
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
