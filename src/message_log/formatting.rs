use crate::config::discord_limits;
use crate::i18n::{Language, TranslationKey, t, tf};
use chrono::DateTime;
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, GuildId, MessageId};

pub fn escape_markdown(value: &str) -> String {
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

pub fn markdown_quote(value: &str, max_chars: usize) -> String {
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

pub fn markdown_message(timestamp: &str, author: &str, content: &str, max_chars: usize) -> String {
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

pub fn fits_embed_batch(count: usize, chars: usize, next_chars: usize) -> bool {
    count < discord_limits::EMBEDS_PER_MESSAGE
        && next_chars <= discord_limits::EMBED_TOTAL_CHARS
        && chars.saturating_add(next_chars) <= discord_limits::EMBED_TOTAL_CHARS
}

pub fn fits_byte_budget(used: u64, next: u64, limit: u64) -> bool {
    used.checked_add(next).is_some_and(|total| total <= limit)
}

pub fn message_url(guild_id: GuildId, channel_id: ChannelId, message_id: MessageId) -> String {
    format!("https://discord.com/channels/{guild_id}/{channel_id}/{message_id}")
}

pub fn reply_field(
    lang: Language,
    guild_id: GuildId,
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
            let content = if reply.content.is_empty() {
                t(lang, TranslationKey::MessageMediaOnly).to_string()
            } else {
                markdown_quote(&reply.content, 700)
            };
            format!("<@{}>: {content}", reply.author.id)
        })
        .unwrap_or_else(|| t(lang, TranslationKey::MessageNoCached).to_string());
    Some(format!(
        "{preview}\n{}",
        tf(lang, TranslationKey::MessageJumpTo, &[&jump_url])
    ))
}

pub fn build_deleted_message_embed(
    lang: Language,
    view: &super::models::DeletedMessageView<'_>,
    preview_chars: usize,
    error_color: serenity::Colour,
) -> serenity::CreateEmbed {
    let content_preview = if view.content.is_empty() {
        t(lang, TranslationKey::MessageMediaOnly).to_string()
    } else {
        markdown_quote(view.content, preview_chars)
    };

    let sent_at = format!("<t:{}:f>", view.sent_at_unix);
    let deleted_at = serenity::Timestamp::now();
    let deleted_at_str = format!("<t:{}:f>", deleted_at.unix_timestamp());

    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageDeleted))
        .thumbnail(view.author_face)
        .color(error_color)
        .field(
            t(lang, TranslationKey::MessageAuthorLabel),
            format!("<@{}>", view.author_id),
            true,
        )
        .field(
            t(lang, TranslationKey::MessageChannelLabel),
            format!("<#{}>", view.channel_id),
            true,
        )
        .field(
            t(lang, TranslationKey::MessageContent),
            content_preview,
            false,
        );

    if let Some(reply) = &view.reply_info {
        embed = embed.field(t(lang, TranslationKey::MessageReplyTo), reply, false);
    }

    embed
        .field(
            t(lang, TranslationKey::MessageDeletedAt),
            deleted_at_str,
            true,
        )
        .field(t(lang, TranslationKey::MessageSentAt), sent_at, true)
        .field(
            t(lang, TranslationKey::MessageJumpTo),
            message_url(view.guild_id, view.channel_id, view.message_id),
            false,
        )
        .timestamp(deleted_at)
}

#[allow(clippy::too_many_arguments)]
pub fn build_edited_message_embed(
    lang: Language,
    channel_id: ChannelId,
    author_id: &str,
    author_face: &str,
    old_content: &str,
    new_content: &str,
    sent_at_unix: i64,
    preview_chars: usize,
    warning_color: serenity::Colour,
    reply_info: Option<String>,
) -> serenity::CreateEmbed {
    let old_preview = markdown_quote(old_content, preview_chars);
    let new_preview = markdown_quote(new_content, preview_chars);

    let before_label = t(lang, TranslationKey::MessageBefore);
    let after_label = t(lang, TranslationKey::MessageAfter);

    let sent_at = format!("<t:{sent_at_unix}:f>");
    let edited_at = serenity::Timestamp::now();
    let edited_at_str = format!("<t:{}:f>", edited_at.unix_timestamp());

    let mut embed = serenity::CreateEmbed::new()
        .title(t(lang, TranslationKey::MessageEditedTitle))
        .thumbnail(author_face)
        .field(
            t(lang, TranslationKey::MessageAuthorLabel),
            format!("<@{author_id}>"),
            true,
        )
        .field(
            t(lang, TranslationKey::MessageChannelLabel),
            format!("<#{channel_id}>"),
            true,
        )
        .field(before_label, old_preview, false)
        .field(after_label, new_preview, false);

    if let Some(reply) = reply_info {
        embed = embed.field(t(lang, TranslationKey::MessageReplyTo), reply, false);
    }

    embed
        .field(
            t(lang, TranslationKey::MessageEditedAt),
            edited_at_str,
            true,
        )
        .field(t(lang, TranslationKey::MessageSentAt), sent_at, true)
        .color(warning_color)
        .timestamp(edited_at)
}

