pub mod everyone;
pub mod owner;

use crate::{Data, Error};

/// Returns all global commands (DM + Guild) across all permission levels.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    let mut commands = Vec::new();
    commands.extend(everyone::all());
    commands.extend(owner::all());
    commands
}
