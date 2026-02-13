use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Vietnamese,
    Japanese,
}

impl Language {
    /// Parse from database string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "vi" | "vietnamese" => Language::Vietnamese,
            "ja" | "japanese" => Language::Japanese,
            _ => Language::English, // Default to English
        }
    }

    /// Convert to database string
    pub fn to_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Vietnamese => "vi",
            Language::Japanese => "ja",
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Vietnamese => "Tiếng Việt",
            Language::Japanese => "日本語",
        }
    }
}

/// Translation key enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TranslationKey {
    // Ping command
    PingPong,
    PingLatency,

    // Bot info command
    BotInfoTitle,
    BotInfoUptime,
    BotInfoServers,
    BotInfoLanguage,
    BotInfoFramework,

    // Server info command
    ServerInfoTitle,
    ServerInfoName,
    ServerInfoMembers,
    ServerInfoChannels,
    ServerInfoRoles,
    ServerInfoCreated,

    // Message log commands
    MessageLogEnabled,
    MessageLogDisabled,
    MessageLogNotSetup,
    MessageLogStatusTitle,
    MessageLogStatus,
    MessageLogStatusEnabled,
    MessageLogStatusDisabled,
    MessageLogChannel,
    MessageLogUseEnable,

    // Message log handlers
    MessageDeleted,
    MessageEditedTitle,
    MessageBulkDeleteTitle,
    MessageAuthor,
    MessageChannel,
    MessageContent,
    MessageBefore,
    MessageAfter,
    MessageJumpTo,
    MessageMediaOnly,
    MessageTotalDeleted,
    MessageCached,
    MessageUser,
    MessageBot,
    MessageDeletedMessages,
    MessagePurged,

    // Language command
    LanguageChanged,
    LanguageChangedTo,
    LanguageCurrent,
    LanguageAvailable,

    // Moderation commands
    KickSuccess,
    KickFailed,
    BanSuccess,
    BanFailed,
    PurgeSuccess,
    PurgeFailed,

    // Settings
    SettingsTitle,
    SettingsPrefix,
    SettingsLogChannel,
    SettingsNotConfigured,
    PrefixChanged,

    // Common
    ErrorNotInGuild,
    ErrorNoPermission,
    ErrorGeneric,
}

type TranslationMap = HashMap<TranslationKey, &'static str>;

