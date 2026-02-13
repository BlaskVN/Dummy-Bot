pub mod admin;
pub mod everyone;
pub mod moderator;

use crate::{Data, Error};

/// Returns all guild-only commands across all permission levels.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    let mut commands = Vec::new();
    commands.extend(everyone::all());
    commands.extend(moderator::all());
    commands.extend(admin::all());
    commands
}
