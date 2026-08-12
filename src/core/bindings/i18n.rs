use crate::i18n::{Language, TranslationKey};
use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("t", |key: &str, lang: &str| -> String {
        let language = Language::parse(lang);
        if let Some(trans_key) = TranslationKey::try_from_str(key) {
            crate::i18n::t(language, trans_key).to_string()
        } else {
            key.to_string()
        }
    });
}
