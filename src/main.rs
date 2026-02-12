use my_rust_bot::commands;
use my_rust_bot::config;
use my_rust_bot::database;
use my_rust_bot::error;
use my_rust_bot::Data;

use poise::serenity_prelude as serenity;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file (silently skip if not present)
    let _ = dotenv::dotenv();

    // Initialize structured logging with tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                tracing_subscriber::EnvFilter::new("my_rust_bot=info,serenity=warn")
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

    // Build the Poise framework
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: commands::all(),
            on_error: |error| Box::pin(error::on_error(error)),
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

                Ok(Data {
                    db_pool,
                    start_time,
                })
            })
        })
        .build();

    // Configure gateway intents
    let intents = serenity::GatewayIntents::non_privileged()
        | serenity::GatewayIntents::MESSAGE_CONTENT
        | serenity::GatewayIntents::GUILD_MEMBERS;

    // Build and start the Serenity client
    let mut client = serenity::ClientBuilder::new(&config.discord_token, intents)
        .framework(framework)
        .await?;

    // Run the bot (blocks until shutdown)
    tracing::info!("Bot is starting...");
    client.start().await?;

    Ok(())
}
