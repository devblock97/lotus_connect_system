use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use database::PgPool;
use config_crate::AppConfig;
use errors::{AppError, Result};
use models::RefreshToken;
use dto::{RegisterRequest, LoginRequest, AuthResponse, UserResponse, TokenResponse};
use service_users::UserService;

#[async_trait::async_trait]
pub trait RefreshTokenRepository: Send + Sync {
    async fn create(&self, id: Uuid, user_id: Uuid, token: &str, expires_at: chrono::DateTime<Utc>) -> Result<RefreshToken>;
    async fn find_by_token(&self, token: &str) -> Result<Option<RefreshToken>>;
    async fn revoke_by_token(&self, token: &str) -> Result<()>;
    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<()>;
}

#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    async fn register(&self, req: RegisterRequest) -> Result<UserResponse>;
    async fn login(&self, req: LoginRequest) -> Result<AuthResponse>;
    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse>;
    async fn logout(&self, refresh_token: &str) -> Result<()>;
}

pub struct RefreshTokenRepositoryImpl {
    pool: PgPool,
}

impl RefreshTokenRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl RefreshTokenRepository for RefreshTokenRepositoryImpl {
    async fn create(&self, id: Uuid, user_id: Uuid, token: &str, expires_at: chrono::DateTime<Utc>) -> Result<RefreshToken> {
        sqlx::query_as::<_, RefreshToken>(
            "INSERT INTO refresh_tokens (id, user_id, token, expires_at, revoked) VALUES ($1, $2, $3, $4, FALSE) RETURNING id, user_id, token, expires_at, created_at, revoked"
        )
        .bind(id)
        .bind(user_id)
        .bind(token)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_by_token(&self, token: &str) -> Result<Option<RefreshToken>> {
        sqlx::query_as::<_, RefreshToken>(
            "SELECT id, user_id, token, expires_at, created_at, revoked FROM refresh_tokens WHERE token = $1"
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn revoke_by_token(&self, token: &str) -> Result<()> {
        sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE token = $1")
            .bind(token)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn revoke_all_for_user(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }
}

pub struct AuthServiceImpl {
    config: AppConfig,
    pool: PgPool,
    user_service: Arc<dyn UserService>,
    token_repo: Arc<dyn RefreshTokenRepository>,
}

impl AuthServiceImpl {
    pub fn new(
        config: AppConfig,
        pool: PgPool,
        user_service: Arc<dyn UserService>,
        token_repo: Arc<dyn RefreshTokenRepository>,
    ) -> Self {
        Self {
            config,
            pool,
            user_service,
            token_repo,
        }
    }
}

#[async_trait::async_trait]
impl AuthService for AuthServiceImpl {
    async fn register(&self, req: RegisterRequest) -> Result<UserResponse> {
        let password_hash = auth::hash_password(&req.password)?;
        let user = self.user_service.create_user(&req.username, req.full_name.as_deref(), &req.email, &password_hash).await?;

        Ok(UserResponse {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            email: user.email,
            friendship_status: None,
            friendship_sender_id: None,
        })
    }

    async fn login(&self, req: LoginRequest) -> Result<AuthResponse> {
        let user = self.user_service.get_user_by_email(&req.email).await?;

        let is_valid = auth::verify_password(&req.password, &user.password_hash)?;
        if !is_valid {
            return Err(AppError::Authentication("Invalid email or password".to_string()));
        }

        let (access_token, _) = auth::generate_token(
            user.id,
            auth::TokenType::Access,
            &self.config.jwt_secret,
            self.config.jwt_access_expiration_minutes,
        )?;

        let (refresh_token_str, exp) = auth::generate_token(
            user.id,
            auth::TokenType::Refresh,
            &self.config.jwt_refresh_secret,
            self.config.jwt_refresh_expiration_days,
        )?;

        let refresh_token_id = Uuid::now_v7();
        let expires_at = chrono::DateTime::from_timestamp(exp, 0)
            .unwrap_or_else(|| Utc::now() + chrono::Duration::days(self.config.jwt_refresh_expiration_days));
        
        self.token_repo.create(refresh_token_id, user.id, &refresh_token_str, expires_at).await?;

        if let (Some(token), Some(platform)) = (req.device_token, req.platform) {
            let device_id = Uuid::now_v7();
            sqlx::query(
                r#"
                INSERT INTO devices (id, user_id, token, platform, updated_at)
                VALUES ($1, $2, $3, $4, NOW())
                ON CONFLICT (user_id, token) DO UPDATE SET updated_at = NOW()
                "#
            )
            .bind(device_id)
            .bind(user.id)
            .bind(token)
            .bind(platform)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        }

        Ok(AuthResponse {
            user: UserResponse {
                id: user.id,
                username: user.username,
                full_name: user.full_name,
                email: user.email,
                friendship_status: None,
                friendship_sender_id: None,
            },
            access_token,
            refresh_token: refresh_token_str,
        })
    }

    async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse> {
        let claims = auth::verify_token(refresh_token, &self.config.jwt_refresh_secret)?;
        if claims.token_type != auth::TokenType::Refresh {
            return Err(AppError::Authentication("Invalid token type".to_string()));
        }

        let db_token = self.token_repo.find_by_token(refresh_token).await?
            .ok_or_else(|| AppError::Authentication("Refresh token not found".to_string()))?;

        if db_token.revoked {
            self.token_repo.revoke_all_for_user(claims.sub).await?;
            return Err(AppError::Authentication("Compromised token used. Revoked all sessions.".to_string()));
        }

        if db_token.expires_at < Utc::now() {
            return Err(AppError::Authentication("Refresh token has expired".to_string()));
        }

        self.token_repo.revoke_by_token(refresh_token).await?;

        let (new_access_token, _) = auth::generate_token(
            claims.sub,
            auth::TokenType::Access,
            &self.config.jwt_secret,
            self.config.jwt_access_expiration_minutes,
        )?;

        let (new_refresh_token_str, exp) = auth::generate_token(
            claims.sub,
            auth::TokenType::Refresh,
            &self.config.jwt_refresh_secret,
            self.config.jwt_refresh_expiration_days,
        )?;

        let new_token_id = Uuid::now_v7();
        let expires_at = chrono::DateTime::from_timestamp(exp, 0)
            .unwrap_or_else(|| Utc::now() + chrono::Duration::days(self.config.jwt_refresh_expiration_days));

        self.token_repo.create(new_token_id, claims.sub, &new_refresh_token_str, expires_at).await?;

        Ok(TokenResponse {
            access_token: new_access_token,
            refresh_token: new_refresh_token_str,
        })
    }

    async fn logout(&self, refresh_token: &str) -> Result<()> {
        self.token_repo.revoke_by_token(refresh_token).await
    }
}
