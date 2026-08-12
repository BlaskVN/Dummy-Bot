use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("format_user_mention", |user_id: &str| -> String {
        format!("<@{}>", user_id)
    });

    engine.register_fn("format_role_mention", |role_id: &str| -> String {
        format!("<@&{}>", role_id)
    });

    engine.register_fn("format_channel_mention", |channel_id: &str| -> String {
        format!("<#{}>", channel_id)
    });
}
