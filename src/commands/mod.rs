pub mod global;
pub mod guild;

use crate::{Data, Error};

/// Returns a vector of all registered commands.
///
/// Commands are organized by:
/// - **Channel type**: `guild` (server-only) / `global` (DM + server)
/// - **Permission level**: `everyone` / `moderator` / `admin` / `owner`
/// - **Feature**: individual command files
///
/// ```text
/// commands/
/// ├── guild/                          # Server-only commands
/// │   ├── everyone/                   # No special permissions
/// │   │   ├── serverinfo.rs
/// │   │   └── music.rs
/// │   ├── moderator/                  # KICK/BAN/MANAGE_MESSAGES
/// │   │   ├── kick.rs
/// │   │   ├── ban.rs
/// │   │   └── purge.rs
/// │   └── admin/                      # MANAGE_GUILD
/// │       ├── settings.rs
/// │       ├── setprefix.rs
/// │       ├── logging.rs
/// │       └── language.rs
/// └── global/                         # DM + Server commands
///     ├── everyone/
///     │   ├── ping.rs
///     │   └── botinfo.rs
///     └── owner/                      # Bot owner only
///         └── presence.rs
/// ```
pub fn all() -> Vec<poise::Command<Data, Error>> {
    let mut commands = Vec::new();
    commands.extend(guild::all());
    commands.extend(global::all());
    commands
}
