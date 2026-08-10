# v2.0 slash-only upgrade

Dummy Bot v2.0 removes text-prefix command dispatch. Discord application
commands remain available under the same slash names, permissions, cooldowns,
autocomplete, buttons, and modal behavior. Text such as `!ping` is ordinary
message content after upgrading and receives no command response.

## Legacy command map

| Pre-v2 prefix surface | v2 slash replacement | Source before removal |
|---|---|---|
| `ping` | `/ping` | `src/commands/general/ping.rs` |
| `botinfo` | `/botinfo` | `src/commands/general/botinfo.rs` |
| `serverinfo` | `/serverinfo` | `src/commands/general/serverinfo.rs` |
| `donate` | `/donate` | `src/commands/general/donate.rs` |
| `ban` | `/ban` | `src/commands/moderation/ban.rs` |
| `kick` | `/kick` | `src/commands/moderation/kick.rs` |
| `purge` | `/purge` | `src/commands/moderation/purge.rs` |
| `settings` | `/settings` | `src/commands/configuration/settings.rs` |
| `setprefix` | Removed; no prefix is needed | `src/commands/configuration/prefix.rs` |
| `presence` | `/presence` | `src/commands/presence.rs` |
| `presence status` | `/presence status` | `src/commands/presence.rs` |
| `presence activity` | `/presence activity` | `src/commands/presence.rs` |
| `presence clear` | `/presence clear` | `src/commands/presence.rs` |
| `connect` | `/connect` | `src/commands/voice.rs` |
| `disconnect` | `/disconnect` | `src/commands/voice.rs` |

Owner presence management remains available through `/presence` and its slash
subcommands.

Already slash-only surfaces include `/messagelog`, `/language`, `/timezone`,
`/moderation-channel`, `/automod-observer`, `/game-config`, `/case`, `/warn`,
`/timeout`, `/donation`, `/activity`, and `/word-puzzle`. They do not need a
compatibility change.

## Prefix-only implementation removed in v2.0

- `src/app.rs`: `PrefixFrameworkOptions` and `dynamic_prefix`.
- `src/database.rs`: `guild_prefix` and runtime reads of `guild_config.prefix`.
- `src/commands/configuration/prefix.rs`: `/setprefix` and its prefix form.
- `src/commands/configuration/settings.rs`: the prefix query and display row.
- `src/i18n.rs`: `SettingsPrefix`, `PrefixChanged`, and
  `PrefixInvalidLength`.
- `src/config.rs`, `config.env`: `DEFAULT_PREFIX` and `PREFIX_MAX_CHARS`.
- `README.md`, `README.vi.md`: prefix feature, layout, configuration, and
  command references.
- SQLite `guild_config`: the table only stores obsolete prefix-era fields and
  is dropped by a forward migration. Other Guild settings live in independent
  tables and are preserved.

There is no prefix-only parser outside Poise's dynamic prefix dispatch and no
deprecation telemetry or compatibility framework is added.

## Upgrade and rollback

The removal version is v2.0.0. Deploy the new binary and run its forward SQLite
migrations before expecting slash-only behavior. Discord may take time to show
newly registered application commands after deployment.

The migration discards saved per-Guild prefix values. Rolling the database back
to a pre-v2 binary therefore requires a pre-upgrade database backup (or manual
recreation of the old table); switching only the executable is not a supported
rollback. v2 does not retain a hidden compatibility prefix.
