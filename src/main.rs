use rust_discord_bot::commands;
use rust_discord_bot::config;
use rust_discord_bot::database;
use rust_discord_bot::error;
use rust_discord_bot::handlers;
use rust_discord_bot::Data;

use poise::serenity_prelude as serenity;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file (silently skip if not present)
    let _ = dotenv::dotenv();

    // Initialize structured logging with tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("RUST_Discord_Bot=info,serenity=warn")
            }),
        )
        .compact()
        .init();

    tracing::info!("Starting Discord bot...");

    // Load configuration
    let config = config::Config::from_env()?;

    // Initialize database
    let db_pool = database::init_db(&config.database_url).await?;

    // Record start time for uptime tracking
    let start_time = std::time::Instant::now();

    // Initialize HTTP client for downloading attachments
    let http_client = reqwest::Client::new();

    // Determine bot owner(s) before building framework
    let mut owners = std::collections::HashSet::new();

    // Check for OWNER_ID env var first (optional override)
    if let Ok(owner_id_str) = std::env::var("OWNER_ID") {
        if let Ok(id) = owner_id_str.parse::<u64>() {
            owners.insert(serenity::UserId::new(id));
            tracing::info!(owner_id = id, "Owner set from OWNER_ID env var");
        }
    }

    // If no env var, try to fetch from Discord application info using bot token
    if owners.is_empty() {
        let http = serenity::Http::new(&config.discord_token);
        match http.get_current_application_info().await {
            Ok(app_info) => {
                if let Some(owner) = app_info.owner {
                    owners.insert(owner.id);
                    tracing::info!(owner = %owner.name, "Owner set from application info");
                }
            }
            Err(e) => {
                tracing::warn!("Could not fetch application info for owner detection: {}", e);
            }
        }
    }

    if owners.is_empty() {
        tracing::warn!("No bot owners configured! Set OWNER_ID env var or ensure your bot application has an owner.");
    }

    // Build the Poise framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            owners,
            on_error: |error| Box::pin(error::on_error(error)),
            event_handler: |ctx, event, _framework, data| {
                Box::pin(async move {
                    match event {
                        serenity::FullEvent::MessageDelete {
                            channel_id,
                            deleted_message_id,
                            guild_id,
                        } => {
                            handlers::message_log::handle_message_delete(
                                ctx,
                                *channel_id,
                                *deleted_message_id,
                                *guild_id,
                                data,
                            )
                                .await;
                        }
                        serenity::FullEvent::MessageDeleteBulk {
                            channel_id,
                            multiple_deleted_messages_ids,
                            guild_id,
                        } => {
                            handlers::message_log::handle_message_delete_bulk(
                                ctx,
                                *channel_id,
                                multiple_deleted_messages_ids,
                                *guild_id,
                                data,
                            )
                                .await;
                        }
                        serenity::FullEvent::MessageUpdate { event, .. } => {
                            handlers::message_log::handle_message_update(ctx, event, data).await;
                        }
                        _ => {}
                    }
                    Ok(())
                })
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
                    "Bot is connected and ready!"
                );

                // Register slash commands globally with DM support
                // Convert poise commands to serenity commands with contexts set
                let commands = &framework.options().commands;
                let serenity_commands = poise::builtins::create_application_commands(commands);
                
                // Modify each command to support DM contexts
                let serenity_commands: Vec<_> = serenity_commands
                    .into_iter()
                    .map(|cmd| {
                        cmd
                            // Enable DM support: set contexts to allow guild, bot DM, and private channels
                            .contexts(vec![
                                serenity::InteractionContext::Guild,
                                serenity::InteractionContext::BotDm,
                                serenity::InteractionContext::PrivateChannel,
                            ])
                            // integration_types: Guild install and User install
                            .integration_types(vec![
                                serenity::InstallationContext::Guild,
                                serenity::InstallationContext::User,
                            ])
                    })
                    .collect();
                
                // Register the commands globally
                serenity::Command::set_global_commands(ctx, serenity_commands).await?;
                tracing::info!("Slash commands registered globally with DM support");

                Ok(Data {
                    db_pool,
                    start_time,
                    http_client,
                })
            })
        })
        .build();

    // Configure gateway intents
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS;

    // Build and start the Serenity client
    let cache_settings = {
        let mut settings = serenity::cache::Settings::default();
        settings.max_messages = 500;
        settings
    };
    let mut client = serenity::ClientBuilder::new(&config.discord_token, intents)
        .framework(framework)
        .cache_settings(cache_settings)
        .await?;

    // Run the bot (blocks until shutdown)
    tracing::info!("Bot is starting...");
    client.start().await?;

    Ok(())
}
