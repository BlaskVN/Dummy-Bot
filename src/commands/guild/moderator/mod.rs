pub mod ban;
pub mod kick;
pub mod purge;

use crate::{Data, Error};

/// Returns all guild-only commands requiring moderator permissions.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        kick::kick(),
        ban::ban(),
        purge::purge(),
    ]
}
