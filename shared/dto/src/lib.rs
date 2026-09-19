// Shared Data Transfer Objects (DTO) for Request/Response serialization
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatusResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterRequest {
    #[validate(length(min = 3, max = 50, message = "Username must be between 3 and 50 characters"))]
    pub username: String,
    pub full_name: Option<String>,
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 6, message = "Password must be at least 6 characters"))]
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    pub password: String,
    pub platform: Option<String>,
    pub device_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserResponse {
    pub id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub email: String,
    pub avatar_url: Option<String>,
    pub friendship_status: Option<String>,
    pub friendship_sender_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAvatarRequest {
    #[validate(length(min = 1, message = "Avatar URL must not be empty"))]
    pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarUploadResponse {
    pub avatar_url: String,
    pub file_url: String,
    pub user: UserResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub user: UserResponse,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenericResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserConversationResponse {
    pub id: Uuid,
    pub title: Option<String>,
    pub is_group: bool,
    pub peer_id: Option<Uuid>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageRequest {
    pub content: Option<String>,
    pub message_type: Option<String>,
    pub reply_to_id: Option<Uuid>,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub duration: Option<i32>,
    #[serde(alias = "media_items", alias = "medias")]
    pub media_items: Option<Vec<models::MediaItem>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageRequest {
    #[validate(length(min = 1, message = "Content cannot be empty"))]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddReactionRequest {
    #[validate(length(min = 1, max = 32, message = "Reaction cannot be empty"))]
    pub reaction: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MessageReactionResponse {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub reaction: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePostRequest {
    pub content: Option<String>,
    #[serde(alias = "media_items", alias = "medias")]
    pub media_items: Option<Vec<models::MediaItem>>,
    pub visibility: Option<String>, // 'public', 'friends', 'private'
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdatePostRequest {
    pub content: Option<String>,
    #[serde(alias = "media_items", alias = "medias")]
    pub media_items: Option<Vec<models::MediaItem>>,
    pub visibility: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostAuthorResponse {
    pub id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostResponse {
    pub id: Uuid,
    pub author: PostAuthorResponse,
    pub content: String,
    pub media_items: Vec<models::MediaItem>,
    pub visibility: String,
    pub like_count: i64,
    pub comment_count: i64,
    pub user_has_liked: bool,
    pub user_reaction: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddPostReactionRequest {
    #[validate(length(min = 1, max = 32, message = "Reaction cannot be empty"))]
    pub reaction: Option<String>, // defaults to "like"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostReactionDetailResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub user: PostAuthorResponse,
    pub reaction: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateCommentRequest {
    #[validate(length(min = 1, message = "Comment content cannot be empty"))]
    pub content: String,
    pub parent_comment_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author: PostAuthorResponse,
    pub parent_comment_id: Option<Uuid>,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

