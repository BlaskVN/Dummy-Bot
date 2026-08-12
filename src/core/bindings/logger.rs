use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("log_info", |msg: &str| {
        tracing::info!(target: "rhai_script", "{}", msg);
    });

    engine.register_fn("log_warn", |msg: &str| {
        tracing::warn!(target: "rhai_script", "{}", msg);
    });

    engine.register_fn("log_error", |msg: &str| {
        tracing::error!(target: "rhai_script", "{}", msg);
    });
}
