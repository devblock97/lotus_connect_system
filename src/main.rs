use config_crate::AppConfig;
use errors::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration
    let config = AppConfig::load()?;

    // 2. Initialize tracing configuration
    common::init_tracing(&config.environment);

    tracing::info!("Starting Lotus Connect System backend...");

    // 3. Initialize database connection pool
    let db_pool = database::init_db(&config).await?;

    // 4. Start the gateway server
    gateway::run_server(config, db_pool).await?;

    Ok(())
}
