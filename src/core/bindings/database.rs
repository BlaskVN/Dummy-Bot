use rhai::{Dynamic, Engine, Map};

pub fn register(engine: &mut Engine) {
    // In-memory key-value store interface exposed to Rhai for transient script states
    engine.register_fn("db_parse_json", |json_str: &str| -> Dynamic {
        match serde_json::from_str::<Map>(json_str) {
            Ok(map) => Dynamic::from(map),
            Err(_) => Dynamic::UNIT,
        }
    });

    engine.register_fn("db_stringify_json", |map: Map| -> String {
        serde_json::to_string(&map).unwrap_or_default()
    });
}
