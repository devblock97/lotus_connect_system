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
#[validate(schema(function = "validate_create_comment"))]
pub struct CreateCommentRequest {
    #[serde(default)]
    pub content: String,
    pub parent_comment_id: Option<Uuid>,
    pub media_url: Option<String>,
}

fn validate_create_comment(req: &CreateCommentRequest) -> std::result::Result<(), validator::ValidationError> {
    if req.content.trim().is_empty() && req.media_url.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err(validator::ValidationError::new("content_or_media_required"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[validate(schema(function = "validate_update_comment"))]
pub struct UpdateCommentRequest {
    #[serde(default)]
    pub content: String,
    pub media_url: Option<String>,
}

fn validate_update_comment(req: &UpdateCommentRequest) -> std::result::Result<(), validator::ValidationError> {
    if req.content.trim().is_empty() && req.media_url.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        return Err(validator::ValidationError::new("content_or_media_required"));
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
    pub sort: Option<String>, // "newest", "oldest", "popular"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author: PostAuthorResponse,
    pub parent_comment_id: Option<Uuid>,
    pub content: String,
    pub media_url: Option<String>,
    pub like_count: i64,
    pub reply_count: i64,
    pub is_pinned: bool,
    pub pinned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub user_has_liked: bool,
    pub user_reaction: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddCommentReactionRequest {
    #[validate(length(min = 1, max = 32, message = "Reaction cannot be empty"))]
    pub reaction: Option<String>, // defaults to "like"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommentReactionResponse {
    pub id: Uuid,
    pub comment_id: Uuid,
    pub user: PostAuthorResponse,
    pub reaction: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateStoryRequest {
    pub media_type: Option<String>, // 'image', 'video', 'text' (default: 'image')
    #[validate(length(min = 1, message = "Media URL must not be empty"))]
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub caption: Option<String>,
    pub duration: Option<f64>, // defaults to 5.0
    pub visibility: Option<String>, // 'public', 'friends', 'close_friends' (default: 'friends')
    pub background_color: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryResponse {
    pub id: Uuid,
    pub author: PostAuthorResponse,
    pub media_type: String,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub caption: Option<String>,
    pub duration: f64,
    pub visibility: String,
    pub background_color: Option<String>,
    pub metadata: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub view_count: i64,
    pub has_viewed: bool,
    pub viewer_reaction: Option<String>,
    pub is_close_friend: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryTrayItemResponse {
    pub user: PostAuthorResponse,
    pub stories: Vec<StoryResponse>,
    pub has_unseen: bool,
    pub total_stories: usize,
    pub latest_story_created_at: chrono::DateTime<chrono::Utc>,
    pub has_close_friends_story: bool,
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryViewerResponse {
    pub id: Uuid,
    pub viewer: PostAuthorResponse,
    pub viewed_at: chrono::DateTime<chrono::Utc>,
    pub reaction: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddStoryReactionRequest {
    #[validate(length(min = 1, max = 32, message = "Reaction cannot be empty"))]
    pub reaction: Option<String>, // default: "❤️"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryReactionResponse {
    pub id: Uuid,
    pub story_id: Uuid,
    pub user: PostAuthorResponse,
    pub reaction: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct StoryReplyRequest {
    #[validate(length(min = 1, message = "Reply message cannot be empty"))]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryReplyResponse {
    pub success: bool,
    pub message: String,
    pub conversation_id: Option<Uuid>,
    pub message_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddCloseFriendRequest {
    pub friend_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseFriendResponse {
    pub id: Uuid,
    pub friend: PostAuthorResponse,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ================= LIVE STREAM DTOs =================

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateLiveStreamRequest {
    #[validate(length(min = 1, max = 255, message = "Title must be between 1 and 255 characters"))]
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub privacy: Option<String>, // 'public', 'friends', 'private' (default: 'public')
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLiveStreamRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub privacy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndLiveStreamRequest {
    pub recording_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamQuery {
    pub status: Option<String>, // 'live', 'ended', 'all' (default: 'live')
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamResponse {
    pub id: Uuid,
    pub host: PostAuthorResponse,
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub status: String,
    pub privacy: String,
    pub playback_url: Option<String>,
    pub channel_id: Option<String>,
    pub viewer_count: i32,
    pub peak_viewers: i32,
    pub total_views: i32,
    pub like_count: i64,
    pub pinned_comment_id: Option<Uuid>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub ended_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_seconds: i32,
    pub recording_url: Option<String>,
    pub is_host: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamDetailResponse {
    #[serde(flatten)]
    pub stream: LiveStreamResponse,
    /// Stream key and ingest URL are ONLY returned to the stream host
    pub stream_key: Option<String>,
    pub ingest_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamViewerResponse {
    pub id: Uuid,
    pub user: PostAuthorResponse,
    pub joined_at: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendLiveCommentRequest {
    #[validate(length(min = 1, max = 500, message = "Comment must be between 1 and 500 characters"))]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamCommentResponse {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub user: PostAuthorResponse,
    pub content: String,
    pub is_pinned: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendLiveReactionRequest {
    pub reaction: Option<String>, // 'heart', 'like', 'fire', 'star' (default: 'heart')
    pub count: Option<i32>, // burst count (default: 1, max: 20)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamReactionResponse {
    pub stream_id: Uuid,
    pub user: PostAuthorResponse,
    pub reaction: String,
    pub count: i32,
    pub total_likes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SendLiveGiftRequest {
    #[validate(length(min = 1, max = 64, message = "Gift type must not be empty"))]
    pub gift_type: String, // 'rose', 'heart', 'diamond', 'coffee', 'firework', 'rocket', 'crown'
    pub count: Option<i32>, // combo count (default: 1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamGiftResponse {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub sender: PostAuthorResponse,
    pub gift_type: String,
    pub gift_count: i32,
    pub coin_amount: i32,
    pub combo_count: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamLeaderboardResponse {
    pub stream_id: Uuid,
    pub supporters: Vec<LiveStreamLeaderboardItemResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamLeaderboardItemResponse {
    pub user: PostAuthorResponse,
    pub total_coins: i64,
    pub gift_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RespondLiveCohostRequest {
    pub user_id: Uuid,
    pub action: String, // 'accept', 'reject'
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiveStreamCohostResponse {
    pub id: Uuid,
    pub stream_id: Uuid,
    pub user: PostAuthorResponse,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}



