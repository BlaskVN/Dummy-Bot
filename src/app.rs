use crate::{Data, commands, config::Config, database, error, handlers};
use poise::serenity_prelude as serenity;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Notify, RwLock, Semaphore};

pub async fn run(config: Config) -> anyhow::Result<()> {
    let _ = rustls::crypto::ring::default_provider().install_default();
    tracing::info!("Starting Discord bot");

    let config = Arc::new(config);
    let db_pool = database::init_db(&config.database_url, &config.data_directory).await?;
    let owners = resolve_owners(&config).await;
    let attachment_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let setup_config = Arc::clone(&config);
    let setup_pool = db_pool.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            owners,
            on_error: |error| Box::pin(error::on_error(error)),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(handlers::dispatch(ctx, event, data))
            },
            pre_command: |ctx| {
                Box::pin(async move {
                    tracing::debug!(
                        command = ctx.command().name,
                        user = %ctx.author().name,
                        guild = ?ctx.guild_id(),
                        "Executing command"
                    );
                })
            },
            post_command: |ctx| {
                Box::pin(async move {
                    tracing::debug!(command = ctx.command().name, "Command completed");
                })
            },
            ..Default::default()
        })
        .setup(move |ctx, ready, framework| {
            Box::pin(async move {
                tracing::info!(
                    bot_name = %ready.user.name,
                    guild_count = ready.guilds.len(),
                    "Bot connected"
                );
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                commands::presence::restore_presence(ctx, &setup_pool).await;

                let rhai_manager = Arc::new(crate::core::RhaiManager::new(&setup_config.rhai_modules_directory));
                if let Err(err) = rhai_manager.load_all().await {
                    tracing::error!(error = %err, "Failed to load Rhai script modules");
                }

                let data = Data {
                    config: setup_config,
                    db_pool: setup_pool,
                    start_time: Instant::now(),
                    attachment_client,
                    attachment_downloads: Arc::new(Semaphore::new(2)),
                    voice_connections: Arc::new(RwLock::new(HashMap::new())),
                    game_session_creation: Arc::new(Mutex::new(())),
                    game_expiry_wakeup: Arc::new(Notify::new()),
                    automatic_beacons: Arc::new(RwLock::new(HashSet::new())),
                    manual_checkins: Arc::new(RwLock::new(HashSet::new())),
                    rhai_manager,
                };
                if let Err(error) =
                    crate::attendance::clear_stale_active_starts(&data.db_pool).await
                {
                    tracing::error!(%error, "Could not clear stale attendance starts");
                }
                if let Err(error) = crate::activity_aggregate::finalize_pending(
                    &data.db_pool,
                    chrono::Utc::now().timestamp(),
                )
                .await
                {
                    tracing::error!(%error, "Could not finalize pending activity aggregates");
                }
                commands::word_puzzle::reconcile_and_deliver_all(ctx, &data).await;
                handlers::message_log::reconcile_all_health(ctx, &data).await;
                handlers::community::reconcile_all(ctx, &data).await;
                handlers::rewards::reconcile_all(ctx, &data).await;
                handlers::game_session::spawn_expiry_worker(
                    ctx.clone(),
                    data.db_pool.clone(),
                    Arc::clone(&data.game_expiry_wakeup),
                );
                Ok(data)
            })
        })
        .build();

    let intents = gateway_intents(
        config.message_content_enabled,
        config.guild_presences_enabled,
    );
    let cache_settings = {
        let mut settings = serenity::cache::Settings::default();
        settings.max_messages = config.cache_max_messages;
        settings
    };
    let mut client = serenity::ClientBuilder::new(&config.discord_token, intents)
        .framework(framework)
        .cache_settings(cache_settings)
        .await?;

    client.start().await?;
    Ok(())
}

fn gateway_intents(
    message_content_enabled: bool,
    guild_presences_enabled: bool,
) -> serenity::GatewayIntents {
    let mut intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::GUILD_MEMBERS
        | serenity::GatewayIntents::GUILD_VOICE_STATES;
    if message_content_enabled {
        intents |= serenity::GatewayIntents::MESSAGE_CONTENT;
    }
    if guild_presences_enabled {
        intents |= serenity::GatewayIntents::GUILD_PRESENCES;
    }
    intents
}

async fn resolve_owners(config: &Config) -> HashSet<serenity::UserId> {
    let mut owners = config
        .owner_ids
        .iter()
        .copied()
        .map(serenity::UserId::new)
        .collect::<HashSet<_>>();

    if owners.is_empty() {
        let http = serenity::Http::new(&config.discord_token);
        match http.get_current_application_info().await {
            Ok(info) => {
                if let Some(owner) = info.owner {
                    owners.insert(owner.id);
                }
            }
            Err(error) => tracing::warn!(%error, "Could not auto-detect application owner"),
        }
    }

    if owners.is_empty() {
        tracing::warn!("No bot owner is configured");
    }
    owners
}

#[cfg(test)]
mod tests {
    use super::gateway_intents;
    use poise::serenity_prelude::GatewayIntents;

    #[test]
    fn startup_intents_support_message_content_enabled_and_disabled() {
        let enabled = gateway_intents(true, true);
        let disabled = gateway_intents(false, false);
        assert!(enabled.contains(GatewayIntents::MESSAGE_CONTENT));
        assert!(!disabled.contains(GatewayIntents::MESSAGE_CONTENT));
        assert!(enabled.contains(GatewayIntents::GUILD_PRESENCES));
        assert!(!disabled.contains(GatewayIntents::GUILD_PRESENCES));
        for intent in [
            GatewayIntents::GUILD_MEMBERS,
            GatewayIntents::GUILD_VOICE_STATES,
            GatewayIntents::GUILD_MESSAGES,
            GatewayIntents::GUILD_SCHEDULED_EVENTS,
            GatewayIntents::AUTO_MODERATION_EXECUTION,
            GatewayIntents::AUTO_MODERATION_CONFIGURATION,
        ] {
            assert!(enabled.contains(intent));
            assert!(disabled.contains(intent));
        }
    }
}
