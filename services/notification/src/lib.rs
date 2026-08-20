use async_trait::async_trait;
use chrono::Utc;
use database::PgPool;
use errors::{AppError, Result};
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing;
use uuid::Uuid;

#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send_notification(
        &self,
        user_id: Uuid,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<()>;
    async fn register_device(&self, user_id: Uuid, token: &str, platform: &str) -> Result<()>;
    async fn unregister_device(&self, user_id: Uuid, token: &str) -> Result<()>;
    async fn get_notifications(&self, user_id: Uuid) -> Result<Vec<models::Notification>>;
    async fn mark_all_read(&self, user_id: Uuid) -> Result<()>;
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PushOutcome {
    Success,
    UnregisteredToken,
    Failed(String),
}

#[async_trait]
pub trait NotificationProvider: Send + Sync {
    async fn send_push(
        &self,
        device_token: &str,
        platform: &str,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<PushOutcome>;
}

pub struct MockNotificationProvider;

#[async_trait]
impl NotificationProvider for MockNotificationProvider {
    async fn send_push(
        &self,
        device_token: &str,
        platform: &str,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<PushOutcome> {
        tracing::info!(
            "Mock sending push to device ({}): platform={}, title='{}', body='{}', data={:?}",
            device_token,
            platform,
            title,
            body,
            data
        );
        Ok(PushOutcome::Success)
    }
}

/// Firebase Service Account JSON credentials structure
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountKey {
    pub project_id: String,
    pub private_key: String,
    pub client_email: String,
    pub token_uri: Option<String>,
}

impl ServiceAccountKey {
    pub fn from_env() -> Option<Self> {
        // 1. Try FCM_SERVICE_ACCOUNT_JSON
        if let Ok(json_str) = std::env::var("FCM_SERVICE_ACCOUNT_JSON") {
            if !json_str.trim().is_empty() {
                match serde_json::from_str::<Self>(&json_str) {
                    Ok(key) => {
                        tracing::info!("Loaded FCM Service Account key from FCM_SERVICE_ACCOUNT_JSON");
                        return Some(key);
                    }
                    Err(e) => tracing::error!("Failed to parse FCM_SERVICE_ACCOUNT_JSON: {}", e),
                }
            }
        }

        // 2. Try FCM_SERVICE_ACCOUNT_FILE
        if let Ok(filepath) = std::env::var("FCM_SERVICE_ACCOUNT_FILE") {
            if !filepath.trim().is_empty() {
                match std::fs::read_to_string(&filepath) {
                    Ok(content) => match serde_json::from_str::<Self>(&content) {
                        Ok(key) => {
                            tracing::info!("Loaded FCM Service Account key from {}", filepath);
                            return Some(key);
                        }
                        Err(e) => tracing::error!(
                            "Failed to parse FCM service account file at {}: {}",
                            filepath,
                            e
                        ),
                    },
                    Err(e) => tracing::error!(
                        "Failed to read FCM service account file at {}: {}",
                        filepath,
                        e
                    ),
                }
            }
        }

        // 3. Fallback default file checks
        let possible_paths = [
            "./firebase-service-account.json",
            "../firebase-service-account.json",
            "../../firebase-service-account.json",
        ];

        for default_path in possible_paths {
            if std::path::Path::new(default_path).exists() {
                if let Ok(content) = std::fs::read_to_string(default_path) {
                    if let Ok(key) = serde_json::from_str::<Self>(&content) {
                        tracing::info!("Loaded FCM Service Account key from path {}", default_path);
                        return Some(key);
                    }
                }
            }
        }

        None
    }
}


/// Cached OAuth2 access token for Google APIs
#[derive(Clone)]
struct CachedAccessToken {
    token: String,
    expires_at: Instant,
}

/// Thread-safe token manager that signs RS256 JWTs and fetches short-lived OAuth2 tokens for FCM HTTP v1 API
pub struct FcmTokenManager {
    service_account: ServiceAccountKey,
    client: Client,
    cached_token: RwLock<Option<CachedAccessToken>>,
}

#[derive(Serialize)]
struct GoogleJwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

impl FcmTokenManager {
    pub fn new(service_account: ServiceAccountKey) -> Self {
        Self {
            service_account,
            client: Client::new(),
            cached_token: RwLock::new(None),
        }
    }

