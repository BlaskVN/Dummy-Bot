pub mod administration;
pub mod general;
pub mod language;
pub mod logging;
pub mod moderation;
pub mod music;
pub mod presence;


use crate::{Data, Error};

/// Returns a vector of all registered commands.
///
/// This function acts as the central registry — add new command modules here
/// and call their individual commands. `main.rs` only needs to call `commands::all()`.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        // General utilities
        general::ping(),
        general::botinfo(),
        general::serverinfo(),
        // Moderation
        moderation::kick(),
        moderation::ban(),
        moderation::purge(),
        // Administration
        administration::settings(),
        administration::setprefix(),
        // Message Logging
        logging::messagelog(),
        // Language
        language::language(),
        // Music (placeholder)
        music::play(),
        music::stop(),
        // Presence (owner only)
        presence::presence(),
    ]
}
