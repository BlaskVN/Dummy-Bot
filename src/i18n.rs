use poise::serenity_prelude;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    English,
    Vietnamese,
    Japanese,
}

impl Language {
    pub fn try_parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "en" | "english" => Some(Language::English),
            "vi" | "vietnamese" | "tiếng việt" => Some(Language::Vietnamese),
            "ja" | "japanese" | "日本語" => Some(Language::Japanese),
            _ => None,
        }
    }

    /// Parse stored data, falling back to English for backward compatibility.
    pub fn parse(s: &str) -> Self {
        Self::try_parse(s).unwrap_or(Language::English)
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
    BotInfoDonate,

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
    MessageLogHealth,
    MessageLogHealthDisabled,
    MessageLogHealthHealthy,
    MessageLogHealthDegraded,
    MessageLogDegradedWarning,

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
    MessageAuthorLabel,
    MessageChannelLabel,
    MessageId,
    MessageIdValue,
    MessageDeletedAt,
    MessageUnknownTimestamp,
    MessageNoCached,

    // Language command
    LanguageChanged,
    LanguageChangedTo,
    LanguageCurrent,
    LanguageAvailable,
    LanguageInvalid,

    // Moderation commands
    ModerationNoReason,
    ModerationKicked,
    ModerationKickReason,
    ModerationBanned,
    ModerationBanReason,
    ModerationPurged,
    ModerationInvalidArgument,
    ModerationBotMissingPermissions,
    ModerationUserMissingPermissions,
    ModerationCannotTargetSelf,
    ModerationUserHierarchy,
    ModerationBotHierarchy,
    ModerationDeleteDaysRange,
    ModerationPurgeRange,
    ModerationChannelSet,
    ModerationChannelCurrent,
    ModerationChannelNotConfigured,
    ModerationChannelCleared,
    ModerationChannelInvalid,
    ModerationReasonRequired,
    ModerationInvalidEvidence,
    ModerationTimeoutRange,
    ModerationActionWarn,
    ModerationActionKick,
    ModerationActionBan,
    ModerationActionTimeout,
    ModerationCaseSummary,
    ModerationActionCaseFailed,
    ModerationActionUnknown,
    ModerationCaseNotFound,
    ModerationCaseListEmpty,
    ModerationCaseList,
    ModerationCaseListRow,
    ModerationCaseView,
    ModerationCaseStatusActive,
    ModerationCaseStatusVoided,
    ModerationCaseVoidDetails,
    ModerationCaseAlreadyVoided,
    ModerationCaseVoided,
    AutoModObserverEnabled,
    AutoModObserverDisabled,
    AutoModObserverNeedsChannel,
    AutoModObserverNeedsPermission,
    AutoModExecution,
    AutoModSuggestion,
    AutoModSuggestionHandled,
    AutoModSuggestionNotFound,
    AutoModSuggestionInvalidRule,
    AutoModSuggestionDelivered,
    AutoModSuggestionDeliveryFailed,

    // Settings
    SettingsTitle,
    SettingsPrefix,
    SettingsLogChannel,
    SettingsModerationChannel,
    SettingsTimezone,
    SettingsNotConfigured,
    PrefixChanged,
    PrefixInvalidLength,

    // Time zone commands
    TimezoneSet,
    TimezoneCurrent,
    TimezoneNotConfigured,
    TimezoneCleared,
    TimezoneInvalid,

    // Installation onboarding
    OnboardingMessage,

    // Donation configuration
    DonationSaved,
    DonationCleared,
    DonationInvalidUpdate,
    DonationInvalidUrl,
    DonationInvalidImage,
    DonateNotConfigured,

    // Presence commands
    PresenceTitle,
    PresenceHelp,
    PresenceStatusTitle,
    PresenceStatusSet,
    PresenceStatusSetDuration,
    PresenceActivityTitle,
    PresenceActivitySet,
    PresenceActivitySetDuration,
    PresenceActivityCleared,
    PresenceOwnerOnly,
    PresenceDurationRange,
    PresencePersistent,

    // Voice commands
    VoiceConnected,
    VoiceDisconnected,
    VoiceNotInChannel,
    VoiceNotConnected,
    VoiceAlreadyConnected,
    VoiceJoinFailed,
    VoiceKicked,

    // Common
    ErrorNotInGuild,
    ErrorNoPermission,
    ErrorGeneric,
    ErrorCooldown,
}

type TranslationMap = HashMap<TranslationKey, &'static str>;