    pub async fn get_access_token(&self) -> Result<String> {
        // Read lock check
        {
            let guard = self.cached_token.read().await;
            if let Some(cached) = guard.as_ref() {
                if Instant::now() + Duration::from_secs(300) < cached.expires_at {
                    return Ok(cached.token.clone());
                }
            }
        }

        // Write lock refresh
        let mut guard = self.cached_token.write().await;
        if let Some(cached) = guard.as_ref() {
            if Instant::now() + Duration::from_secs(300) < cached.expires_at {
                return Ok(cached.token.clone());
            }
        }

        let now = Utc::now().timestamp();
        let claims = GoogleJwtClaims {
            iss: self.service_account.client_email.clone(),
            scope: "https://www.googleapis.com/auth/firebase.messaging".to_string(),
            aud: self
                .service_account
                .token_uri
                .clone()
                .unwrap_or_else(|| "https://oauth2.googleapis.com/token".to_string()),
            iat: now,
            exp: now + 3600,
        };

        let pem_str = self
            .service_account
            .private_key
            .replace("\\n", "\n")
            .replace("\\r", "");
        let pem_str = self
            .service_account
            .private_key
            .replace("\\n", "\n")
            .replace("\\r", "");
        let encoding_key = EncodingKey::from_rsa_pem(pem_str.as_bytes())
            .map_err(|e| {
                println!("FCM RSA PEM Error Details: {:?}", e);
                AppError::Internal(format!("Invalid FCM RSA private key: {:?}", e))
            })?;


        let jwt = jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &encoding_key)
            .map_err(|e| AppError::Internal(format!("Failed to sign FCM OAuth2 JWT: {}", e)))?;

        let token_url = self
            .service_account
            .token_uri
            .as_deref()
            .unwrap_or("https://oauth2.googleapis.com/token");

        let res = self
            .client
            .post(token_url)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer"),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("OAuth2 token request failed: {}", e)))?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!(
                "OAuth2 token fetch failed (status {}): {}",
                status, body
            )));
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: u64,
        }

        let token_res: TokenResponse = res.json().await.map_err(|e| {
            AppError::Internal(format!("Failed to parse Google OAuth2 response: {}", e))
        })?;

        let expires_in_sec = if token_res.expires_in > 60 {
            token_res.expires_in - 60
        } else {
            token_res.expires_in
        };

        let cached = CachedAccessToken {
            token: token_res.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(expires_in_sec),
        };

        *guard = Some(cached);
        Ok(token_res.access_token)
    }

    pub fn project_id(&self) -> &str {
        &self.service_account.project_id
    }
}

/// Production FCM HTTP v1 Provider
pub struct FcmV1NotificationProvider {
    token_manager: FcmTokenManager,
    client: Client,
}

impl FcmV1NotificationProvider {
    pub fn new(service_account: ServiceAccountKey) -> Self {
        Self {
            token_manager: FcmTokenManager::new(service_account),
            client: Client::new(),
        }
    }

