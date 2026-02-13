pub mod language;
pub mod logging;
pub mod setprefix;
pub mod settings;

use crate::{Data, Error};

/// Returns all guild-only commands requiring admin (MANAGE_GUILD) permissions.
pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        settings::settings(),
        setprefix::setprefix(),
        logging::messagelog(),
        language::language(),
    ]
}
