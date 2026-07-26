use errors::{AppError, Result};
use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct AppConfig {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_refresh_secret: String,

    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_environment")]
    pub environment: String,

    #[serde(default = "default_jwt_access_expiration_minutes")]
    pub jwt_access_expiration_minutes: i64,
    #[serde(default = "default_jwt_refresh_expiration_days")]
    pub jwt_refresh_expiration_days: i64,

    #[serde(default = "default_upload_dir")]
    pub upload_dir: String,

    pub s3_bucket: Option<String>,
    pub s3_endpoint: Option<String>,
    pub s3_region: Option<String>,
    pub s3_access_key: Option<String>,
    pub s3_secret_key: Option<String>,
}

fn default_port() -> u16 {
    8080
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_jwt_access_expiration_minutes() -> i64 {
    15
}

fn default_jwt_refresh_expiration_days() -> i64 {
    7
}

fn default_upload_dir() -> String {
    "./uploads".to_string()
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        // Attempt to load .env file
        let _ = dotenvy::dotenv();

        let run_mode = env::var("APP_ENV").unwrap_or_else(|_| "development".into());

        let builder = config::Config::builder()
            // Add configuration fields directly from environment variables
            .add_source(config::Environment::default());

        let cfg = builder.build().map_err(|err| {
            AppError::Internal(format!("Failed to build configuration: {}", err))
        })?;

        let mut app_config: AppConfig = cfg.try_deserialize().map_err(|err| {
            AppError::Internal(format!("Configuration deserialization failed: {}", err))
        })?;

        // Normalize run mode
        app_config.environment = run_mode;

        Ok(app_config)
    }
}
