use std::sync::Arc;
use uuid::Uuid;
use models::{User, Friendship};
use errors::{AppError, Result};
use database::PgPool;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync {
    async fn create(&self, id: Uuid, username: &str, email: &str, password_hash: &str) -> Result<User>;
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>>;
    
    // Friendship management
    async fn create_friendship(&self, id: Uuid, user_id: Uuid, friend_id: Uuid, status: &str) -> Result<Friendship>;
    async fn find_friendship(&self, user_id: Uuid, friend_id: Uuid) -> Result<Option<Friendship>>;
    async fn update_friendship_status(&self, user_id: Uuid, friend_id: Uuid, status: &str) -> Result<()>;
    async fn list_friends(&self, user_id: Uuid) -> Result<Vec<User>>;
}

#[async_trait::async_trait]
pub trait UserService: Send + Sync {
    async fn create_user(&self, username: &str, email: &str, password_hash: &str) -> Result<User>;
    async fn get_user_by_id(&self, id: Uuid) -> Result<User>;
    async fn get_user_by_email(&self, email: &str) -> Result<User>;
    async fn get_user_by_username(&self, username: &str) -> Result<User>;
    
    async fn send_friend_request(&self, user_id: Uuid, friend_username: &str) -> Result<Friendship>;
    async fn accept_friend_request(&self, user_id: Uuid, friend_id: Uuid) -> Result<()>;
    async fn get_friends_list(&self, user_id: Uuid) -> Result<Vec<User>>;
}

pub struct UserRepositoryImpl {
    pool: PgPool,
}

impl UserRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for UserRepositoryImpl {
    async fn create(&self, id: Uuid, username: &str, email: &str, password_hash: &str) -> Result<User> {
        sqlx::query_as::<_, User>(
            "INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING id, username, email, password_hash, created_at, updated_at"
        )
        .bind(id)
        .bind(username)
        .bind(email)
        .bind(password_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>> {
        sqlx::query_as::<_, User>(
            "SELECT id, username, email, password_hash, created_at, updated_at FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn create_friendship(&self, id: Uuid, user_id: Uuid, friend_id: Uuid, status: &str) -> Result<Friendship> {
        sqlx::query_as::<_, Friendship>(
            "INSERT INTO friendships (id, user_id, friend_id, status) VALUES ($1, $2, $3, $4) RETURNING id, user_id, friend_id, status, created_at"
        )
        .bind(id)
        .bind(user_id)
        .bind(friend_id)
        .bind(status)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_friendship(&self, user_id: Uuid, friend_id: Uuid) -> Result<Option<Friendship>> {
        sqlx::query_as::<_, Friendship>(
            "SELECT id, user_id, friend_id, status, created_at FROM friendships WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)"
        )
        .bind(user_id)
        .bind(friend_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn update_friendship_status(&self, user_id: Uuid, friend_id: Uuid, status: &str) -> Result<()> {
        sqlx::query(
            "UPDATE friendships SET status = $3 WHERE (user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1)"
        )
        .bind(user_id)
        .bind(friend_id)
        .bind(status)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn list_friends(&self, user_id: Uuid) -> Result<Vec<User>> {
        sqlx::query_as::<_, User>(
            r#"
            SELECT u.id, u.username, u.email, u.password_hash, u.created_at, u.updated_at 
            FROM users u
            JOIN friendships f ON (f.user_id = u.id OR f.friend_id = u.id)
            WHERE (f.user_id = $1 OR f.friend_id = $1) AND f.status = 'accepted' AND u.id != $1
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }
}

pub struct UserServiceImpl {
    repo: Arc<dyn UserRepository>,
}

impl UserServiceImpl {
    pub fn new(repo: Arc<dyn UserRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl UserService for UserServiceImpl {
    async fn create_user(&self, username: &str, email: &str, password_hash: &str) -> Result<User> {
        if self.repo.find_by_email(email).await?.is_some() {
            return Err(AppError::Conflict("A user with this email already exists".to_string()));
        }
        if self.repo.find_by_username(username).await?.is_some() {
            return Err(AppError::Conflict("A user with this username already exists".to_string()));
        }

        let user_id = Uuid::now_v7();
        self.repo.create(user_id, username, email, password_hash).await
    }

    async fn get_user_by_id(&self, id: Uuid) -> Result<User> {
        self.repo.find_by_id(id).await?.ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    async fn get_user_by_email(&self, email: &str) -> Result<User> {
        self.repo.find_by_email(email).await?.ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    async fn get_user_by_username(&self, username: &str) -> Result<User> {
        self.repo.find_by_username(username).await?.ok_or_else(|| AppError::NotFound("User not found".to_string()))
    }

    async fn send_friend_request(&self, user_id: Uuid, friend_username: &str) -> Result<Friendship> {
        let friend = self.get_user_by_username(friend_username).await?;
        if friend.id == user_id {
            return Err(AppError::Validation("You cannot add yourself as a friend".to_string()));
        }

        if let Some(friendship) = self.repo.find_friendship(user_id, friend.id).await? {
            return Err(AppError::Conflict(format!("Friendship status is already: {}", friendship.status)));
        }

        let friendship_id = Uuid::now_v7();
        self.repo.create_friendship(friendship_id, user_id, friend.id, "pending").await
    }

    async fn accept_friend_request(&self, user_id: Uuid, friend_id: Uuid) -> Result<()> {
        let friendship = self.repo.find_friendship(user_id, friend_id).await?
            .ok_or_else(|| AppError::NotFound("Friend request not found".to_string()))?;

        if friendship.status != "pending" {
            return Err(AppError::Validation("Friend request is not pending".to_string()));
        }

        self.repo.update_friendship_status(user_id, friend_id, "accepted").await
    }

    async fn get_friends_list(&self, user_id: Uuid) -> Result<Vec<User>> {
        self.repo.list_friends(user_id).await
    }
}
