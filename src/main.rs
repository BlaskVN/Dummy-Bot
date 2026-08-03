use rust_discord_bot::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load()?;

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new(&config.log_filter))
        .compact()
        .init();

    rust_discord_bot::run(config).await
}
