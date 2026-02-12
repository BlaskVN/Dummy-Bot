use rust_discord_bot::Data;
use rust_discord_bot::commands;
use rust_discord_bot::config;
use rust_discord_bot::database;
use rust_discord_bot::error;
use rust_discord_bot::handlers;

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

    // Build the Poise framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
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

                // Register slash commands globally
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                tracing::info!("Slash commands registered globally");

                // // Initialize HTTP client for downloading attachments
                // let http_client = reqwest::Client::new();

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
