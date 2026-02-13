pub mod presence;

use crate::{Data, Error};

/// Returns all global commands requiring bot owner permissions.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        presence::presence(),
    ]
}
