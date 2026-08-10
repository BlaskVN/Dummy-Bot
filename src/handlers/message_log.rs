use crate::Data;
use crate::config::discord_limits;
use crate::database;
use crate::i18n::{TranslationKey, t, tf};
use crate::message_log_health::{MessageLogHealth, current_health, mark_warning_sent, reconcile};
use crate::ui::{self, Tone};
use chrono::DateTime;
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, Context, MessageId, MessageUpdateEvent};

pub async fn reconcile_all_health(ctx: &Context, data: &Data) {
    let rows = match sqlx::query_as::<_, (String, String)>(
        "SELECT guild_id, log_channel_id FROM message_log_config WHERE enabled = 1",
    )
    .fetch_all(&data.db_pool)
    .await
    {
        Ok(rows) => rows,
        Err(error) => {
            tracing::error!(%error, "Failed to load Message Log health");
            return;
        }
    };
    for (guild, channel) in rows {
        let (Ok(guild_id), Ok(channel_id)) = (guild.parse(), channel.parse()) else {
            tracing::error!(guild, channel, "Invalid stored Message Log identifiers");
            continue;
        };
        let guild_id = serenity::GuildId::new(guild_id);
        match reconcile(&data.db_pool, guild_id, data.config.message_content_enabled).await {
            Ok((_, true)) => {
                let language = data.language(guild_id).await;
                let result = serenity::ChannelId::new(channel_id)
                    .send_message(
                        &ctx.http,
                        serenity::CreateMessage::new()
                            .embed(ui::panel(
                                data,
                                Tone::Warning,
                                t(language, TranslationKey::MessageLogDegradedWarning),
                            ))
                            .allowed_mentions(serenity::CreateAllowedMentions::new()),
                    )
                    .await;
                if result.is_ok()
                    && let Err(error) = mark_warning_sent(&data.db_pool, guild_id).await
                {
                    tracing::error!(%guild_id, %error, "Failed to persist Message Log warning state");
                }
            }
            Ok(_) => {}
            Err(error) => {
                tracing::error!(%guild_id, %error, "Failed to reconcile Message Log health")
            }
        }
    }
}

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
    let message = ctx
        .cache
        .message(channel_id, deleted_message_id)
        .map(|message| message.clone());
    let Some(message) = message else {
        send_metadata_log(
            ctx,
            data,
            guild_id,
            channel_id,
            deleted_message_id,
            TranslationKey::MessageDeleted,
        )
        .await;
        return;
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

    let deleted_by = deletion_actor(ctx, guild_id, channel_id, &message)
        .await
        .unwrap_or_else(|| message.author.clone());

    let content_preview = if message.content.is_empty() {
        t(lang, TranslationKey::MessageMediaOnly).to_string()
    } else {
        markdown_quote(&message.content, data.config.message_preview_chars)
    };

    let sent_at = format!("<t:{}:f>", message.timestamp.unix_timestamp());
    let deleted_at = serenity::Timestamp::now();
    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageDeleted))
        .author(serenity::CreateEmbedAuthor::from(&deleted_by))
        .thumbnail(message.author.face())
        .color(data.config.colors.error)
        .field(
            t(lang, TranslationKey::MessageAuthorLabel),
            format!("<@{}>", message.author.id),
            false,
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
        .field(t(lang, TranslationKey::MessageSentAt), sent_at, false)
        .timestamp(deleted_at)
        .footer(serenity::CreateEmbedFooter::new(t(
            lang,
            TranslationKey::MessageDeletedAt,
        )));
    if let Some(reply) = reply_field(lang, guild_id, &message) {
        embed = embed.field(t(lang, TranslationKey::MessageReplyTo), reply, false);
    }

    let builder = serenity::CreateMessage::new()
        .embed(embed)
        .allowed_mentions(serenity::CreateAllowedMentions::new());

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
        None => {
            send_metadata_log(
                ctx,
                data,
                guild_id,
                event.channel_id,
                event.id,
                TranslationKey::MessageEditedTitle,
            )
            .await;
            return;
        }
    };

    // Skip bot messages
    if old_message.author.bot {
        return;
    }

    // Only log if content actually changed
    let new_content = match &event.content {
        Some(content) => content,
        None => {
            if current_health(&data.db_pool, guild_id).await.ok()
                == Some(MessageLogHealth::Degraded)
            {
                send_metadata_log(
                    ctx,
                    data,
                    guild_id,
                    event.channel_id,
                    event.id,
                    TranslationKey::MessageEditedTitle,
                )
                .await;
            }
            return;
        }
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
    let old_preview = markdown_quote(&old_message.content, data.config.message_preview_chars);
    let new_preview = markdown_quote(new_content, data.config.message_preview_chars);

    let before_label = t(lang, TranslationKey::MessageBefore);
    let after_label = t(lang, TranslationKey::MessageAfter);

    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageEditedTitle))
        .author(serenity::CreateEmbedAuthor::from(&old_message.author))
        .thumbnail(old_message.author.face())
        .field(
            t(lang, TranslationKey::MessageAuthorLabel),
            format!("<@{}>", old_message.author.id),
            false,
        )
        .field(
            t(lang, TranslationKey::MessageChannelLabel),
            format!("<#{}>", event.channel_id),
            false,
        )
        .field(before_label, old_preview, false)
        .field(after_label, new_preview, false)
        .color(data.config.colors.warning)
        .timestamp(serenity::Timestamp::now())
        .footer(serenity::CreateEmbedFooter::new(format!(
            "{} <t:{}:f>",
            t(lang, TranslationKey::MessageSentAt),
            old_message.timestamp.unix_timestamp()
        )));
    if let Some(reply) = reply_field(lang, guild_id, old_message) {
        embed = embed.field(t(lang, TranslationKey::MessageReplyTo), reply, false);
    }

    let builder = serenity::CreateMessage::new()
        .embed(embed)
        .allowed_mentions(serenity::CreateAllowedMentions::new());

    if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
        tracing::error!("Failed to send edit log: {}", e);
    }
}

