# Dummy Bot

Dummy Bot supports multiple independent Discord communities with moderation, social, and gaming-oriented features.

## Language

**Guild**:
An independent Discord community with its own members, permissions, configuration, and data.
_Avoid_: Server

**Public Guild Installation**:
Any Guild Administrator may install the bot without approval from the Bot Owner. A Guild is isolated after installation but is not allowlisted beforehand.
_Avoid_: Invite-only Installation, Approved Guild

**Onboarding Message**:
A one-time setup guide sent to the installer when they can be identified without extra permissions, otherwise to an available Guild system channel.
_Avoid_: Announcement, Direct-message Campaign

**Guild Administrator**:
A Guild member whose native Discord permissions authorize configuration or moderation actions. Bot-specific manager roles do not grant authority.
_Avoid_: Bot Manager, Admin Role

**Bot Owner**:
A globally authorized operator who controls bot-wide settings such as presence and donation information, but does not replace Guild Administrators in daily Guild management.
_Avoid_: Guild Administrator

**Donation Information**:
An optional global link, message, and QR image configured by the Bot Owner and shown only through `/donate` or unobtrusively in `/botinfo`.
_Avoid_: Advertisement, Guild Donation

**Guild Language**:
A Guild's selected language for every bot interaction; supported values are English, Vietnamese, and Japanese, and new features ship with all three.
_Avoid_: Word Set Language

**Guild Time Zone**:
The Guild's configured IANA time zone used for local day boundaries and the 05:00 expiry of Ad-hoc Game Sessions. Game Role Automation cannot be enabled without it.
_Avoid_: UTC Offset, Bot Time Zone

**Interactive Command**:
A capability that acts only when a member explicitly invokes it and is available by default.
_Avoid_: Always-on Feature

**Automation**:
A capability that observes events or initiates actions without a member invoking it each time and must be enabled per Guild.
_Avoid_: Background Command

**Game Session**:
A Community Activity where Guild members plan to play a named game together, optionally with limited places.
_Avoid_: LFG, Game Event

**Game Role**:
A Discord role mapped to one game by a Guild Administrator whose mention starts an Ad-hoc Game Session. The mention itself never grants Session Credit or Play Time.
_Avoid_: Activity Reward Role

**Ad-hoc Game Session**:
An unscheduled, hostless Game Session opened by mentioning a configured Game Role and closed at the next 05:00 in the Guild Time Zone. Only one is open per Game Role; later mentions reuse it without extending its deadline, and any member who earns Verified Game Attendance in the Voice Pool becomes a Participant.
_Avoid_: Scheduled Game Session, Role Ping

**Voice Pool**:
One or more voice channels configured for a Game Role and observed for its Ad-hoc Game Sessions.
_Avoid_: Selected Voice Channel

**Primary Voice Channel**:
The channel in a Game Role's Voice Pool used as the `VOICE` Discord Scheduled Event location. Attendance may still qualify in every channel in the Voice Pool.
_Avoid_: Voice Pool, Only Attendance Channel

**Game Channel**:
The Guild channel where configured Game Role mentions may open Ad-hoc Game Sessions. Game Role mentions elsewhere do not trigger the bot.
_Avoid_: Voice Channel, Moderation Channel

**Game Integration**:
A connection to an external game's service for retrieving or presenting game-specific data.
_Avoid_: Game Plugin

**Tracker Profile Link**:
A tracker.gg URL voluntarily attached once to a Discord user's bot profile for navigation only and shared across Guilds rather than configured separately in each Guild. Members who share a Guild with its owner may view it. The bot neither verifies the profile nor imports data from it.
_Avoid_: Linked Riot Account, Tracker Integration

**Linked Riot Account**:
A Riot account that a member has explicitly connected through Riot Sign On, permitting policy-compliant access to that member's VALORANT data.
_Avoid_: Tracker Profile Link, Riot ID Text

**Guild Profile Visibility**:
A member's per-Guild consent for bot commands to display data from their Linked Riot Account. Linking alone leaves the profile hidden in every Guild.
_Avoid_: Account Link

**Guild VALORANT Leaderboard**:
A comparison using Riot's official measures and containing only members who enabled Guild Profile Visibility. It never calculates an alternative skill rating.
_Avoid_: Custom MMR, Global Leaderboard

**Companion Website**:
The minimal web surface for bot information, legal policies, Riot Sign On, account unlinking, and data-deletion requests. Community and game interactions remain in Discord.
_Avoid_: Web Dashboard

**Mini-game**:
A game whose play happens through interactions with the bot inside Discord.
_Avoid_: Game Integration

**Word Puzzle Session**:
A competitive Mini-game where Participants privately attempt the same hidden word up to six times before a deadline. Results remain hidden until a participant finishes or time expires.
_Avoid_: Wordle

**Word Set**:
A curated collection defining the language, allowed guesses, and possible answers for Word Puzzle Sessions. The initial Word Set uses five-letter English words.
_Avoid_: Dictionary

## Moderation

