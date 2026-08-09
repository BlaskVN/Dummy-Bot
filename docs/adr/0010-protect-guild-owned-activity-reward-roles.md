# Protect Guild-owned Activity Reward Roles

Activity Reward Roles may be created by the bot or selected from existing Guild roles, but the bot records and removes only its own grants and deletes only Bot-owned Reward Roles. Roles with moderation or administrative authority are rejected, and later permission escalation stops new grants and alerts Guild Administrators. Changing a role or level threshold reconciles every bot-managed grant without touching manual grants, preventing an activity score from becoming an authority path.
