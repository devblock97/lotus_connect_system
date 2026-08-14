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

pub struct FcmNotificationProvider {
    client: reqwest::Client,
    server_key: Option<String>,
}

impl FcmNotificationProvider {
    pub fn new(server_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            server_key,
        }
    }
}

#[async_trait::async_trait]
impl NotificationProvider for FcmNotificationProvider {
    async fn send_push(
        &self,
        device_token: &str,
        platform: &str,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<()> {
        let server_key = match &self.server_key {
            Some(key) if !key.is_empty() => key,
            _ => {
                tracing::info!(
                    "FCM server key not set. Logged push to device ({}): platform={}, title='{}', body='{}'",
                    device_token,
                    platform,
                    title,
                    body
                );
                return Ok(());
            }
        };

        let is_call_invite = data
            .as_ref()
            .and_then(|d| d.get("type"))
            .and_then(|t| t.as_str())
            .map(|t| t == "call_invite")
            .unwrap_or(false);

        let priority = if is_call_invite { "high" } else { "normal" };

        let payload = serde_json::json!({
            "to": device_token,
            "priority": priority,
            "notification": {
                "title": title,
                "body": body,
                "sound": "default"
            },
            "data": data.unwrap_or_else(|| serde_json::json!({}))
        });

        let response = self
            .client
            .post("https://fcm.googleapis.com/fcm/send")
            .header("Authorization", format!("key={}", server_key))
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await;

        match response {
            Ok(res) if res.status().is_success() => {
                tracing::info!("FCM Push successfully dispatched to {}", device_token);
                Ok(())
            }
            Ok(res) => {
                let status = res.status();
                let err_text = res.text().await.unwrap_or_default();
                tracing::warn!(
                    "FCM Push failed with status {}: {} for token {}",
                    status,
                    err_text,
                    device_token
                );
                Ok(())
            }
            Err(e) => {
                tracing::error!("FCM HTTP request error for token {}: {}", device_token, e);
                Ok(())
            }
        }
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
            VALUES (, , , , NOW())
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
            VALUES (, , , , , FALSE)
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
        let devices = sqlx::query(
            "SELECT token, platform FROM devices WHERE user_id = "
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        // 3. Dispatch the push events via the active provider
        for device in devices {
            use sqlx::Row;
            let token: String = device.get("token");
            let platform: String = device.get("platform");
            let _ = self.provider.send_push(&token, &platform, title, body, data.clone()).await;
        }

        Ok(())
    }

    async fn get_notifications(&self, user_id: Uuid) -> Result<Vec<models::Notification>> {
        sqlx::query_as::<_, models::Notification>(
            "SELECT id, user_id, title, body, data, is_read, created_at FROM notifications WHERE user_id =  ORDER BY created_at DESC LIMIT 50"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn mark_all_read(&self, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE notifications SET is_read = TRUE WHERE user_id = "
        )
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }
}
