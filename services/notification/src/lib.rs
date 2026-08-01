use std::sync::Arc;
use uuid::Uuid;
use database::PgPool;
use errors::{AppError, Result};

#[async_trait::async_trait]
pub trait NotificationService: Send + Sync {
    async fn send_notification(&self, user_id: Uuid, title: &str, body: &str, data: Option<serde_json::Value>) -> Result<()>;
    async fn register_device(&self, user_id: Uuid, token: &str, platform: &str) -> Result<()>;
    async fn get_notifications(&self, user_id: Uuid) -> Result<Vec<models::Notification>>;
    async fn mark_all_read(&self, user_id: Uuid) -> Result<()>;
}

#[async_trait::async_trait]
pub trait NotificationProvider: Send + Sync {
    async fn send_push(&self, device_token: &str, platform: &str, title: &str, body: &str, data: Option<serde_json::Value>) -> Result<()>;
}

pub struct MockNotificationProvider;

#[async_trait::async_trait]
impl NotificationProvider for MockNotificationProvider {
    async fn send_push(&self, device_token: &str, platform: &str, title: &str, body: &str, data: Option<serde_json::Value>) -> Result<()> {
        tracing::info!(
            "Mock sending push to device ({}): platform={}, title='{}', body='{}', data={:?}",
            device_token,
            platform,
            title,
            body,
            data
        );
        Ok(())
    }
}

pub struct NotificationServiceImpl {
    pool: PgPool,
    provider: Arc<dyn NotificationProvider>,
}

impl NotificationServiceImpl {
    pub fn new(pool: PgPool, provider: Arc<dyn NotificationProvider>) -> Self {
        Self { pool, provider }
    }
}

#[async_trait::async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn register_device(&self, user_id: Uuid, token: &str, platform: &str) -> Result<()> {
        let device_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO devices (id, user_id, token, platform, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id, token) DO UPDATE SET updated_at = NOW()
            "#
        )
        .bind(device_id)
        .bind(user_id)
        .bind(token)
        .bind(platform)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn send_notification(&self, user_id: Uuid, title: &str, body: &str, data: Option<serde_json::Value>) -> Result<()> {
        // 1. Write the notification record to PostgreSQL
        let notification_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO notifications (id, user_id, title, body, data, is_read)
            VALUES ($1, $2, $3, $4, $5, FALSE)
            "#
        )
        .bind(notification_id)
        .bind(user_id)
        .bind(title)
        .bind(body)
        .bind(&data)
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;

        // 2. Resolve active target devices for this user
        let devices = sqlx::query!(
            "SELECT token, platform FROM devices WHERE user_id = $1",
            user_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        // 3. Dispatch the push events via the active provider
        for device in devices {
            let _ = self.provider.send_push(&device.token, &device.platform, title, body, data.clone()).await;
        }

        Ok(())
    }

    async fn get_notifications(&self, user_id: Uuid) -> Result<Vec<models::Notification>> {
        sqlx::query_as::<_, models::Notification>(
            "SELECT id, user_id, title, body, data, is_read, created_at FROM notifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn mark_all_read(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE notifications SET is_read = TRUE WHERE user_id = $1"
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }
}