pub fn build_metadata_embed(
    lang: Language,
    channel_id: ChannelId,
    title_key: TranslationKey,
    warning_color: serenity::Colour,
) -> serenity::CreateEmbed {
    serenity::CreateEmbed::new()
        .title(t(lang, title_key))
        .description(t(lang, TranslationKey::MessageNoCached))
        .field(
            t(lang, TranslationKey::MessageChannelLabel),
            format!("<#{channel_id}>"),
            false,
        )
        .color(warning_color)
}

#[allow(clippy::too_many_arguments)]
pub fn build_bulk_delete_embeds(
    lang: Language,
    channel_id: ChannelId,
    total_count: usize,
    cached_count: usize,
    bot_count: usize,
    mut user_messages: Vec<super::models::PurgedMessageSummary>,
    timestamp_format: &str,
    preview_chars: usize,
    chunk_chars_limit: usize,
    warning_color: serenity::Colour,
) -> Vec<serenity::CreateMessage> {
    user_messages.sort_by_key(|msg| msg.created_at);
    let user_count = cached_count.saturating_sub(bot_count);

    let media_only = t(lang, TranslationKey::MessageMediaOnly);
    let mut all_lines: Vec<String> = Vec::new();

    for msg in &user_messages {
        let ts_str = DateTime::from_timestamp(msg.created_at, 0)
            .map(|dt| dt.format(timestamp_format).to_string())
            .unwrap_or_else(|| t(lang, TranslationKey::MessageUnknownTimestamp).to_string());

        let preview = if msg.content.is_empty() {
            media_only
        } else {
            &msg.content
        };
        all_lines.push(markdown_message(
            &ts_str,
            &msg.author_name,
            preview,
            preview_chars.min(chunk_chars_limit),
        ));
    }

    let mut chunks: Vec<String> = Vec::new();
    let mut current_chunk = String::new();

    for line in &all_lines {
        let needed = if current_chunk.is_empty() {
            line.chars().count()
        } else {
            line.chars().count() + 2
        };

        if !current_chunk.is_empty() && current_chunk.chars().count() + needed > chunk_chars_limit {
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

    let mut embeds: Vec<(serenity::CreateEmbed, usize)> = Vec::new();

    if chunks.is_empty() {
        let title = t(lang, TranslationKey::MessageBulkDeleteTitle);
        let no_cached = t(lang, TranslationKey::MessageNoCached);
        let embed = serenity::CreateEmbed::new()
            .title(title)
            .description(&description)
            .field(deleted_messages_label, no_cached, false)
            .color(warning_color)
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
                    .color(warning_color)
                    .timestamp(serenity::Timestamp::now())
                    .footer(serenity::CreateEmbedFooter::new(&footer_text));
                let chars = [title, &description, &field_name, chunk, &footer_text]
                    .iter()
                    .map(|value| value.chars().count())
                    .sum();
                embeds.push((embed, chars));
            } else {
                let field_name =
                    format!("{} [{}/{}]", deleted_messages_label, idx + 1, total_chunks);

                let embed = serenity::CreateEmbed::new()
                    .field(&field_name, chunk, false)
                    .color(warning_color);
                let chars = field_name.chars().count() + chunk.chars().count();
                embeds.push((embed, chars));
            }
        }
    }

    let mut messages: Vec<serenity::CreateMessage> = Vec::new();
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
            break;
        }
        messages.push(builder);
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn reply_link_points_to_the_referenced_message() {
        assert_eq!(
            message_url(GuildId::new(1), ChannelId::new(2), MessageId::new(3)),
            "https://discord.com/channels/1/2/3"
        );
    }

    #[test]
    fn purge_attachment_budget_includes_boundary_and_rejects_overflow() {
        assert!(fits_byte_budget(6, 4, 10));
        assert!(!fits_byte_budget(7, 4, 10));
        assert!(!fits_byte_budget(u64::MAX, 1, u64::MAX));
    }

    #[test]
    fn reply_field_formats_user_content_and_jump_link() {
        let mut msg = serenity::Message::default();
        let mut reference =
            serenity::MessageReference::from((ChannelId::new(2), MessageId::new(3)));
        reference.guild_id = Some(GuildId::new(1));
        msg.message_reference = Some(reference);
        let mut ref_msg = serenity::Message::default();
        ref_msg.author.id = serenity::UserId::new(99);
        ref_msg.content = "Test content".to_string();
        msg.referenced_message = Some(Box::new(ref_msg));

        let formatted = reply_field(Language::English, GuildId::new(1), &msg).unwrap();
        assert!(formatted.contains("<@99>: "));
        assert!(formatted.contains("[Jump to Message](https://discord.com/channels/1/2/3)"));
    }
}