async fn deletion_actor(
    ctx: &Context,
    guild_id: serenity::GuildId,
    channel_id: ChannelId,
    message: &serenity::Message,
) -> Option<serenity::User> {
    let logs = guild_id
        .audit_logs(
            &ctx.http,
            Some(serenity::audit_log::Action::Message(
                serenity::audit_log::MessageAction::Delete,
            )),
            None,
            None,
            Some(5),
        )
        .await
        .ok()?;
    let entry = logs.entries.iter().find(|entry| {
        entry.options.as_ref().is_some_and(|options| {
            exact_delete_match(
                entry.target_id,
                serenity::GenericId::new(message.author.id.get()),
                options.channel_id,
                channel_id,
                options.message_id,
                message.id,
            )
        })
    })?;
    logs.users.get(&entry.user_id).cloned()
}

fn exact_delete_match(
    audit_target_id: Option<serenity::GenericId>,
    author_id: serenity::GenericId,
    audit_channel_id: Option<ChannelId>,
    channel_id: ChannelId,
    audit_message_id: Option<MessageId>,
    message_id: MessageId,
) -> bool {
    audit_target_id == Some(author_id)
        && audit_channel_id == Some(channel_id)
        && audit_message_id == Some(message_id)
}

fn reply_field(
    lang: crate::i18n::Language,
    guild_id: serenity::GuildId,
    message: &serenity::Message,
) -> Option<String> {
    let reference = message.message_reference.as_ref()?;
    let message_id = reference.message_id?;
    let channel_id = reference.channel_id;
    let jump_url = message_url(guild_id, channel_id, message_id);
    let preview = message
        .referenced_message
        .as_deref()
        .map(|reply| {
            format!(
                "<@{}>\n{}",
                reply.author.id,
                markdown_quote(&reply.content, 700)
            )
        })
        .unwrap_or_else(|| t(lang, TranslationKey::MessageNoCached).to_string());
    Some(format!(
        "{preview}\n{}",
        tf(lang, TranslationKey::MessageJumpTo, &[&jump_url])
    ))
}

fn message_url(
    guild_id: serenity::GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
) -> String {
    format!("https://discord.com/channels/{guild_id}/{channel_id}/{message_id}")
}

