use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use database::PgPool;
use errors::{AppError, Result};
use models::{Call, CallEvent};

#[async_trait::async_trait]
pub trait CallRepository: Send + Sync {
    async fn create_call(&self, id: Uuid, host_id: Uuid, conversation_id: Option<Uuid>, channel_id: &str, is_video: bool) -> Result<Call>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Call>>;
    async fn update_status(&self, id: Uuid, status: &str, ended_at: Option<chrono::DateTime<Utc>>) -> Result<()>;
    
    async fn add_participant(&self, call_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn update_participant_left(&self, call_id: Uuid, user_id: Uuid) -> Result<()>;
    
    async fn create_event(&self, id: Uuid, call_id: Uuid, user_id: Uuid, event_type: &str) -> Result<CallEvent>;
    async fn list_calls_for_user(&self, user_id: Uuid) -> Result<Vec<Call>>;
}

#[async_trait::async_trait]
pub trait CallService: Send + Sync {
    async fn start_call(&self, host_id: Uuid, conversation_id: Option<Uuid>, channel_id: &str, is_video: bool) -> Result<Call>;
    async fn get_call(&self, id: Uuid) -> Result<Option<Call>>;
    async fn join_call(&self, call_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn leave_call(&self, call_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn end_call(&self, call_id: Uuid) -> Result<()>;
    async fn log_event(&self, call_id: Uuid, user_id: Uuid, event_type: &str) -> Result<()>;
    async fn get_user_call_history(&self, user_id: Uuid) -> Result<Vec<Call>>;
}

pub struct CallRepositoryImpl {
    pool: PgPool,
}

impl CallRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl CallRepository for CallRepositoryImpl {
    async fn create_call(&self, id: Uuid, host_id: Uuid, conversation_id: Option<Uuid>, channel_id: &str, is_video: bool) -> Result<Call> {
        sqlx::query_as::<_, Call>(
            "INSERT INTO calls (id, host_id, conversation_id, channel_id, is_video, status) VALUES ($1, $2, $3, $4, $5, 'initiated') RETURNING id, host_id, conversation_id, channel_id, is_video, status, created_at, ended_at"
        )
        .bind(id)
        .bind(host_id)
        .bind(conversation_id)
        .bind(channel_id)
        .bind(is_video)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<Call>> {
        sqlx::query_as::<_, Call>(
            "SELECT id, host_id, conversation_id, channel_id, is_video, status, created_at, ended_at FROM calls WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn update_status(&self, id: Uuid, status: &str, ended_at: Option<chrono::DateTime<Utc>>) -> Result<()> {
        sqlx::query("UPDATE calls SET status = $2, ended_at = $3 WHERE id = $1")
            .bind(id)
            .bind(status)
            .bind(ended_at)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn add_participant(&self, call_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO call_participants (call_id, user_id, joined_at) VALUES ($1, $2, NOW()) ON CONFLICT (call_id, user_id) DO UPDATE SET joined_at = NOW(), left_at = NULL"
        )
        .bind(call_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn update_participant_left(&self, call_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "UPDATE call_participants SET left_at = NOW() WHERE call_id = $1 AND user_id = $2"
        )
        .bind(call_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn create_event(&self, id: Uuid, call_id: Uuid, user_id: Uuid, event_type: &str) -> Result<CallEvent> {
        sqlx::query_as::<_, CallEvent>(
            "INSERT INTO call_events (id, call_id, user_id, event_type) VALUES ($1, $2, $3, $4) RETURNING id, call_id, user_id, event_type, created_at"
        )
        .bind(id)
        .bind(call_id)
        .bind(user_id)
        .bind(event_type)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_calls_for_user(&self, user_id: Uuid) -> Result<Vec<Call>> {
        sqlx::query_as::<_, Call>(
            r#"
            SELECT DISTINCT c.id, c.host_id, c.conversation_id, c.channel_id, c.is_video, c.status, c.created_at, c.ended_at
            FROM calls c
            LEFT JOIN call_participants p ON p.call_id = c.id
            WHERE c.host_id = $1 OR p.user_id = $1
            ORDER BY c.created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }
}

pub struct CallServiceImpl {
    repo: Arc<dyn CallRepository>,
}

impl CallServiceImpl {
    pub fn new(repo: Arc<dyn CallRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl CallService for CallServiceImpl {
    async fn start_call(&self, host_id: Uuid, conversation_id: Option<Uuid>, channel_id: &str, is_video: bool) -> Result<Call> {
        let call_id = Uuid::now_v7();
        let call = self.repo.create_call(call_id, host_id, conversation_id, channel_id, is_video).await?;
        self.repo.add_participant(call_id, host_id).await?;
        Ok(call)
    }

    async fn get_call(&self, id: Uuid) -> Result<Option<Call>> {
        self.repo.find_by_id(id).await
    }

    async fn join_call(&self, call_id: Uuid, user_id: Uuid) -> Result<()> {
        self.repo.add_participant(call_id, user_id).await?;
        self.repo.update_status(call_id, "active", None).await
    }

    async fn leave_call(&self, call_id: Uuid, user_id: Uuid) -> Result<()> {
        self.repo.update_participant_left(call_id, user_id).await
    }

    async fn end_call(&self, call_id: Uuid) -> Result<()> {
        self.repo.update_status(call_id, "ended", Some(Utc::now())).await
    }

    async fn log_event(&self, call_id: Uuid, user_id: Uuid, event_type: &str) -> Result<()> {
        let event_id = Uuid::now_v7();
        self.repo.create_event(event_id, call_id, user_id, event_type).await.map(|_| ())
    }

    async fn get_user_call_history(&self, user_id: Uuid) -> Result<Vec<Call>> {
        self.repo.list_calls_for_user(user_id).await
    }
}
