pub mod serverinfo;
pub mod voice;

use crate::{Data, Error};

/// Returns all guild-only commands available to everyone.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        serverinfo::serverinfo(),
        voice::voice_connect(),
        voice::voice_disconnect(),
    ]
}