async fn send_metadata_log(
    ctx: &Context,
    data: &Data,
    guild_id: serenity::GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
    title: TranslationKey,
) {
    let Ok(Some(log_channel)) = database::message_log_channel(&data.db_pool, guild_id).await else {
        return;
    };
    let language = data.language(guild_id).await;
    let embed = serenity::CreateEmbed::new()
        .title(t(language, title))
        .description(t(language, TranslationKey::MessageNoCached))
        .field(
            t(language, TranslationKey::MessageChannelLabel),
            format!("<#{channel_id}>"),
            false,
        )
        .color(data.config.colors.warning);
    if let Err(error) = log_channel
        .send_message(
            &ctx.http,
            serenity::CreateMessage::new()
                .embed(embed)
                .allowed_mentions(serenity::CreateAllowedMentions::new()),
        )
        .await
    {
        tracing::warn!(%guild_id, %message_id, %error, "Failed to send metadata-only Message Log entry");
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

        let preview = if content.is_empty() {
            media_only
        } else {
            content
        };
        all_lines.push(markdown_message(
            &ts_str,
            author,
            preview,
            data.config
                .message_preview_chars
                .min(data.config.message_log_chunk_chars),
        ));
    }

    // Split lines into chunks that fit within field value limit
    // Field value limit: 1024 chars, ``` ``` overhead: 6 chars → 1018 usable
    let mut chunks: Vec<String> = Vec::new();
    let mut current_chunk = String::new();

    for line in &all_lines {
        let needed = if current_chunk.is_empty() {
            line.chars().count()
        } else {
            line.chars().count() + 2 // blank line between message blocks
        };

        if !current_chunk.is_empty()
            && current_chunk.chars().count() + needed > data.config.message_log_chunk_chars
        {
            chunks.push(current_chunk);
            current_chunk = String::new();
        }

        if !current_chunk.is_empty() {
            current_chunk.push_str("\n\n");
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
    let mut embeds: Vec<(serenity::CreateEmbed, usize)> = Vec::new();

    if chunks.is_empty() {
        // No cached messages to display
        let title = t(lang, TranslationKey::MessageBulkDeleteTitle);
        let no_cached = t(lang, TranslationKey::MessageNoCached);
        let embed = serenity::CreateEmbed::new()
            .title(title)
            .description(&description)
            .field(deleted_messages_label, no_cached, false)
            .color(data.config.colors.warning)
            .timestamp(serenity::Timestamp::now())
            .footer(serenity::CreateEmbedFooter::new(&footer_text));
        let chars = [
            title,
            &description,
            deleted_messages_label,
            no_cached,
            &footer_text,
        ]
        .iter()
        .map(|value| value.chars().count())
        .sum();
        embeds.push((embed, chars));
    } else {
        for (idx, chunk) in chunks.iter().enumerate() {
            if idx == 0 {
                // Main embed with summary info
                let field_name = if total_chunks > 1 {
                    format!("{} [{}/{}]", deleted_messages_label, idx + 1, total_chunks)
                } else {
                    deleted_messages_label.to_string()
                };

                let title = t(lang, TranslationKey::MessageBulkDeleteTitle);
                let embed = serenity::CreateEmbed::new()
                    .title(title)
                    .description(&description)
                    .field(&field_name, chunk, false)
                    .color(data.config.colors.warning)
                    .timestamp(serenity::Timestamp::now())
                    .footer(serenity::CreateEmbedFooter::new(&footer_text));
                let chars = [title, &description, &field_name, chunk, &footer_text]
                    .iter()
                    .map(|value| value.chars().count())
                    .sum();
                embeds.push((embed, chars));
            } else {
                // Continuation embed — lightweight, just the message chunk
                let field_name =
                    format!("{} [{}/{}]", deleted_messages_label, idx + 1, total_chunks);

                let embed = serenity::CreateEmbed::new()
                    .field(&field_name, chunk, false)
                    .color(data.config.colors.warning);
                let chars = field_name.chars().count() + chunk.chars().count();
                embeds.push((embed, chars));
            }
        }
    }

    let mut remaining = embeds.into_iter().peekable();

    while remaining.peek().is_some() {
        let mut builder =
            serenity::CreateMessage::new().allowed_mentions(serenity::CreateAllowedMentions::new());
        let mut batch_chars = 0;
        let mut batch_count = 0;
        while let Some((_, next_chars)) = remaining.peek()
            && fits_embed_batch(batch_count, batch_chars, *next_chars)
        {
            let (embed, chars) = remaining.next().expect("peeked embed page");
            builder = builder.embed(embed);
            batch_chars += chars;
            batch_count += 1;
        }
        if batch_count == 0 {
            tracing::error!("Bulk delete embed exceeds Discord's per-message limits");
            break;
        }

        if let Err(e) = log_channel_id.send_message(&ctx.http, builder).await {
            tracing::error!("Failed to send bulk delete log: {}", e);
            break;
        }
    }
}

fn fits_embed_batch(count: usize, chars: usize, next_chars: usize) -> bool {
    count < discord_limits::EMBEDS_PER_MESSAGE
        && next_chars <= discord_limits::EMBED_TOTAL_CHARS
        && chars.saturating_add(next_chars) <= discord_limits::EMBED_TOTAL_CHARS
}

fn escape_markdown(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        if matches!(
            character,
            '\\' | '`' | '*' | '_' | '~' | '|' | '>' | '#' | '[' | ']' | '(' | ')' | '<'
        ) {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

fn markdown_quote(value: &str, max_chars: usize) -> String {
    let escaped = escape_markdown(value);
    let mut quote = String::new();
    let mut remaining = max_chars;

    for (index, line) in escaped.lines().enumerate() {
        let prefix = if index == 0 { "> " } else { "\n> " };
        let prefix_chars = prefix.chars().count();
        if remaining <= prefix_chars {
            break;
        }
        quote.push_str(prefix);
        remaining -= prefix_chars;

        let line_chars = line.chars().count();
        if line_chars <= remaining {
            quote.push_str(line);
            remaining -= line_chars;
        } else {
            quote.extend(line.chars().take(remaining.saturating_sub(1)));
            if remaining > 0 {
                quote.push('…');
            }
            break;
        }
    }
    quote
}

fn markdown_message(timestamp: &str, author: &str, content: &str, max_chars: usize) -> String {
    let header = format!(
        "**{} · {}**\n",
        escape_markdown(timestamp),
        escape_markdown(author)
    );
    let header_chars = header.chars().count();
    if header_chars >= max_chars {
        return markdown_quote(author, max_chars);
    }
    format!(
        "{}{}",
        header,
        markdown_quote(content, max_chars - header_chars)
    )
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

#[cfg(test)]
mod tests {
    use poise::serenity_prelude as serenity;

    use super::{
        exact_delete_match, fits_byte_budget, fits_embed_batch, is_discord_cdn, markdown_message,
        markdown_quote, message_url,
    };

    #[test]
    fn deletion_actor_requires_exact_message_id() {
        let author = serenity::GenericId::new(1);
        let channel = serenity::ChannelId::new(2);
        let deleted = serenity::MessageId::new(7);
        assert!(exact_delete_match(
            Some(author),
            author,
            Some(channel),
            channel,
            Some(deleted),
            deleted,
        ));
        assert!(!exact_delete_match(
            Some(author),
            author,
            Some(channel),
            channel,
            None,
            deleted,
        ));
        assert!(!exact_delete_match(
            Some(author),
            author,
            Some(channel),
            channel,
            Some(serenity::MessageId::new(8)),
            deleted
        ));
    }

    #[test]
    fn user_markdown_cannot_escape_its_message_block() {
        let entry = markdown_message(
            "12:00",
            "**admin**",
            "```fake log\n> quote\n@everyone <@1> **next**",
            500,
        );
        assert!(entry.contains("\\`\\`\\`fake log"));
        assert!(entry.contains("\n> \\> quote"));
        assert!(entry.contains("\\<@1\\>"));
        assert!(!entry.contains("```"));
        assert!(entry.chars().count() <= 500);
    }

    #[test]
    fn multiline_quotes_prefix_every_line_and_fit_the_limit() {
        let quote = markdown_quote("first\nsecond\nthird", 20);
        assert!(quote.lines().all(|line| line.starts_with("> ")));
        assert!(quote.chars().count() <= 20);
    }

    #[test]
    fn embed_batches_respect_count_and_combined_character_limits() {
        assert!(fits_embed_batch(0, 0, 6_000));
        assert!(fits_embed_batch(9, 5_000, 1_000));
        assert!(!fits_embed_batch(10, 0, 1));
        assert!(!fits_embed_batch(1, 5_001, 1_000));
        assert!(!fits_embed_batch(0, 0, 6_001));
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
    fn reply_link_points_to_the_referenced_message() {
        assert_eq!(
            message_url(
                serenity::GuildId::new(1),
                serenity::ChannelId::new(2),
                serenity::MessageId::new(3)
            ),
            "https://discord.com/channels/1/2/3"
        );
    }

    #[test]
    fn purge_attachment_budget_includes_boundary_and_rejects_overflow() {
        assert!(fits_byte_budget(6, 4, 10));
        assert!(!fits_byte_budget(7, 4, 10));
        assert!(!fits_byte_budget(u64::MAX, 1, u64::MAX));
    }
}