/// Global translation storage
static TRANSLATIONS: LazyLock<HashMap<Language, TranslationMap>> = LazyLock::new(|| {
    let mut translations = HashMap::new();

    // English translations
    let mut en = HashMap::new();
    en.insert(TranslationKey::PingPong, "Pong!");
    en.insert(TranslationKey::PingLatency, "Pong! Latency: **{}ms**");
    en.insert(TranslationKey::BotInfoTitle, "**Bot Information**");
    en.insert(TranslationKey::BotInfoUptime, "**Uptime:** {}h {}m {}s");
    en.insert(TranslationKey::BotInfoServers, "**Servers:** {}");
    en.insert(TranslationKey::BotInfoLanguage, "**Language:** Rust");
    en.insert(
        TranslationKey::BotInfoFramework,
        "**Framework:** Poise + Serenity",
    );
    en.insert(
        TranslationKey::BotInfoDonate,
        "Donation information: `/donate`",
    );
    en.insert(TranslationKey::ServerInfoTitle, "**Server Information**");
    en.insert(TranslationKey::ServerInfoName, "**Name:** {}");
    en.insert(TranslationKey::ServerInfoMembers, "**Members:** {}");
    en.insert(TranslationKey::ServerInfoChannels, "**Channels:** {}");
    en.insert(TranslationKey::ServerInfoRoles, "**Roles:** {}");
    en.insert(TranslationKey::ServerInfoCreated, "**Created:** <t:{}:R>");
    en.insert(
        TranslationKey::MessageLogEnabled,
        "Message logging enabled. Log channel: <#{}>",
    );
    en.insert(
        TranslationKey::MessageLogDisabled,
        "Message logging disabled.",
    );
    en.insert(
        TranslationKey::MessageLogNotSetup,
        "Message logging not configured.",
    );
    en.insert(
        TranslationKey::MessageLogStatusTitle,
        "**Message Log Status**",
    );
    en.insert(TranslationKey::MessageLogStatus, "**Status:**");
    en.insert(TranslationKey::MessageLogStatusEnabled, "Enabled");
    en.insert(TranslationKey::MessageLogStatusDisabled, "Disabled");
    en.insert(TranslationKey::MessageLogChannel, "**Log Channel:** <#{}>");
    en.insert(
        TranslationKey::MessageLogUseEnable,
        "Message logging not configured. Use `/messagelog enable` to enable.",
    );
    en.insert(TranslationKey::MessageLogHealth, "**Health:** {}");
    en.insert(TranslationKey::MessageLogHealthDisabled, "Disabled");
    en.insert(TranslationKey::MessageLogHealthHealthy, "Healthy");
    en.insert(TranslationKey::MessageLogHealthDegraded, "Degraded");
    en.insert(TranslationKey::MessageLogDegradedWarning, "Message Log is degraded because Message Content access is disabled. Metadata will still be logged, but message text and attachments may be unavailable.");
    en.insert(TranslationKey::MessageDeleted, "Message Deleted");
    en.insert(TranslationKey::MessageEditedTitle, "Message Edited");
    en.insert(
        TranslationKey::MessageBulkDeleteTitle,
        "Bulk Message Delete (Purge)",
    );
    en.insert(TranslationKey::MessageAuthor, "**Author:** <@{}>");
    en.insert(TranslationKey::MessageChannel, "**Channel:** <#{}>");
    en.insert(TranslationKey::MessageContent, "**Content:**");
    en.insert(TranslationKey::MessageBefore, "Before");
    en.insert(TranslationKey::MessageAfter, "After");
    en.insert(TranslationKey::MessageJumpTo, "[Jump to Message]({})");
    en.insert(TranslationKey::MessageMediaOnly, "*[Media only]*");
    en.insert(
        TranslationKey::MessageTotalDeleted,
        "**Total Deleted:** {} messages",
    );
    en.insert(
        TranslationKey::MessageCached,
        "**Cached:** {} messages ({} user, {} bot)",
    );
    en.insert(TranslationKey::MessageUser, "user");
    en.insert(TranslationKey::MessageBot, "bot");
    en.insert(TranslationKey::MessageDeletedMessages, "Deleted Messages");
    en.insert(TranslationKey::MessagePurged, "{} messages purged");
    en.insert(TranslationKey::MessageAuthorLabel, "Author");
    en.insert(TranslationKey::MessageChannelLabel, "Channel");
    en.insert(TranslationKey::MessageId, "ID");
    en.insert(TranslationKey::MessageIdValue, "Message ID: {}");
    en.insert(TranslationKey::MessageDeletedAt, "Deleted at");
    en.insert(TranslationKey::MessageUnknownTimestamp, "Unknown time");
    en.insert(
        TranslationKey::MessageNoCached,
        "*No cached messages to display*",
    );
    en.insert(
        TranslationKey::LanguageChanged,
        "Language changed to **{}**",
    );
    en.insert(TranslationKey::LanguageChangedTo, "Language changed to");
    en.insert(TranslationKey::LanguageCurrent, "**Current Language:** {}");
    en.insert(
        TranslationKey::LanguageAvailable,
        "**Available:** English (en), Tiếng Việt (vi), 日本語 (ja)",
    );
    en.insert(
        TranslationKey::LanguageInvalid,
        "Invalid language. Available: `en`, `vi`, `ja`.",
    );
    en.insert(TranslationKey::ModerationNoReason, "No reason provided");
    en.insert(
        TranslationKey::ModerationKicked,
        "Kicked **{}**\nReason: ```{}```",
    );
    en.insert(TranslationKey::ModerationKickReason, "Reason");
    en.insert(
        TranslationKey::ModerationBanned,
        "Banned **{}**\nReason: ```{}```",
    );
    en.insert(TranslationKey::ModerationBanReason, "Reason");
    en.insert(TranslationKey::ModerationPurged, "Deleted **{}** messages.");
    en.insert(
        TranslationKey::ModerationInvalidArgument,
        "Invalid argument: {}",
    );
    en.insert(
        TranslationKey::ModerationBotMissingPermissions,
        "Bot missing permissions: {}",
    );
    en.insert(
        TranslationKey::ModerationUserMissingPermissions,
        "You're missing permissions: {}",
    );
    en.insert(
        TranslationKey::ModerationCannotTargetSelf,
        "You cannot moderate yourself or the bot.",
    );
    en.insert(
        TranslationKey::ModerationUserHierarchy,
        "Discord role hierarchy does not allow you to moderate this member.",
    );
    en.insert(
        TranslationKey::ModerationBotHierarchy,
        "Move the bot role above this member's highest role first.",
    );
    en.insert(
        TranslationKey::ModerationDeleteDaysRange,
        "Message deletion must be between 0 and {} days.",
    );
    en.insert(
        TranslationKey::ModerationPurgeRange,
        "Message count must be between 1 and {}.",
    );
    en.insert(
        TranslationKey::ModerationChannelSet,
        "Moderation channel set to <#{}>.",
    );
    en.insert(
        TranslationKey::ModerationChannelCurrent,
        "**Moderation channel:** <#{}>",
    );
    en.insert(
        TranslationKey::ModerationChannelNotConfigured,
        "Moderation channel is not configured.",
    );
    en.insert(
        TranslationKey::ModerationChannelCleared,
        "Moderation channel cleared.",
    );
    en.insert(
        TranslationKey::ModerationChannelInvalid,
        "Choose a text channel from this server.",
    );
    en.insert(
        TranslationKey::ModerationReasonRequired,
        "Reason is required.",
    );
    en.insert(
        TranslationKey::ModerationInvalidEvidence,
        "Evidence must be a Discord message link from this server.",
    );
    en.insert(
        TranslationKey::ModerationTimeoutRange,
        "Timeout must be between 1 and 40,320 minutes.",
    );
    en.insert(TranslationKey::ModerationActionWarn, "Warning");
    en.insert(TranslationKey::ModerationActionKick, "Kick");
    en.insert(TranslationKey::ModerationActionBan, "Ban");
    en.insert(TranslationKey::ModerationActionTimeout, "Timeout");
    en.insert(
        TranslationKey::ModerationCaseSummary,
        "Case #{} — {}\nTarget: <@{}>\nModerator: <@{}>\nReason: {}",
    );
    en.insert(TranslationKey::ModerationActionCaseFailed, "Discord action succeeded, but its moderation case could not be recorded. Contact the Bot Owner.");
    en.insert(TranslationKey::ModerationActionUnknown, "Unknown action");
    en.insert(
        TranslationKey::ModerationCaseNotFound,
        "Moderation case not found.",
    );
    en.insert(
        TranslationKey::ModerationCaseListEmpty,
        "No moderation cases on this page.",
    );
    en.insert(
        TranslationKey::ModerationCaseList,
        "**Moderation cases — page {}**\n{}",
    );
    en.insert(
        TranslationKey::ModerationCaseListRow,
        "#{} — {} — <@{}> — {}",
    );
    en.insert(TranslationKey::ModerationCaseView, "**Case #{} — {}**\nTarget: <@{}>\nModerator: <@{}>\nReason: {}\nCreated: {}\nEvidence: {}\nStatus: {}");
    en.insert(TranslationKey::ModerationCaseStatusActive, "Active");
    en.insert(TranslationKey::ModerationCaseStatusVoided, "Voided");
    en.insert(
        TranslationKey::ModerationCaseVoidDetails,
        "\nVoided by: <@{}>\nVoid reason: {}\nVoided at: {}",
    );
    en.insert(
        TranslationKey::ModerationCaseAlreadyVoided,
        "Moderation case is already voided.",
    );
    en.insert(
        TranslationKey::ModerationCaseVoided,
        "Moderation case #{} voided.",
    );
    en.insert(
        TranslationKey::AutoModObserverEnabled,
        "Discord AutoMod observation is enabled.",
    );
    en.insert(
        TranslationKey::AutoModObserverDisabled,
        "Discord AutoMod observation is disabled.",
    );
    en.insert(
        TranslationKey::AutoModObserverNeedsChannel,
        "Configure a moderation channel first.",
    );
    en.insert(
        TranslationKey::AutoModObserverNeedsPermission,
        "The bot needs Manage Server to receive Discord AutoMod events.",
    );
    en.insert(
        TranslationKey::AutoModExecution,
        "Discord AutoMod execution\nRule: {}\nMember: <@{}>\nAction: {}\nMessage: {}",
    );
    en.insert(TranslationKey::AutoModSuggestion, "Review suggested: member <@{}> triggered AutoMod rule {} at least three times in seven days. This does not apply discipline automatically.");
    en.insert(
        TranslationKey::AutoModSuggestionHandled,
        "AutoMod suggestion marked handled.",
    );
    en.insert(
        TranslationKey::AutoModSuggestionNotFound,
        "No open AutoMod suggestion matches that member and rule.",
    );
    en.insert(
        TranslationKey::AutoModSuggestionInvalidRule,
        "Use a valid Discord AutoMod rule ID.",
    );
    en.insert(
        TranslationKey::AutoModSuggestionDelivered,
        "AutoMod suggestion delivered.",
    );
    en.insert(
        TranslationKey::AutoModSuggestionDeliveryFailed,
        "AutoMod suggestion remains open, but delivery failed.",
    );
    en.insert(TranslationKey::SettingsTitle, "**Server Settings**");
    en.insert(TranslationKey::SettingsPrefix, "**Prefix:** `{}`");
    en.insert(TranslationKey::SettingsLogChannel, "**Log Channel:** {}");
    en.insert(
        TranslationKey::SettingsModerationChannel,
        "**Moderation Channel:** {}",
    );
    en.insert(TranslationKey::SettingsTimezone, "**Time zone:** {}");
    en.insert(TranslationKey::SettingsNotConfigured, "Not configured");
    en.insert(TranslationKey::PrefixChanged, "Prefix changed to `{}`");
    en.insert(
        TranslationKey::PrefixInvalidLength,
        "Prefix must contain 1 to {} characters.",
    );
    en.insert(TranslationKey::TimezoneSet, "Time zone set to `{}`.");
    en.insert(TranslationKey::TimezoneCurrent, "**Time zone:** `{}`");
    en.insert(
        TranslationKey::TimezoneNotConfigured,
        "Time zone is not configured.",
    );
    en.insert(TranslationKey::TimezoneCleared, "Time zone cleared.");
    en.insert(
        TranslationKey::TimezoneInvalid,
        "Use a valid IANA time zone, such as `Asia/Bangkok`.",
    );
    en.insert(TranslationKey::OnboardingMessage, "Thanks for installing me! Start with `/settings`, then choose a language and time zone. Automations are opt-in.");
    en.insert(TranslationKey::DonationSaved, "Donation information saved.");
    en.insert(
        TranslationKey::DonationCleared,
        "Donation information cleared.",
    );
    en.insert(
        TranslationKey::DonationInvalidUpdate,
        "Provide a message, HTTPS URL, or PNG/JPEG QR image.",
    );
    en.insert(
        TranslationKey::DonationInvalidUrl,
        "Donation URL must use HTTPS.",
    );
    en.insert(
        TranslationKey::DonationInvalidImage,
        "QR image must be a PNG or JPEG within the attachment size limit.",
    );
    en.insert(
        TranslationKey::DonateNotConfigured,
        "Donation information is not configured.",
    );
    en.insert(TranslationKey::PresenceTitle, "**Bot Presence Management**");
    en.insert(TranslationKey::PresenceHelp, "Use subcommands to manage the bot's presence:\n├ `/presence status` — Set online status\n├ `/presence activity` — Set Rich Presence\n└ `/presence clear` — Clear activity");
    en.insert(TranslationKey::PresenceStatusTitle, "**Status Updated**");
    en.insert(
        TranslationKey::PresenceStatusSet,
        "Bot status set to **{}**",
    );
    en.insert(
        TranslationKey::PresenceStatusSetDuration,
        "Bot status set to **{}** for **{} minutes**",
    );
    en.insert(
        TranslationKey::PresenceActivityTitle,
        "**Activity Updated**",
    );
    en.insert(
        TranslationKey::PresenceActivitySet,
        "Activity set to **{} {}**\nStatus: **{}**",
    );
    en.insert(
        TranslationKey::PresenceActivitySetDuration,
        "Activity set to **{} {}**\nStatus: **{}**\nReverts in **{} minutes**",
    );
    en.insert(
        TranslationKey::PresenceActivityCleared,
        "Activity cleared and status reset to **Online**",
    );
    en.insert(
        TranslationKey::PresenceOwnerOnly,
        "Only the bot owner can use this command.",
    );
    en.insert(
        TranslationKey::PresenceDurationRange,
        "Duration must be between 0 and {} minutes.",
    );
    en.insert(
        TranslationKey::PresencePersistent,
        "Saved persistently — will restore on bot restart",
    );
    en.insert(TranslationKey::VoiceConnected, "Connected to <#{}>");
    en.insert(
        TranslationKey::VoiceDisconnected,
        "Disconnected from voice channel.",
    );
    en.insert(
        TranslationKey::VoiceNotInChannel,
        "You are not in a voice channel.",
    );
    en.insert(
        TranslationKey::VoiceNotConnected,
        "Bot is not connected to any voice channel.",
    );
    en.insert(
        TranslationKey::VoiceAlreadyConnected,
        "Bot is already connected to a voice channel. Use `/disconnect` first.",
    );
    en.insert(
        TranslationKey::VoiceJoinFailed,
        "Failed to join voice channel.",
    );
    en.insert(
        TranslationKey::VoiceKicked,
        "Bot left the voice channel. Use `/connect` to rejoin.",
    );
    en.insert(
        TranslationKey::ErrorNotInGuild,
        "This command can only be used in a server.",
    );
    en.insert(
        TranslationKey::ErrorNoPermission,
        "You don't have permission to use this command.",
    );
    en.insert(TranslationKey::ErrorGeneric, "An internal error occurred.");
    en.insert(
        TranslationKey::ErrorCooldown,
        "Please wait {} seconds before using this command again.",
    );
    translations.insert(Language::English, en);

    // Vietnamese translations
    let mut vi = HashMap::new();
    vi.insert(TranslationKey::PingPong, "Pong!");
    vi.insert(TranslationKey::PingLatency, "Pong! Độ trễ: **{}ms**");
    vi.insert(TranslationKey::BotInfoTitle, "**Thông Tin Bot**");
    vi.insert(
        TranslationKey::BotInfoUptime,
        "**Thời gian hoạt động:** {}h {}m {}s",
    );
    vi.insert(TranslationKey::BotInfoServers, "**Máy chủ:** {}");
    vi.insert(
        TranslationKey::BotInfoLanguage,
        "**Ngôn ngữ lập trình:** Rust",
    );
    vi.insert(
        TranslationKey::BotInfoFramework,
        "**Framework:** Poise + Serenity",
    );
    vi.insert(TranslationKey::BotInfoDonate, "Thông tin ủng hộ: `/donate`");
    vi.insert(TranslationKey::ServerInfoTitle, "**Thông Tin Máy Chủ**");
    vi.insert(TranslationKey::ServerInfoName, "**Tên:** {}");
    vi.insert(TranslationKey::ServerInfoMembers, "**Thành viên:** {}");
    vi.insert(TranslationKey::ServerInfoChannels, "**Kênh:** {}");
    vi.insert(TranslationKey::ServerInfoRoles, "**Vai trò:** {}");
    vi.insert(TranslationKey::ServerInfoCreated, "**Ngày tạo:** <t:{}:R>");
    vi.insert(
        TranslationKey::MessageLogEnabled,
        "Đã bật message log. Kênh log: <#{}>",
    );
    vi.insert(TranslationKey::MessageLogDisabled, "Đã tắt message log.");
    vi.insert(
        TranslationKey::MessageLogNotSetup,
        "Message logging chưa được thiết lập.",
    );
    vi.insert(
        TranslationKey::MessageLogStatusTitle,
        "**Trạng Thái Message Log**",
    );
    vi.insert(TranslationKey::MessageLogStatus, "**Trạng thái:**");
    vi.insert(TranslationKey::MessageLogStatusEnabled, "Đang bật");
    vi.insert(TranslationKey::MessageLogStatusDisabled, "Đang tắt");
    vi.insert(TranslationKey::MessageLogChannel, "**Kênh log:** <#{}>");
    vi.insert(
        TranslationKey::MessageLogUseEnable,
        "Message logging chưa được thiết lập. Sử dụng `/messagelog enable` để bật.",
    );
    vi.insert(TranslationKey::MessageLogHealth, "**Tình trạng:** {}");
    vi.insert(TranslationKey::MessageLogHealthDisabled, "Đã tắt");
    vi.insert(TranslationKey::MessageLogHealthHealthy, "Bình thường");
    vi.insert(TranslationKey::MessageLogHealthDegraded, "Suy giảm");
    vi.insert(TranslationKey::MessageLogDegradedWarning, "Message Log đang suy giảm vì quyền truy cập Message Content bị tắt. Bot vẫn ghi metadata nhưng có thể không có nội dung và tệp đính kèm.");
    vi.insert(TranslationKey::MessageDeleted, "Tin Nhắn Đã Xóa");
    vi.insert(TranslationKey::MessageEditedTitle, "Tin Nhắn Đã Chỉnh Sửa");
    vi.insert(
        TranslationKey::MessageBulkDeleteTitle,
        "Xóa Hàng Loạt Tin Nhắn",
    );
    vi.insert(TranslationKey::MessageAuthor, "**Tác giả:** <@{}>");
    vi.insert(TranslationKey::MessageChannel, "**Kênh:** <#{}>");
    vi.insert(TranslationKey::MessageContent, "**Nội dung:**");
    vi.insert(TranslationKey::MessageBefore, "Trước");
    vi.insert(TranslationKey::MessageAfter, "Sau");
    vi.insert(TranslationKey::MessageJumpTo, "[Nhảy đến tin nhắn]({})");
    vi.insert(TranslationKey::MessageMediaOnly, "*[Chỉ có media]*");
    vi.insert(
        TranslationKey::MessageTotalDeleted,
        "**Tổng số đã xóa:** {} tin nhắn",
    );
    vi.insert(
        TranslationKey::MessageCached,
        "**Đã lưu:** {} tin nhắn ({} người dùng, {} bot)",
    );
    vi.insert(TranslationKey::MessageUser, "người dùng");
    vi.insert(TranslationKey::MessageBot, "bot");
    vi.insert(TranslationKey::MessageDeletedMessages, "Tin Nhắn Đã Xóa");
    vi.insert(TranslationKey::MessagePurged, "Đã xóa {} tin nhắn");
    vi.insert(TranslationKey::MessageAuthorLabel, "Tác giả");
    vi.insert(TranslationKey::MessageChannelLabel, "Kênh");
    vi.insert(TranslationKey::MessageId, "ID");
    vi.insert(TranslationKey::MessageIdValue, "ID tin nhắn: {}");
    vi.insert(TranslationKey::MessageDeletedAt, "Thời điểm xóa");
    vi.insert(
        TranslationKey::MessageUnknownTimestamp,
        "Không rõ thời gian",
    );
    vi.insert(
        TranslationKey::MessageNoCached,
        "*Không có tin nhắn cache để hiển thị*",
    );
    vi.insert(
        TranslationKey::LanguageChanged,
        "Đã đổi ngôn ngữ sang **{}**",
    );
    vi.insert(TranslationKey::LanguageChangedTo, "Đã đổi ngôn ngữ sang");
    vi.insert(TranslationKey::LanguageCurrent, "**Ngôn ngữ hiện tại:** {}");
    vi.insert(
        TranslationKey::LanguageAvailable,
        "**Có sẵn:** English (en), Tiếng Việt (vi), 日本語 (ja)",
    );
    vi.insert(
        TranslationKey::LanguageInvalid,
        "Ngôn ngữ không hợp lệ. Có sẵn: `en`, `vi`, `ja`.",
    );
    vi.insert(TranslationKey::ModerationNoReason, "Không có lý do");
    vi.insert(
        TranslationKey::ModerationKicked,
        "Đã kick **{}**\nLý do: ```{}```",
    );
    vi.insert(TranslationKey::ModerationKickReason, "Lý do");
    vi.insert(
        TranslationKey::ModerationBanned,
        "Đã ban **{}**\nLý do: ```{}```",
    );
    vi.insert(TranslationKey::ModerationBanReason, "Lý do");
    vi.insert(TranslationKey::ModerationPurged, "Đã xóa **{}** tin nhắn.");
    vi.insert(
        TranslationKey::ModerationInvalidArgument,
        "Tham số không hợp lệ: {}",
    );
    vi.insert(
        TranslationKey::ModerationBotMissingPermissions,
        "Bot thiếu quyền: {}",
    );
    vi.insert(
        TranslationKey::ModerationUserMissingPermissions,
        "Bạn thiếu quyền: {}",
    );
    vi.insert(
        TranslationKey::ModerationCannotTargetSelf,
        "Bạn không thể kiểm duyệt chính mình hoặc bot.",
    );
    vi.insert(
        TranslationKey::ModerationUserHierarchy,
        "Phân cấp role của Discord không cho phép bạn kiểm duyệt thành viên này.",
    );
    vi.insert(
        TranslationKey::ModerationBotHierarchy,
        "Hãy đưa role của bot lên trên role cao nhất của thành viên này.",
    );
    vi.insert(
        TranslationKey::ModerationDeleteDaysRange,
        "Số ngày xóa tin nhắn phải từ 0 đến {}.",
    );
    vi.insert(
        TranslationKey::ModerationPurgeRange,
        "Số tin nhắn phải từ 1 đến {}.",
    );
    vi.insert(
        TranslationKey::ModerationChannelSet,
        "Đã đặt kênh kiểm duyệt thành <#{}>.",
    );
    vi.insert(
        TranslationKey::ModerationChannelCurrent,
        "**Kênh kiểm duyệt:** <#{}>",
    );
    vi.insert(
        TranslationKey::ModerationChannelNotConfigured,
        "Kênh kiểm duyệt chưa được thiết lập.",
    );
    vi.insert(
        TranslationKey::ModerationChannelCleared,
        "Đã xóa kênh kiểm duyệt.",
    );
    vi.insert(
        TranslationKey::ModerationChannelInvalid,
        "Hãy chọn kênh văn bản trong server này.",
    );
    vi.insert(
        TranslationKey::ModerationReasonRequired,
        "Bắt buộc phải có lý do.",
    );
    vi.insert(
        TranslationKey::ModerationInvalidEvidence,
        "Bằng chứng phải là liên kết tin nhắn Discord trong server này.",
    );
    vi.insert(
        TranslationKey::ModerationTimeoutRange,
        "Thời gian timeout phải từ 1 đến 40.320 phút.",
    );
    vi.insert(TranslationKey::ModerationActionWarn, "Cảnh cáo");
    vi.insert(TranslationKey::ModerationActionKick, "Kick");
    vi.insert(TranslationKey::ModerationActionBan, "Ban");
    vi.insert(TranslationKey::ModerationActionTimeout, "Timeout");
    vi.insert(
        TranslationKey::ModerationCaseSummary,
        "Vụ việc #{} — {}\nThành viên: <@{}>\nNgười kiểm duyệt: <@{}>\nLý do: {}",
    );
    vi.insert(TranslationKey::ModerationActionCaseFailed, "Thao tác Discord đã thành công nhưng không thể ghi vụ việc kiểm duyệt. Hãy liên hệ Chủ Bot.");
    vi.insert(
        TranslationKey::ModerationActionUnknown,
        "Thao tác không xác định",
    );
    vi.insert(
        TranslationKey::ModerationCaseNotFound,
        "Không tìm thấy vụ việc kiểm duyệt.",
    );
    vi.insert(
        TranslationKey::ModerationCaseListEmpty,
        "Trang này không có vụ việc kiểm duyệt.",
    );
    vi.insert(
        TranslationKey::ModerationCaseList,
        "**Các vụ việc kiểm duyệt — trang {}**\n{}",
    );
    vi.insert(
        TranslationKey::ModerationCaseListRow,
        "#{} — {} — <@{}> — {}",
    );
    vi.insert(TranslationKey::ModerationCaseView, "**Vụ việc #{} — {}**\nThành viên: <@{}>\nNgười kiểm duyệt: <@{}>\nLý do: {}\nTạo lúc: {}\nBằng chứng: {}\nTrạng thái: {}");
    vi.insert(TranslationKey::ModerationCaseStatusActive, "Đang hiệu lực");
    vi.insert(TranslationKey::ModerationCaseStatusVoided, "Đã vô hiệu");
    vi.insert(
        TranslationKey::ModerationCaseVoidDetails,
        "\nNgười vô hiệu: <@{}>\nLý do vô hiệu: {}\nVô hiệu lúc: {}",
    );
    vi.insert(
        TranslationKey::ModerationCaseAlreadyVoided,
        "Vụ việc kiểm duyệt đã bị vô hiệu.",
    );
    vi.insert(
        TranslationKey::ModerationCaseVoided,
        "Đã vô hiệu vụ việc kiểm duyệt #{}.",
    );
    vi.insert(
        TranslationKey::AutoModObserverEnabled,
        "Đã bật theo dõi Discord AutoMod.",
    );
    vi.insert(
        TranslationKey::AutoModObserverDisabled,
        "Đã tắt theo dõi Discord AutoMod.",
    );
    vi.insert(
        TranslationKey::AutoModObserverNeedsChannel,
        "Hãy thiết lập kênh kiểm duyệt trước.",
    );
    vi.insert(
        TranslationKey::AutoModObserverNeedsPermission,
        "Bot cần quyền Quản lý Server để nhận sự kiện Discord AutoMod.",
    );
    vi.insert(
        TranslationKey::AutoModExecution,
        "Thực thi Discord AutoMod\nQuy tắc: {}\nThành viên: <@{}>\nThao tác: {}\nTin nhắn: {}",
    );
    vi.insert(TranslationKey::AutoModSuggestion, "Đề xuất xem xét: thành viên <@{}> đã kích hoạt quy tắc AutoMod {} ít nhất ba lần trong bảy ngày. Bot không tự động kỷ luật.");
    vi.insert(
        TranslationKey::AutoModSuggestionHandled,
        "Đã đánh dấu đề xuất AutoMod là đã xử lý.",
    );
    vi.insert(
        TranslationKey::AutoModSuggestionNotFound,
        "Không có đề xuất AutoMod đang mở khớp với thành viên và quy tắc đó.",
    );
    vi.insert(
        TranslationKey::AutoModSuggestionInvalidRule,
        "Hãy dùng ID quy tắc Discord AutoMod hợp lệ.",
    );
    vi.insert(
        TranslationKey::AutoModSuggestionDelivered,
        "Đã gửi đề xuất AutoMod.",
    );
    vi.insert(
        TranslationKey::AutoModSuggestionDeliveryFailed,
        "Đề xuất AutoMod vẫn mở nhưng gửi thất bại.",
    );
    vi.insert(TranslationKey::SettingsTitle, "**Cấu Hình Server**");
    vi.insert(TranslationKey::SettingsPrefix, "**Prefix:** `{}`");
    vi.insert(TranslationKey::SettingsLogChannel, "**Kênh Log:** {}");
    vi.insert(
        TranslationKey::SettingsModerationChannel,
        "**Kênh kiểm duyệt:** {}",
    );
    vi.insert(TranslationKey::SettingsTimezone, "**Múi giờ:** {}");
    vi.insert(TranslationKey::SettingsNotConfigured, "Chưa thiết lập");
    vi.insert(TranslationKey::PrefixChanged, "Đã đổi prefix thành `{}`");
    vi.insert(
        TranslationKey::PrefixInvalidLength,
        "Prefix phải có từ 1 đến {} ký tự.",
    );
    vi.insert(TranslationKey::TimezoneSet, "Đã đặt múi giờ thành `{}`.");
    vi.insert(TranslationKey::TimezoneCurrent, "**Múi giờ:** `{}`");
    vi.insert(
        TranslationKey::TimezoneNotConfigured,
        "Múi giờ chưa được thiết lập.",
    );
    vi.insert(TranslationKey::TimezoneCleared, "Đã xóa múi giờ.");
    vi.insert(
        TranslationKey::TimezoneInvalid,
        "Hãy dùng múi giờ IANA hợp lệ, ví dụ `Asia/Bangkok`.",
    );
    vi.insert(TranslationKey::OnboardingMessage, "Cảm ơn bạn đã cài đặt bot! Hãy bắt đầu với `/settings`, rồi chọn ngôn ngữ và múi giờ. Các tự động hóa đều là tùy chọn.");
    vi.insert(TranslationKey::DonationSaved, "Đã lưu thông tin ủng hộ.");
    vi.insert(TranslationKey::DonationCleared, "Đã xóa thông tin ủng hộ.");
    vi.insert(
        TranslationKey::DonationInvalidUpdate,
        "Hãy cung cấp tin nhắn, URL HTTPS hoặc ảnh QR PNG/JPEG.",
    );
    vi.insert(
        TranslationKey::DonationInvalidUrl,
        "URL ủng hộ phải dùng HTTPS.",
    );
    vi.insert(
        TranslationKey::DonationInvalidImage,
        "Ảnh QR phải là PNG hoặc JPEG và không vượt quá giới hạn tệp đính kèm.",
    );
    vi.insert(
        TranslationKey::DonateNotConfigured,
        "Thông tin ủng hộ chưa được thiết lập.",
    );
    vi.insert(TranslationKey::PresenceTitle, "**Quản Lý Trạng Thái Bot**");
    vi.insert(TranslationKey::PresenceHelp, "Sử dụng lệnh con để quản lý trạng thái bot:\n├ `/presence status` — Đặt trạng thái trực tuyến\n├ `/presence activity` — Đặt Rich Presence\n└ `/presence clear` — Xóa hoạt động");
    vi.insert(
        TranslationKey::PresenceStatusTitle,
        "**Đã Cập Nhật Trạng Thái**",
    );
    vi.insert(
        TranslationKey::PresenceStatusSet,
        "Trạng thái bot đã đặt thành **{}**",
    );
    vi.insert(
        TranslationKey::PresenceStatusSetDuration,
        "Trạng thái bot đã đặt thành **{}** trong **{} phút**",
    );
    vi.insert(
        TranslationKey::PresenceActivityTitle,
        "**Đã Cập Nhật Hoạt Động**",
    );
    vi.insert(
        TranslationKey::PresenceActivitySet,
        "Hoạt động đã đặt thành **{} {}**\nTrạng thái: **{}**",
    );
    vi.insert(
        TranslationKey::PresenceActivitySetDuration,
        "Hoạt động đã đặt thành **{} {}**\nTrạng thái: **{}**\nTự động hoàn lại sau **{} phút**",
    );
    vi.insert(
        TranslationKey::PresenceActivityCleared,
        "Đã xóa hoạt động và đặt lại trạng thái thành **Trực tuyến**",
    );
    vi.insert(
        TranslationKey::PresenceOwnerOnly,
        "Chỉ chủ sở hữu bot mới có thể sử dụng lệnh này.",
    );
    vi.insert(
        TranslationKey::PresenceDurationRange,
        "Thời lượng phải từ 0 đến {} phút.",
    );
    vi.insert(
        TranslationKey::PresencePersistent,
        "Đã lưu lâu dài — sẽ khôi phục khi bot khởi động lại",
    );
    vi.insert(TranslationKey::VoiceConnected, "Đã kết nối tới <#{}>");
    vi.insert(
        TranslationKey::VoiceDisconnected,
        "Đã ngắt kết nối khỏi kênh thoại.",
    );
    vi.insert(
        TranslationKey::VoiceNotInChannel,
        "Bạn không ở trong kênh thoại nào.",
    );
    vi.insert(
        TranslationKey::VoiceNotConnected,
        "Bot không ở trong kênh thoại nào.",
    );
    vi.insert(
        TranslationKey::VoiceAlreadyConnected,
        "Bot đã ở trong một kênh thoại. Sử dụng `/disconnect` trước.",
    );
    vi.insert(
        TranslationKey::VoiceJoinFailed,
        "Không thể tham gia kênh thoại.",
    );
    vi.insert(
        TranslationKey::VoiceKicked,
        "Bot đã rời kênh thoại. Dùng `/connect` để kết nối lại.",
    );
    vi.insert(
        TranslationKey::ErrorNotInGuild,
        "Lệnh này chỉ có thể sử dụng trong máy chủ.",
    );
    vi.insert(
        TranslationKey::ErrorNoPermission,
        "Bạn không có quyền sử dụng lệnh này.",
    );
    vi.insert(TranslationKey::ErrorGeneric, "Đã xảy ra lỗi nội bộ.");
    vi.insert(
        TranslationKey::ErrorCooldown,
        "Vui lòng đợi {} giây trước khi dùng lại lệnh này.",
    );
    translations.insert(Language::Vietnamese, vi);

    // Japanese translations
    let mut ja = HashMap::new();
    ja.insert(TranslationKey::PingPong, "ポン！");
    ja.insert(TranslationKey::PingLatency, "ポン！レイテンシ：**{}ms**");
    ja.insert(TranslationKey::BotInfoTitle, "**ボット情報**");
    ja.insert(
        TranslationKey::BotInfoUptime,
        "**稼働時間：** {}時間 {}分 {}秒",
    );
    ja.insert(TranslationKey::BotInfoServers, "**サーバー数：** {}");
    ja.insert(
        TranslationKey::BotInfoLanguage,
        "**プログラミング言語：** Rust",
    );
    ja.insert(
        TranslationKey::BotInfoFramework,
        "**フレームワーク：** Poise + Serenity",
    );
    ja.insert(TranslationKey::BotInfoDonate, "寄付情報：`/donate`");
    ja.insert(TranslationKey::ServerInfoTitle, "**サーバー情報**");
    ja.insert(TranslationKey::ServerInfoName, "**名前：** {}");
    ja.insert(TranslationKey::ServerInfoMembers, "**メンバー数：** {}");
    ja.insert(TranslationKey::ServerInfoChannels, "**チャンネル数：** {}");
    ja.insert(TranslationKey::ServerInfoRoles, "**ロール数：** {}");
    ja.insert(TranslationKey::ServerInfoCreated, "**作成日：** <t:{}:R>");
    ja.insert(
        TranslationKey::MessageLogEnabled,
        "メッセージログが有効になりました。ログチャンネル：<#{}>",
    );
    ja.insert(
        TranslationKey::MessageLogDisabled,
        "メッセージログを無効にしました。",
    );
    ja.insert(
        TranslationKey::MessageLogNotSetup,
        "メッセージログは設定されていません。",
    );
    ja.insert(
        TranslationKey::MessageLogStatusTitle,
        "**メッセージログのステータス**",
    );
    ja.insert(TranslationKey::MessageLogStatus, "**ステータス：**");
    ja.insert(TranslationKey::MessageLogStatusEnabled, "有効");
    ja.insert(TranslationKey::MessageLogStatusDisabled, "無効");
    ja.insert(
        TranslationKey::MessageLogChannel,
        "**ログチャンネル：** <#{}>",
    );
    ja.insert(
        TranslationKey::MessageLogUseEnable,
        "メッセージログは設定されていません。`/messagelog enable`で有効にしてください。",
    );
    ja.insert(TranslationKey::MessageLogHealth, "**状態：** {}");
    ja.insert(TranslationKey::MessageLogHealthDisabled, "無効");
    ja.insert(TranslationKey::MessageLogHealthHealthy, "正常");
    ja.insert(TranslationKey::MessageLogHealthDegraded, "制限あり");
    ja.insert(TranslationKey::MessageLogDegradedWarning, "Message Contentへのアクセスが無効なため、メッセージログは制限状態です。メタデータは記録されますが、本文や添付ファイルを取得できない場合があります。");
    ja.insert(TranslationKey::MessageDeleted, "メッセージが削除されました");
    ja.insert(
        TranslationKey::MessageEditedTitle,
        "メッセージが編集されました",
    );
    ja.insert(TranslationKey::MessageBulkDeleteTitle, "一括メッセージ削除");
    ja.insert(TranslationKey::MessageAuthor, "**作成者：** <@{}>");
    ja.insert(TranslationKey::MessageChannel, "**チャンネル：** <#{}>");
    ja.insert(TranslationKey::MessageContent, "**内容：**");
    ja.insert(TranslationKey::MessageBefore, "編集前");
    ja.insert(TranslationKey::MessageAfter, "編集後");
    ja.insert(TranslationKey::MessageJumpTo, "[メッセージへジャンプ]({})");
    ja.insert(TranslationKey::MessageMediaOnly, "*[メディアのみ]*");
    ja.insert(
        TranslationKey::MessageTotalDeleted,
        "**削除総数：** {}件のメッセージ",
    );
    ja.insert(
        TranslationKey::MessageCached,
        "**キャッシュ：** {}件のメッセージ（{}ユーザー、{}ボット）",
    );
    ja.insert(TranslationKey::MessageUser, "ユーザー");
    ja.insert(TranslationKey::MessageBot, "ボット");
    ja.insert(
        TranslationKey::MessageDeletedMessages,
        "削除されたメッセージ",
    );
    ja.insert(
        TranslationKey::MessagePurged,
        "{}件のメッセージを削除しました",
    );
    ja.insert(TranslationKey::MessageAuthorLabel, "作成者");
    ja.insert(TranslationKey::MessageChannelLabel, "チャンネル");
    ja.insert(TranslationKey::MessageId, "ID");
    ja.insert(TranslationKey::MessageIdValue, "メッセージID：{}");
    ja.insert(TranslationKey::MessageDeletedAt, "削除日時");
    ja.insert(TranslationKey::MessageUnknownTimestamp, "時刻不明");
    ja.insert(
        TranslationKey::MessageNoCached,
        "*表示できるキャッシュ済みメッセージはありません*",
    );
    ja.insert(
        TranslationKey::LanguageChanged,
        "言語を**{}**に変更しました",
    );
    ja.insert(TranslationKey::LanguageChangedTo, "言語を変更しました");
    ja.insert(TranslationKey::LanguageCurrent, "**現在の言語：** {}");
    ja.insert(
        TranslationKey::LanguageAvailable,
        "**利用可能：** English (en), Tiếng Việt (vi), 日本語 (ja)",
    );
    ja.insert(
        TranslationKey::LanguageInvalid,
        "無効な言語です。利用可能：`en`、`vi`、`ja`。",
    );
    ja.insert(TranslationKey::ModerationNoReason, "理由なし");
    ja.insert(
        TranslationKey::ModerationKicked,
        "**{}**をキックしました\n理由：```{}```",
    );
    ja.insert(TranslationKey::ModerationKickReason, "理由");
    ja.insert(
        TranslationKey::ModerationBanned,
        "**{}**をBANしました\n理由：```{}```",
    );
    ja.insert(TranslationKey::ModerationBanReason, "理由");
    ja.insert(
        TranslationKey::ModerationPurged,
        "**{}**件のメッセージを削除しました。",
    );
    ja.insert(
        TranslationKey::ModerationInvalidArgument,
        "無効なパラメータ：{}",
    );
    ja.insert(
        TranslationKey::ModerationBotMissingPermissions,
        "Botの権限が不足しています：{}",
    );
    ja.insert(
        TranslationKey::ModerationUserMissingPermissions,
        "権限が不足しています：{}",
    );
    ja.insert(
        TranslationKey::ModerationCannotTargetSelf,
        "自分自身またはBotをモデレートすることはできません。",
    );
    ja.insert(
        TranslationKey::ModerationUserHierarchy,
        "Discordのロール階層により、このメンバーをモデレートできません。",
    );
    ja.insert(
        TranslationKey::ModerationBotHierarchy,
        "Botのロールを対象メンバーの最上位ロールより上に移動してください。",
    );
    ja.insert(
        TranslationKey::ModerationDeleteDaysRange,
        "メッセージ削除日数は0から{}日の範囲で指定してください。",
    );
    ja.insert(
        TranslationKey::ModerationPurgeRange,
        "メッセージ数は1から{}の範囲で指定してください。",
    );
    ja.insert(
        TranslationKey::ModerationChannelSet,
        "モデレーションチャンネルを<#{}>に設定しました。",
    );
    ja.insert(
        TranslationKey::ModerationChannelCurrent,
        "**モデレーションチャンネル：** <#{}>",
    );
    ja.insert(
        TranslationKey::ModerationChannelNotConfigured,
        "モデレーションチャンネルは未設定です。",
    );
    ja.insert(
        TranslationKey::ModerationChannelCleared,
        "モデレーションチャンネルをクリアしました。",
    );
    ja.insert(
        TranslationKey::ModerationChannelInvalid,
        "このサーバーのテキストチャンネルを選択してください。",
    );
    ja.insert(TranslationKey::ModerationReasonRequired, "理由は必須です。");
    ja.insert(
        TranslationKey::ModerationInvalidEvidence,
        "証拠にはこのサーバー内のDiscordメッセージリンクを指定してください。",
    );
    ja.insert(
        TranslationKey::ModerationTimeoutRange,
        "タイムアウトは1分から40,320分の範囲で指定してください。",
    );
    ja.insert(TranslationKey::ModerationActionWarn, "警告");
    ja.insert(TranslationKey::ModerationActionKick, "キック");
    ja.insert(TranslationKey::ModerationActionBan, "BAN");
    ja.insert(TranslationKey::ModerationActionTimeout, "タイムアウト");
    ja.insert(
        TranslationKey::ModerationCaseSummary,
        "ケース#{} — {}\n対象：<@{}>\nモデレーター：<@{}>\n理由：{}",
    );
    ja.insert(TranslationKey::ModerationActionCaseFailed, "Discordでの操作は成功しましたが、モデレーションケースを記録できませんでした。Bot所有者に連絡してください。");
    ja.insert(TranslationKey::ModerationActionUnknown, "不明な操作");
    ja.insert(
        TranslationKey::ModerationCaseNotFound,
        "モデレーションケースが見つかりません。",
    );
    ja.insert(
        TranslationKey::ModerationCaseListEmpty,
        "このページにモデレーションケースはありません。",
    );
    ja.insert(
        TranslationKey::ModerationCaseList,
        "**モデレーションケース — ページ{}**\n{}",
    );
    ja.insert(
        TranslationKey::ModerationCaseListRow,
        "#{} — {} — <@{}> — {}",
    );
    ja.insert(TranslationKey::ModerationCaseView, "**ケース#{} — {}**\n対象：<@{}>\nモデレーター：<@{}>\n理由：{}\n作成日時：{}\n証拠：{}\n状態：{}");
    ja.insert(TranslationKey::ModerationCaseStatusActive, "有効");
    ja.insert(TranslationKey::ModerationCaseStatusVoided, "無効");
    ja.insert(
        TranslationKey::ModerationCaseVoidDetails,
        "\n無効化したユーザー：<@{}>\n無効化の理由：{}\n無効化日時：{}",
    );
    ja.insert(
        TranslationKey::ModerationCaseAlreadyVoided,
        "モデレーションケースは既に無効です。",
    );
    ja.insert(
        TranslationKey::ModerationCaseVoided,
        "モデレーションケース#{}を無効にしました。",
    );
    ja.insert(
        TranslationKey::AutoModObserverEnabled,
        "Discord AutoModの監視を有効にしました。",
    );
    ja.insert(
        TranslationKey::AutoModObserverDisabled,
        "Discord AutoModの監視を無効にしました。",
    );
    ja.insert(
        TranslationKey::AutoModObserverNeedsChannel,
        "先にモデレーションチャンネルを設定してください。",
    );
    ja.insert(
        TranslationKey::AutoModObserverNeedsPermission,
        "Discord AutoModイベントを受信するには、Botに「サーバーを管理」権限が必要です。",
    );
    ja.insert(
        TranslationKey::AutoModExecution,
        "Discord AutoMod実行\nルール：{}\nメンバー：<@{}>\n操作：{}\nメッセージ：{}",
    );
    ja.insert(TranslationKey::AutoModSuggestion, "確認の提案：メンバー<@{}>が7日間にAutoModルール{}を3回以上発動しました。Botが自動的に処分することはありません。");
    ja.insert(
        TranslationKey::AutoModSuggestionHandled,
        "AutoMod提案を対応済みにしました。",
    );
    ja.insert(
        TranslationKey::AutoModSuggestionNotFound,
        "そのメンバーとルールに一致する未対応のAutoMod提案はありません。",
    );
    ja.insert(
        TranslationKey::AutoModSuggestionInvalidRule,
        "有効なDiscord AutoModルールIDを指定してください。",
    );
    ja.insert(
        TranslationKey::AutoModSuggestionDelivered,
        "AutoMod提案を送信しました。",
    );
    ja.insert(
        TranslationKey::AutoModSuggestionDeliveryFailed,
        "AutoMod提案は未対応のままですが、送信に失敗しました。",
    );
    ja.insert(TranslationKey::SettingsTitle, "**サーバー設定**");
    ja.insert(TranslationKey::SettingsPrefix, "**プレフィックス：** `{}`");
    ja.insert(
        TranslationKey::SettingsLogChannel,
        "**ログチャンネル：** {}",
    );
    ja.insert(
        TranslationKey::SettingsModerationChannel,
        "**モデレーションチャンネル：** {}",
    );
    ja.insert(TranslationKey::SettingsTimezone, "**タイムゾーン：** {}");
    ja.insert(TranslationKey::SettingsNotConfigured, "未設定");
    ja.insert(
        TranslationKey::PrefixChanged,
        "プレフィックスを`{}`に変更しました",
    );
    ja.insert(
        TranslationKey::PrefixInvalidLength,
        "プレフィックスは1から{}文字で指定してください。",
    );
    ja.insert(
        TranslationKey::TimezoneSet,
        "タイムゾーンを`{}`に設定しました。",
    );
    ja.insert(TranslationKey::TimezoneCurrent, "**タイムゾーン：** `{}`");
    ja.insert(
        TranslationKey::TimezoneNotConfigured,
        "タイムゾーンは未設定です。",
    );
    ja.insert(
        TranslationKey::TimezoneCleared,
        "タイムゾーンをクリアしました。",
    );
    ja.insert(
        TranslationKey::TimezoneInvalid,
        "`Asia/Bangkok`などの有効なIANAタイムゾーンを指定してください。",
    );
    ja.insert(TranslationKey::OnboardingMessage, "導入ありがとうございます！まず`/settings`を開き、言語とタイムゾーンを設定してください。自動化はすべてオプトインです。");
    ja.insert(TranslationKey::DonationSaved, "寄付情報を保存しました。");
    ja.insert(
        TranslationKey::DonationCleared,
        "寄付情報をクリアしました。",
    );
    ja.insert(
        TranslationKey::DonationInvalidUpdate,
        "メッセージ、HTTPS URL、またはPNG/JPEGのQR画像を指定してください。",
    );
    ja.insert(
        TranslationKey::DonationInvalidUrl,
        "寄付URLはHTTPSである必要があります。",
    );
    ja.insert(
        TranslationKey::DonationInvalidImage,
        "QR画像は添付ファイル上限内のPNGまたはJPEGである必要があります。",
    );
    ja.insert(
        TranslationKey::DonateNotConfigured,
        "寄付情報は未設定です。",
    );
    ja.insert(TranslationKey::PresenceTitle, "**ボットプレゼンス管理**");
    ja.insert(TranslationKey::PresenceHelp, "サブコマンドでボットのプレゼンスを管理します：\n├ `/presence status` — オンラインステータスを設定\n├ `/presence activity` — リッチプレゼンスを設定\n└ `/presence clear` — アクティビティをクリア");
    ja.insert(TranslationKey::PresenceStatusTitle, "**ステータス更新**");
    ja.insert(
        TranslationKey::PresenceStatusSet,
        "ボットのステータスを**{}**に設定しました",
    );
    ja.insert(
        TranslationKey::PresenceStatusSetDuration,
        "ボットのステータスを**{}**に**{}分間**設定しました",
    );
    ja.insert(
        TranslationKey::PresenceActivityTitle,
        "**アクティビティ更新**",
    );
    ja.insert(
        TranslationKey::PresenceActivitySet,
        "アクティビティを**{} {}**に設定しました\nステータス: **{}**",
    );
    ja.insert(
        TranslationKey::PresenceActivitySetDuration,
        "アクティビティを**{} {}**に設定しました\nステータス: **{}**\n**{}分後**に元に戻ります",
    );
    ja.insert(
        TranslationKey::PresenceActivityCleared,
        "アクティビティをクリアし、ステータスを**オンライン**にリセットしました",
    );
    ja.insert(
        TranslationKey::PresenceOwnerOnly,
        "このコマンドはボットのオーナーのみ使用できます。",
    );
    ja.insert(
        TranslationKey::PresenceDurationRange,
        "期間は0から{}分の範囲で指定してください。",
    );
    ja.insert(
        TranslationKey::PresencePersistent,
        "永続的に保存しました — Bot再起動時に復元されます",
    );
    ja.insert(TranslationKey::VoiceConnected, "<#{}>に接続しました");
    ja.insert(
        TranslationKey::VoiceDisconnected,
        "ボイスチャンネルから切断しました。",
    );
    ja.insert(
        TranslationKey::VoiceNotInChannel,
        "ボイスチャンネルに参加していません。",
    );
    ja.insert(
        TranslationKey::VoiceNotConnected,
        "ボットはボイスチャンネルに接続していません。",
    );
    ja.insert(
        TranslationKey::VoiceAlreadyConnected,
        "ボットは既にボイスチャンネルに接続しています。先に`/disconnect`を使用してください。",
    );
    ja.insert(
        TranslationKey::VoiceJoinFailed,
        "ボイスチャンネルへの参加に失敗しました。",
    );
    ja.insert(
        TranslationKey::VoiceKicked,
        "ボットはボイスチャンネルから退出しました。再参加するには`/connect`を使用してください。",
    );
    ja.insert(
        TranslationKey::ErrorNotInGuild,
        "このコマンドはサーバー内でのみ使用できます。",
    );
    ja.insert(
        TranslationKey::ErrorNoPermission,
        "このコマンドを使用する権限がありません。",
    );
    ja.insert(TranslationKey::ErrorGeneric, "内部エラーが発生しました。");
    ja.insert(
        TranslationKey::ErrorCooldown,
        "このコマンドを再度使用するには{}秒お待ちください。",
    );
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

    for arg in args.iter() {
        result = result.replacen("{}", &arg.to_string(), 1);
    }

    result
}

/// Get language for a guild from database
pub async fn get_guild_language(
    db_pool: &sqlx::SqlitePool,
    guild_id: serenity_prelude::GuildId,
    default: Language,
) -> Language {
    let lang_str =
        sqlx::query_scalar::<_, String>("SELECT language FROM guild_language WHERE guild_id = ?")
            .bind(guild_id.to_string())
            .fetch_optional(db_pool)
            .await
            .ok()
            .flatten();

    match lang_str {
        Some(s) => Language::try_parse(&s).unwrap_or(default),
        None => default,
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
         ON CONFLICT(guild_id) DO UPDATE SET language = excluded.language",
    )
    .bind(guild_id.to_string())
    .bind(language.to_str())
    .execute(db_pool)
    .await?;

    Ok(())
}
