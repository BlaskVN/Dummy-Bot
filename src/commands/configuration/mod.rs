pub mod language;
pub mod logging;
pub mod moderation_channel;
pub mod prefix;
pub mod settings;
pub mod timezone;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        settings::settings(),
        prefix::setprefix(),
        logging::messagelog(),
        moderation_channel::moderation_channel(),
        language::language(),
        timezone::timezone(),
    ]
}