/// Global translation storage
static TRANSLATIONS: Lazy<HashMap<Language, TranslationMap>> = Lazy::new(|| {
    let mut translations = HashMap::new();

    // English translations
    let mut en = HashMap::new();
    en.insert(TranslationKey::PingPong, "Pong!");
    en.insert(TranslationKey::PingLatency, "Pong! Latency: **{}ms**");
    en.insert(TranslationKey::BotInfoTitle, "**Bot Information**");
    en.insert(TranslationKey::BotInfoUptime, "**Uptime:** {}h {}m {}s");
    en.insert(TranslationKey::BotInfoServers, "**Servers:** {}");
    en.insert(TranslationKey::BotInfoLanguage, "**Language:** Rust");
    en.insert(TranslationKey::BotInfoFramework, "**Framework:** Poise + Serenity");
    en.insert(TranslationKey::ServerInfoTitle, "**Server Information**");
    en.insert(TranslationKey::ServerInfoName, "**Name:** {}");
    en.insert(TranslationKey::ServerInfoMembers, "**Members:** {}");
    en.insert(TranslationKey::ServerInfoChannels, "**Channels:** {}");
    en.insert(TranslationKey::ServerInfoRoles, "**Roles:** {}");
    en.insert(TranslationKey::ServerInfoCreated, "**Created:** <t:{}:R>");
    en.insert(TranslationKey::MessageLogEnabled, "Message logging enabled. Log channel: <#{}>");
    en.insert(TranslationKey::MessageLogDisabled, "Message logging disabled.");
    en.insert(TranslationKey::MessageLogNotSetup, "Message logging not configured.");
    en.insert(TranslationKey::MessageLogStatusTitle, "**Message Log Status**");
    en.insert(TranslationKey::MessageLogStatus, "**Status:**");
    en.insert(TranslationKey::MessageLogStatusEnabled, "Enabled");
    en.insert(TranslationKey::MessageLogStatusDisabled, "Disabled");
    en.insert(TranslationKey::MessageLogChannel, "**Log Channel:** <#{}>");
    en.insert(TranslationKey::MessageLogUseEnable, "Message logging not configured. Use `/messagelog enable` to enable.");
    en.insert(TranslationKey::MessageDeleted, "Message Deleted");
    en.insert(TranslationKey::MessageEditedTitle, "Message Edited");
    en.insert(TranslationKey::MessageBulkDeleteTitle, "Bulk Message Delete (Purge)");
    en.insert(TranslationKey::MessageAuthor, "**Author:** <@{}>");
    en.insert(TranslationKey::MessageChannel, "**Channel:** <#{}>");
    en.insert(TranslationKey::MessageContent, "**Content:**");
    en.insert(TranslationKey::MessageBefore, "Before");
    en.insert(TranslationKey::MessageAfter, "After");
    en.insert(TranslationKey::MessageJumpTo, "[Jump to Message]({})");
    en.insert(TranslationKey::MessageMediaOnly, "*[Media only]*");
    en.insert(TranslationKey::MessageTotalDeleted, "**Total Deleted:** {} messages");
    en.insert(TranslationKey::MessageCached, "**Cached:** {} messages ({} user, {} bot)");
    en.insert(TranslationKey::MessageUser, "user");
    en.insert(TranslationKey::MessageBot, "bot");
    en.insert(TranslationKey::MessageDeletedMessages, "Deleted Messages");
    en.insert(TranslationKey::MessagePurged, "{} messages purged");
    en.insert(TranslationKey::LanguageChanged, "Language changed to **{}**");
    en.insert(TranslationKey::LanguageChangedTo, "Language changed to");
    en.insert(TranslationKey::LanguageCurrent, "**Current Language:** {}");
    en.insert(TranslationKey::LanguageAvailable, "**Available:** English (en), Tiếng Việt (vi), 日本語 (ja)");
    en.insert(TranslationKey::SettingsTitle, "**Server Settings**");
    en.insert(TranslationKey::SettingsPrefix, "**Prefix:** `{}`");
    en.insert(TranslationKey::SettingsLogChannel, "**Log Channel:** {}");
    en.insert(TranslationKey::SettingsNotConfigured, "Not configured");
    en.insert(TranslationKey::PrefixChanged, "Prefix changed to `{}`");
    en.insert(TranslationKey::ErrorNotInGuild, "This command can only be used in a server.");
    en.insert(TranslationKey::ErrorNoPermission, "You don't have permission to use this command.");
    en.insert(TranslationKey::ErrorGeneric, "An error occurred: {}");
    translations.insert(Language::English, en);

    // Vietnamese translations
    let mut vi = HashMap::new();
    vi.insert(TranslationKey::PingPong, "Pong!");
    vi.insert(TranslationKey::PingLatency, "Pong! Độ trễ: **{}ms**");
    vi.insert(TranslationKey::BotInfoTitle, "**Thông Tin Bot**");
    vi.insert(TranslationKey::BotInfoUptime, "**Thời gian hoạt động:** {}h {}m {}s");
    vi.insert(TranslationKey::BotInfoServers, "**Máy chủ:** {}");
    vi.insert(TranslationKey::BotInfoLanguage, "**Ngôn ngữ lập trình:** Rust");
    vi.insert(TranslationKey::BotInfoFramework, "**Framework:** Poise + Serenity");
    vi.insert(TranslationKey::ServerInfoTitle, "**Thông Tin Máy Chủ**");
    vi.insert(TranslationKey::ServerInfoName, "**Tên:** {}");
    vi.insert(TranslationKey::ServerInfoMembers, "**Thành viên:** {}");
    vi.insert(TranslationKey::ServerInfoChannels, "**Kênh:** {}");
    vi.insert(TranslationKey::ServerInfoRoles, "**Vai trò:** {}");
    vi.insert(TranslationKey::ServerInfoCreated, "**Ngày tạo:** <t:{}:R>");
    vi.insert(TranslationKey::MessageLogEnabled, "Đã bật message log. Kênh log: <#{}>");
    vi.insert(TranslationKey::MessageLogDisabled, "Đã tắt message log.");
    vi.insert(TranslationKey::MessageLogNotSetup, "Message logging chưa được thiết lập.");
    vi.insert(TranslationKey::MessageLogStatusTitle, "**Trạng Thái Message Log**");
    vi.insert(TranslationKey::MessageLogStatus, "**Trạng thái:**");
    vi.insert(TranslationKey::MessageLogStatusEnabled, "Đang bật");
    vi.insert(TranslationKey::MessageLogStatusDisabled, "Đang tắt");
    vi.insert(TranslationKey::MessageLogChannel, "**Kênh log:** <#{}>");
    vi.insert(TranslationKey::MessageLogUseEnable, "Message logging chưa được thiết lập. Sử dụng `/messagelog enable` để bật.");
    vi.insert(TranslationKey::MessageDeleted, "Tin Nhắn Đã Xóa");
    vi.insert(TranslationKey::MessageEditedTitle, "Tin Nhắn Đã Chỉnh Sửa");
    vi.insert(TranslationKey::MessageBulkDeleteTitle, "Xóa Hàng Loạt Tin Nhắn");
    vi.insert(TranslationKey::MessageAuthor, "**Tác giả:** <@{}>");
    vi.insert(TranslationKey::MessageChannel, "**Kênh:** <#{}>");
    vi.insert(TranslationKey::MessageContent, "**Nội dung:**");
    vi.insert(TranslationKey::MessageBefore, "Trước");
    vi.insert(TranslationKey::MessageAfter, "Sau");
    vi.insert(TranslationKey::MessageJumpTo, "[Nhảy đến tin nhắn]({})");
    vi.insert(TranslationKey::MessageMediaOnly, "*[Chỉ có media]*");
    vi.insert(TranslationKey::MessageTotalDeleted, "**Tổng số đã xóa:** {} tin nhắn");
    vi.insert(TranslationKey::MessageCached, "**Đã lưu:** {} tin nhắn ({} người dùng, {} bot)");
    vi.insert(TranslationKey::MessageUser, "người dùng");
    vi.insert(TranslationKey::MessageBot, "bot");
    vi.insert(TranslationKey::MessageDeletedMessages, "Tin Nhắn Đã Xóa");
    vi.insert(TranslationKey::MessagePurged, "Đã xóa {} tin nhắn");
    vi.insert(TranslationKey::LanguageChanged, "Đã đổi ngôn ngữ sang **{}**");
    vi.insert(TranslationKey::LanguageChangedTo, "Đã đổi ngôn ngữ sang");
    vi.insert(TranslationKey::LanguageCurrent, "**Ngôn ngữ hiện tại:** {}");
    vi.insert(TranslationKey::LanguageAvailable, "**Có sẵn:** English (en), Tiếng Việt (vi), 日本語 (ja)");
    vi.insert(TranslationKey::SettingsTitle, "**Cấu Hình Server**");
    vi.insert(TranslationKey::SettingsPrefix, "**Prefix:** `{}`");
    vi.insert(TranslationKey::SettingsLogChannel, "**Kênh Log:** {}");
    vi.insert(TranslationKey::SettingsNotConfigured, "Chưa thiết lập");
    vi.insert(TranslationKey::PrefixChanged, "Đã đổi prefix thành `{}`");
    vi.insert(TranslationKey::ErrorNotInGuild, "Lệnh này chỉ có thể sử dụng trong máy chủ.");
    vi.insert(TranslationKey::ErrorNoPermission, "Bạn không có quyền sử dụng lệnh này.");
    vi.insert(TranslationKey::ErrorGeneric, "Đã xảy ra lỗi: {}");
    translations.insert(Language::Vietnamese, vi);

    // Japanese translations
    let mut ja = HashMap::new();
    ja.insert(TranslationKey::PingPong, "ポン！");
    ja.insert(TranslationKey::PingLatency, "ポン！レイテンシ：**{}ms**");
    ja.insert(TranslationKey::BotInfoTitle, "**ボット情報**");
    ja.insert(TranslationKey::BotInfoUptime, "**稼働時間：** {}時間 {}分 {}秒");
    ja.insert(TranslationKey::BotInfoServers, "**サーバー数：** {}");
    ja.insert(TranslationKey::BotInfoLanguage, "**プログラミング言語：** Rust");
    ja.insert(TranslationKey::BotInfoFramework, "**フレームワーク：** Poise + Serenity");
    ja.insert(TranslationKey::ServerInfoTitle, "**サーバー情報**");
    ja.insert(TranslationKey::ServerInfoName, "**名前：** {}");
    ja.insert(TranslationKey::ServerInfoMembers, "**メンバー数：** {}");
    ja.insert(TranslationKey::ServerInfoChannels, "**チャンネル数：** {}");
    ja.insert(TranslationKey::ServerInfoRoles, "**ロール数：** {}");
    ja.insert(TranslationKey::ServerInfoCreated, "**作成日：** <t:{}:R>");
    ja.insert(TranslationKey::MessageLogEnabled, "メッセージログが有効になりました。ログチャンネル：<#{}>");
    ja.insert(TranslationKey::MessageLogDisabled, "メッセージログを無効にしました。");
    ja.insert(TranslationKey::MessageLogNotSetup, "メッセージログは設定されていません。");
    ja.insert(TranslationKey::MessageLogStatusTitle, "**メッセージログのステータス**");
    ja.insert(TranslationKey::MessageLogStatus, "**ステータス：**");
    ja.insert(TranslationKey::MessageLogStatusEnabled, "有効");
    ja.insert(TranslationKey::MessageLogStatusDisabled, "無効");
    ja.insert(TranslationKey::MessageLogChannel, "**ログチャンネル：** <#{}>");
    ja.insert(TranslationKey::MessageLogUseEnable, "メッセージログは設定されていません。`/messagelog enable`で有効にしてください。");
    ja.insert(TranslationKey::MessageDeleted, "メッセージが削除されました");
    ja.insert(TranslationKey::MessageEditedTitle, "メッセージが編集されました");
    ja.insert(TranslationKey::MessageBulkDeleteTitle, "一括メッセージ削除");
    ja.insert(TranslationKey::MessageAuthor, "**作成者：** <@{}>");
    ja.insert(TranslationKey::MessageChannel, "**チャンネル：** <#{}>");
    ja.insert(TranslationKey::MessageContent, "**内容：**");
    ja.insert(TranslationKey::MessageBefore, "編集前");
    ja.insert(TranslationKey::MessageAfter, "編集後");
    ja.insert(TranslationKey::MessageJumpTo, "[メッセージへジャンプ]({})");
    ja.insert(TranslationKey::MessageMediaOnly, "*[メディアのみ]*");
    ja.insert(TranslationKey::MessageTotalDeleted, "**削除総数：** {}件のメッセージ");
    ja.insert(TranslationKey::MessageCached, "**キャッシュ：** {}件のメッセージ（{}ユーザー、{}ボット）");
    ja.insert(TranslationKey::MessageUser, "ユーザー");
    ja.insert(TranslationKey::MessageBot, "ボット");
    ja.insert(TranslationKey::MessageDeletedMessages, "削除されたメッセージ");
    ja.insert(TranslationKey::MessagePurged, "{}件のメッセージを削除しました");
    ja.insert(TranslationKey::LanguageChanged, "言語を**{}**に変更しました");
    ja.insert(TranslationKey::LanguageChangedTo, "言語を変更しました");
    ja.insert(TranslationKey::LanguageCurrent, "**現在の言語：** {}");
    ja.insert(TranslationKey::LanguageAvailable, "**利用可能：** English (en), Tiếng Việt (vi), 日本語 (ja)");
    ja.insert(TranslationKey::SettingsTitle, "**サーバー設定**");
    ja.insert(TranslationKey::SettingsPrefix, "**プレフィックス：** `{}`");
    ja.insert(TranslationKey::SettingsLogChannel, "**ログチャンネル：** {}");
    ja.insert(TranslationKey::SettingsNotConfigured, "未設定");
    ja.insert(TranslationKey::PrefixChanged, "プレフィックスを`{}`に変更しました");
    ja.insert(TranslationKey::ErrorNotInGuild, "このコマンドはサーバー内でのみ使用できます。");
    ja.insert(TranslationKey::ErrorNoPermission, "このコマンドを使用する権限がありません。");
    ja.insert(TranslationKey::ErrorGeneric, "エラーが発生しました：{}");
    translations.insert(Language::Japanese, ja);

    translations
});

