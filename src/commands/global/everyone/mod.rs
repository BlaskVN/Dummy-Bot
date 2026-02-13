pub mod botinfo;
pub mod ping;

use crate::{Data, Error};

/// Returns all global commands (DM + Guild) available to everyone.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        ping::ping(),
        botinfo::botinfo(),
    ]
}
