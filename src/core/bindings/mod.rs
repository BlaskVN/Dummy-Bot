pub mod logger;

use rhai::Engine;

pub fn register_all(engine: &mut Engine) {
    logger::register(engine);
}
