# Use slash commands while retaining optional message content

New commands use Discord interactions and legacy prefix commands will be deprecated, but the bot still requests the privileged Message Content intent for Guilds that opt into Message Log. If Discord does not grant access, the bot continues with a Degraded Message Log, sends one warning to the configured channel, and exposes the degraded state in settings instead of silently dropping content.