**Moderation Case**:
A per-Guild numbered record of a warning, kick, ban, or timeout that a Guild Administrator decided to apply to a member. Message purges and Discord AutoMod events are not cases.
_Avoid_: AutoMod Event

**Voided Case**:
A Moderation Case marked invalid while preserving its original record, the void reason, and the Guild Administrator who voided it.
_Avoid_: Deleted Case, Edited Case

**Discord AutoMod**:
Discord's native rules and enforcement for automatically detecting or blocking unwanted content; the bot does not duplicate this enforcement.
_Avoid_: Bot AutoMod

**Moderation Suggestion**:
A non-binding recommendation sent to Guild Administrators after repeated Discord AutoMod events involving the same member and rule. Only one remains open for that member and rule until a moderator handles it or Discord reports that exact rule was updated. It never changes rules or disciplines a member automatically.
_Avoid_: Automatic Punishment, Moderation Case

**Moderation Channel**:
A private Guild channel for Moderation Cases, Discord AutoMod notifications, and Moderation Suggestions.
_Avoid_: Message Log Channel

**Message Log Channel**:
A private Guild channel for records of edited or deleted messages, separate from moderation decisions and suggestions.
_Avoid_: Moderation Channel

**Degraded Message Log**:
A Message Log that records available metadata but cannot capture message content or attachments because Discord has not granted the bot Message Content access. Guild Administrators are explicitly notified of this state.
_Avoid_: Disabled Message Log

**Guild Data**:
Configuration, Moderation Cases, counters, and other bot-held records owned by one Guild. It is isolated from other Guilds and deleted when the bot is permanently removed from that Guild.
_Avoid_: Global Data

## Community

**Community Activity**:
A planned occasion for Guild members to gather, represented initially by a `VOICE` Discord Scheduled Event. The bot may attach domain-specific details that Discord does not provide.
_Avoid_: Bot Event, Calendar Entry

**Participant**:
A Guild member who explicitly joins a Community Activity through the bot. Subscribing to its Discord Scheduled Event does not reserve a place.
_Avoid_: Interested User, Subscriber

**Verified Game Attendance**:
A member's eligibility for Session Credit and Play Time, established by 30 cumulative minutes in a session voice channel while an Activity Beacon for the matching game is active. Manual check-in may provide the game signal but never replaces the individual voice-time requirement.
_Avoid_: Voice Time, Event Subscription

**Activity Beacon**:
Evidence that at least one non-bot member in a voice channel is sharing the session's game or has manually checked in, matched first by Discord application ID and then by exact, case-insensitive name when Presence is available. It lets every member in that channel accumulate their own attendance time and stops when the last matching or checked-in member leaves or ends the signal.
_Avoid_: Individual Activity, Voice Presence

**Host**:
A member who creates a scheduled Community Activity through the bot and may edit or cancel it. Members with Discord `MANAGE_EVENTS` may manage any Community Activity; Ad-hoc Game Sessions are hostless.
_Avoid_: Guild Administrator, Event Creator

**Waitlisted Member**:
A member who tried to join a capacity-limited Community Activity after all places were reserved. Members are promoted automatically in join order when a place opens.
_Avoid_: Participant

**Activity Level**:
A playful level derived from a member's cumulative Play Time in a Guild, where Level `n` requires `n(n+1)/2` total hours. A member may opt out and erase it; it never represents trust, reputation, or authority and cannot grant moderation permissions.
_Avoid_: Reputation, Trust Score

**Session Credit**:
A count increased once for each non-overlapping Game Session with Verified Game Attendance and once per local calendar day for a valid Word Puzzle completion, resetting at 00:00 in the Guild Time Zone. A Scheduled Game Session takes precedence over an overlapping Ad-hoc Game Session; the count is displayed separately and does not determine Activity Level.
_Avoid_: XP, Reputation Point

**Play Time**:
Cumulative verified minutes spent playing games during Game Sessions in a Guild. A session under 30 minutes contributes nothing; once qualified, all valid minutes count exactly once even across overlapping sessions, while Word Puzzle time remains excluded.
_Avoid_: Voice Time, Session Credit

**Activity Profile**:
A Guild-visible summary of one participating member's Activity Level, Session Credit, and Play Time, including aggregate breakdowns by game but no voice-event history. It is scoped to one Guild and disappears when the member opts out.
_Avoid_: Global Profile, Riot Profile

**Activity Leaderboard**:
A per-Guild ranking ordered by Play Time, with equal minutes sharing a rank and Session Credit shown without breaking ties. It shows the leading members and the requesting member's own position.
_Avoid_: Reputation Ranking, Global Leaderboard

**Activity Reward Role**:
A non-moderation Discord role granted and revoked by the bot at a configured Activity Level. It may be bot-created or selected from existing Guild roles, but the bot tracks only grants it made and stops granting if the role gains moderation authority.
_Avoid_: Permission Role, Reputation Role

**Bot-owned Reward Role**:
An Activity Reward Role created by the bot, which the bot may delete when replaced. Existing Guild roles are never bot-owned and are never deleted by the bot.
_Avoid_: Existing Reward Role
