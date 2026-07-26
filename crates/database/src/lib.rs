use config_crate::AppConfig;
use errors::{AppError, Result};
use sqlx::postgres::PgPoolOptions;
pub use sqlx::PgPool;

pub async fn init_db(config: &AppConfig) -> Result<PgPool> {
    tracing::info!("Connecting to database at {}", config.database_url);
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .connect(&config.database_url)
        .await
        .map_err(AppError::Database)?;

    tracing::info!("Running database migrations...");
    sqlx::migrate!("./../../migrations")
        .run(&pool)
        .await
        .map_err(|err| AppError::Internal(format!("Database migration failed: {}", err)))?;

    tracing::info!("Database migrations applied successfully.");
    Ok(pool)
}
