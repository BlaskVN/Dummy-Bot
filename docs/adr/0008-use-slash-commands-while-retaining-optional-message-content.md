# Use slash-only commands while retaining optional Message Content

As of v2.0, every command uses Discord interactions and no command parser reads
message content. Legacy prefix dispatch and its per-Guild configuration are
removed.

The deployment may still request the privileged Message Content intent solely
for Guilds that opt into edited/deleted Message Log content. When access is
disabled or denied, Message Log remains explicitly Degraded, records safe
metadata, sends one warning to its configured channel, and exposes health in
`/settings`. No compatibility prefix is retained.
