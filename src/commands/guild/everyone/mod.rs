pub mod music;
pub mod serverinfo;

use crate::{Data, Error};

/// Returns all guild-only commands available to everyone.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        serverinfo::serverinfo(),
        music::play(),
        music::stop(),
    ]
}
