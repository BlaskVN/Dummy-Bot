use crate::Data;
use crate::message_log::{
    DiscordOutbox, HttpAttachmentFetcher, MessageLogOptions, MessageLogService,
};
use poise::serenity_prelude as serenity;
use serenity::{ChannelId, Context, MessageId, MessageUpdateEvent};

fn make_service<'a>(
    ctx: &'a Context,
    data: &'a Data,
) -> MessageLogService<'a, DiscordOutbox<'a>, HttpAttachmentFetcher> {
    let outbox = DiscordOutbox::new(&ctx.http);
    let fetcher = HttpAttachmentFetcher::new(
        data.attachment_client.clone(),
        Some(data.attachment_downloads.clone()),
    );
    MessageLogService::new(
        &data.db_pool,
        outbox,
        fetcher,
        MessageLogOptions {
            preview_chars: data.config.message_preview_chars,
            chunk_chars: data.config.message_log_chunk_chars,
            timestamp_format: data.config.message_timestamp_format.clone(),
            attachment_max_bytes: data.config.attachment_max_bytes,
            purge_attachment_max_total_bytes: data.config.purge_attachment_max_total_bytes,
            message_content_enabled: data.config.message_content_enabled,
            error_color: serenity::Colour(data.config.colors.error),
            warning_color: serenity::Colour(data.config.colors.warning),
        },
    )
}

pub async fn reconcile_all_health(ctx: &Context, data: &Data) {
    let service = make_service(ctx, data);
    service
        .reconcile_all_health(|guild_id| data.language(guild_id))
        .await;
}

/// Persist incoming non-bot guild message to SQLite cache for restart resilience.
pub async fn save_message(message: &serenity::Message, data: &Data) {
    MessageLogService::<DiscordOutbox<'_>, HttpAttachmentFetcher>::save_message(
        &data.db_pool,
        message,
    )
    .await;
}

/// Handle message deletion events.
pub async fn handle_message_delete(
    ctx: &Context,
    channel_id: ChannelId,
    deleted_message_id: MessageId,
    guild_id: Option<serenity::GuildId>,
    data: &Data,
) {
    let Some(guild_id) = guild_id else {
        return;
    };
    let lang = data.language(guild_id).await;
    let ram_msg = ctx
        .cache
        .message(channel_id, deleted_message_id)
        .map(|m| m.clone());

    let service = make_service(ctx, data);
    service
        .handle_message_delete(lang, channel_id, deleted_message_id, guild_id, ram_msg)
        .await;
}

/// Archive attachments fetched by `/purge` while their signed CDN URLs are still valid.
pub async fn archive_purge_attachments(
    ctx: &Context,
    guild_id: serenity::GuildId,
    messages: &[serenity::Message],
    data: &Data,
) {
    let service = make_service(ctx, data);
    service.archive_purge_attachments(guild_id, messages).await;
}

/// Handle message update (edit) events.
pub async fn handle_message_update(
    ctx: &Context,
    old_message: Option<&serenity::Message>,
    event: &MessageUpdateEvent,
    data: &Data,
) {
    let Some(guild_id) = event.guild_id else {
        return;
    };
    let lang = data.language(guild_id).await;
    let service = make_service(ctx, data);
    service
        .handle_message_update(lang, old_message, event)
        .await;
}

/// Handle bulk message deletion events (purge/prune).
pub async fn handle_message_delete_bulk(
    ctx: &Context,
    channel_id: ChannelId,
    deleted_message_ids: &[MessageId],
    guild_id: Option<serenity::GuildId>,
    data: &Data,
) {
    let Some(guild_id) = guild_id else {
        return;
    };
    let lang = data.language(guild_id).await;
    let service = make_service(ctx, data);
    service
        .handle_message_delete_bulk(lang, channel_id, deleted_message_ids, guild_id, |ch, id| {
            ctx.cache.message(ch, id).map(|m| m.clone())
        })
        .await;
}