/// Get translation for a key in the specified language
pub fn t(lang: Language, key: TranslationKey) -> &'static str {
    TRANSLATIONS
        .get(&lang)
        .and_then(|map| map.get(&key))
        .copied()
        .unwrap_or_else(|| {
            // Fallback to English if translation not found
            TRANSLATIONS
                .get(&Language::English)
                .and_then(|map| map.get(&key))
                .copied()
                .unwrap_or("Translation missing")
        })
}

/// Format translation with arguments
pub fn tf(lang: Language, key: TranslationKey, args: &[&dyn std::fmt::Display]) -> String {
    let template = t(lang, key);
    let mut result = template.to_string();

    for (i, arg) in args.iter().enumerate() {
        result = result.replace("{}", &arg.to_string());
        if i < args.len() - 1 {
            // Find next occurrence of {} for the next replacement
            continue;
        }
    }

    result
}

/// Get language for a guild from database
pub async fn get_guild_language(
    db_pool: &sqlx::SqlitePool,
    guild_id: serenity_prelude::GuildId,
) -> Language {
    let lang_str = sqlx::query_scalar::<_, String>(
        "SELECT language FROM guild_language WHERE guild_id = ?"
    )
        .bind(guild_id.to_string())
        .fetch_optional(db_pool)
        .await
        .ok()
        .flatten();

    match lang_str {
        Some(s) => Language::from_str(&s),
        None => Language::English, // Default to English
    }
}

/// Set language for a guild in database
pub async fn set_guild_language(
    db_pool: &sqlx::SqlitePool,
    guild_id: serenity_prelude::GuildId,
    language: Language,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO guild_language (guild_id, language)
         VALUES (?, ?)
         ON CONFLICT(guild_id) DO UPDATE SET language = excluded.language"
    )
        .bind(guild_id.to_string())
        .bind(language.to_str())
        .execute(db_pool)
        .await?;

    Ok(())
}

// Re-export serenity types for convenience
use poise::serenity_prelude;
