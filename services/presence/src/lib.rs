use uuid::Uuid;
use chrono::Utc;
use redis::AsyncCommands;
use redis::aio::ConnectionManager;

use database::PgPool;
use errors::{AppError, Result};
use models::UserPresence;

#[async_trait::async_trait]
pub trait PresenceService: Send + Sync {
    async fn set_status(&self, user_id: Uuid, status: &str) -> Result<()>;
    async fn get_status(&self, user_id: Uuid) -> Result<Option<UserPresence>>;
    async fn heartbeat(&self, user_id: Uuid) -> Result<()>;
}

pub struct PresenceServiceImpl {
    pool: PgPool,
    redis_conn: ConnectionManager,
}

impl PresenceServiceImpl {
    pub fn new(pool: PgPool, redis_conn: ConnectionManager) -> Self {
        Self { pool, redis_conn }
    }
}

#[async_trait::async_trait]
impl PresenceService for PresenceServiceImpl {
    async fn set_status(&self, user_id: Uuid, status: &str) -> Result<()> {
        let mut conn = self.redis_conn.clone();
        let key = format!("presence:{}", user_id);

        // 1. Write to Redis with a TTL of 60 seconds (heartbeat window)
        conn.set_ex::<_, _, ()>(&key, status, 60)
            .await
            .map_err(|err| AppError::Internal(format!("Redis presence write failed: {}", err)))?;

        // 2. Sync to Postgres (persistent user_presence)
        sqlx::query(
            r#"
            INSERT INTO user_presence (user_id, status, last_seen)
            VALUES ($1, $2, NOW())
            ON CONFLICT (user_id) DO UPDATE SET status = $2, last_seen = NOW()
            "#
        )
        .bind(user_id)
        .bind(status)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn get_status(&self, user_id: Uuid) -> Result<Option<UserPresence>> {
        let mut conn = self.redis_conn.clone();
        let key = format!("presence:{}", user_id);

        // 1. Try to read from Redis
        let status: Option<String> = conn.get(&key)
            .await
            .map_err(|err| AppError::Internal(format!("Redis presence fetch failed: {}", err)))?;

        if let Some(s) = status {
            return Ok(Some(UserPresence {
                user_id,
                status: s,
                last_seen: Utc::now(),
            }));
        }

        // 2. Fall back to Postgres if not in Redis cache
        let presence = sqlx::query_as::<_, UserPresence>(
            "SELECT user_id, status, last_seen FROM user_presence WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(presence)
    }

    async fn heartbeat(&self, user_id: Uuid) -> Result<()> {
        let mut conn = self.redis_conn.clone();
        let key = format!("presence:{}", user_id);

        // Retrieve current status from Redis, or default to "online"
        let status: Option<String> = conn.get(&key)
            .await
            .map_err(|err| AppError::Internal(format!("Redis presence get failed: {}", err)))?;
        
        let status = status.unwrap_or_else(|| "online".to_string());

        // Refresh expiration window to 60 seconds
        conn.set_ex::<_, _, ()>(&key, &status, 60)
            .await
            .map_err(|err| AppError::Internal(format!("Redis presence expire failed: {}", err)))?;

        // Update last_seen in Postgres
        sqlx::query("UPDATE user_presence SET last_seen = NOW() WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }
}
