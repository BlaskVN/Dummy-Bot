pub mod database;
pub mod discord;
pub mod i18n;
pub mod logger;

use rhai::Engine;

pub fn register_all(engine: &mut Engine) {
    logger::register(engine);
    i18n::register(engine);
    database::register(engine);
    discord::register(engine);
}
