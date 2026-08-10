use std::path::{Path, PathBuf};

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

#[test]
fn published_map_accounts_for_every_prefix_registration_and_support_symbol() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let inventory = std::fs::read_to_string(root.join("docs/v2-slash-only-upgrade.md")).unwrap();
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut registered = files
        .into_iter()
        .filter(|path| {
            std::fs::read_to_string(path)
                .unwrap()
                .contains("prefix_command")
        })
        .map(|path| {
            path.strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>();
    registered.sort();
    let expected = [
        "src/commands/configuration/prefix.rs",
        "src/commands/configuration/settings.rs",
        "src/commands/general/botinfo.rs",
        "src/commands/general/donate.rs",
        "src/commands/general/ping.rs",
        "src/commands/general/serverinfo.rs",
        "src/commands/moderation/ban.rs",
        "src/commands/moderation/kick.rs",
        "src/commands/moderation/purge.rs",
        "src/commands/presence.rs",
        "src/commands/voice.rs",
    ];
    assert!(registered.is_empty() || registered == expected);
    for source in expected {
        assert!(inventory.contains(source));
    }
    for symbol in [
        "dynamic_prefix",
        "guild_prefix",
        "SettingsPrefix",
        "PrefixChanged",
        "PrefixInvalidLength",
        "DEFAULT_PREFIX",
        "PREFIX_MAX_CHARS",
        "guild_config",
        "/messagelog",
        "/language",
    ] {
        assert!(inventory.contains(symbol), "inventory missing {symbol}");
    }
}