    async fn send_single_push(
        &self,
        device_token: &str,
        _platform: &str,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<PushOutcome> {
        let access_token = match self.token_manager.get_access_token().await {
            Ok(t) => t,
            Err(e) => {
                tracing::error!("FCM OAuth2 access token error: {}", e);
                return Ok(PushOutcome::Failed(e.to_string()));
            }
        };

        let project_id = self.token_manager.project_id();
        let url = format!(
            "https://fcm.googleapis.com/v1/projects/{}/messages:send",
            project_id
        );

        let mut data_map = HashMap::new();
        if let Some(d) = data {
            if let Some(obj) = d.as_object() {
                for (k, v) in obj {
                    let val_str = match v {
                        serde_json::Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    data_map.insert(k.clone(), val_str);
                }
            }
        }

        let payload = serde_json::json!({
            "message": {
                "token": device_token,
                "notification": {
                    "title": title,
                    "body": body
                },
                "data": data_map,
                "android": {
                    "priority": "HIGH"
                },
                "apns": {
                    "payload": {
                        "aps": {
                            "sound": "default",
                            "content-available": 1
                        }
                    }
                }
            }
        });

        // Retry transient errors (HTTP 5xx / Network) up to 2 attempts
        for attempt in 1..=2 {
            let response = self
                .client
                .post(&url)
                .bearer_auth(&access_token)
                .header("Content-Type", "application/json")
                .json(&payload)
                .send()
                .await;

            match response {
                Ok(res) => {
                    let status = res.status();
                    if status.is_success() {
                        tracing::info!("FCM HTTP v1 Push delivered to token {}", device_token);
                        return Ok(PushOutcome::Success);
                    }

                    let err_body = res.text().await.unwrap_or_default();
                    tracing::warn!(
                        "FCM HTTP v1 response error ({}) for token {}: {}",
                        status,
                        device_token,
                        err_body
                    );

                    // Check permanent token error codes (UNREGISTERED, INVALID_ARGUMENT, NOT_FOUND)
                    if status.as_u16() == 404 || status.as_u16() == 400 {
                        if err_body.contains("UNREGISTERED")
                            || err_body.contains("NOT_FOUND")
                            || err_body.contains("INVALID_ARGUMENT")
                            || err_body.contains("registration-token-not-registered")
                        {
                            return Ok(PushOutcome::UnregisteredToken);
                        }
                    }

                    if status.is_server_error() && attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }

                    return Ok(PushOutcome::Failed(format!(
                        "FCM API HTTP {}: {}",
                        status, err_body
                    )));
                }
                Err(e) => {
                    if attempt < 2 {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    return Ok(PushOutcome::Failed(format!("HTTP send error: {}", e)));
                }
            }
        }

        Ok(PushOutcome::Failed("Max retries exceeded".to_string()))
    }
}

#[async_trait]
impl NotificationProvider for FcmV1NotificationProvider {
    async fn send_push(
        &self,
        device_token: &str,
        platform: &str,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<PushOutcome> {
        self.send_single_push(device_token, platform, title, body, data)
            .await
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

#[async_trait]
impl NotificationService for NotificationServiceImpl {
    async fn register_device(&self, user_id: Uuid, token: &str, platform: &str) -> Result<()> {
        // Prevent token leaking across user accounts on shared physical devices
        sqlx::query("DELETE FROM devices WHERE token = $1 AND user_id != $2")
            .bind(token)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        let device_id = Uuid::now_v7();
        sqlx::query(
            r#"
            INSERT INTO devices (id, user_id, token, platform, updated_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (user_id, token) DO UPDATE SET updated_at = NOW(), platform = EXCLUDED.platform
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

    async fn unregister_device(&self, user_id: Uuid, token: &str) -> Result<()> {
        sqlx::query("DELETE FROM devices WHERE user_id = $1 AND token = $2")
            .bind(user_id)
            .bind(token)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn send_notification(
        &self,
        user_id: Uuid,
        title: &str,
        body: &str,
        data: Option<serde_json::Value>,
    ) -> Result<()> {
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
        let devices = sqlx::query(
            "SELECT token, platform FROM devices WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        if devices.is_empty() {
            return Ok(());
        }

        // 3. Dispatch push notifications asynchronously in spawned background task
        let pool = self.pool.clone();
        let provider = self.provider.clone();
        let title_owned = title.to_string();
        let body_owned = body.to_string();
        let data_owned = data.clone();

        tokio::spawn(async move {
            use futures_util::future::join_all;
            use sqlx::Row;

            let mut tasks = Vec::new();

            for device in &devices {
                let token: String = device.get("token");
                let platform: String = device.get("platform");
                let provider_ref = provider.clone();
                let t_title = title_owned.clone();
                let t_body = body_owned.clone();
                let t_data = data_owned.clone();

                tasks.push(async move {
                    let outcome = provider_ref
                        .send_push(&token, &platform, &t_title, &t_body, t_data)
                        .await;
                    (token, outcome)
                });
            }

            let results = join_all(tasks).await;
            let mut stale_tokens = Vec::new();

            for (token, outcome_res) in results {
                match outcome_res {
                    Ok(PushOutcome::UnregisteredToken) => {
                        tracing::info!("Marking stale token for pruning: {}", token);
                        stale_tokens.push(token);
                    }
                    Ok(PushOutcome::Success) => {}
                    Ok(PushOutcome::Failed(reason)) => {
                        tracing::warn!("Push failed for token {}: {}", token, reason);
                    }
                    Err(e) => {
                        tracing::error!("Push execution error for token {}: {}", token, e);
                    }
                }
            }

            // Prune stale tokens from database
            if !stale_tokens.is_empty() {
                for token in stale_tokens {
                    let _ = sqlx::query("DELETE FROM devices WHERE token = $1")
                        .bind(&token)
                        .execute(&pool)
                        .await;
                }
            }
        });

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_account_key_parsing() {
        let sample_json = r#"{
            "project_id": "test-project-123",
            "private_key": "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----",
            "client_email": "test@test-project-123.iam.gserviceaccount.com"
        }"#;

        let key: ServiceAccountKey = serde_json::from_str(sample_json).expect("Failed to parse sample json");
        assert_eq!(key.project_id, "test-project-123");
        assert_eq!(key.client_email, "test@test-project-123.iam.gserviceaccount.com");
    }

    #[tokio::test]
    async fn test_mock_notification_provider() {
        let provider = MockNotificationProvider;
        let res = provider
            .send_push(
                "token_123",
                "android",
                "Hello",
                "World",
                Some(serde_json::json!({ "type": "chat" })),
            )
            .await;
        assert_eq!(res.unwrap(), PushOutcome::Success);
    }

    #[tokio::test]
    async fn test_real_firebase_service_account_token() {
        let key = ServiceAccountKey::from_env().expect("ServiceAccountKey::from_env() must return Some(key)");
        let manager = FcmTokenManager::new(key);
        let token_res = manager.get_access_token().await;
        assert!(
            token_res.is_ok(),
            "Failed to get OAuth2 access token from Google: {:?}",
            token_res.err()
        );
        let token = token_res.unwrap();
        assert!(!token.is_empty(), "Access token should not be empty");
    }
}



