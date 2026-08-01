use axum::{
    extract::{State, Extension, Path, Query, Multipart},
    Json,
};
use validator::Validate;
use uuid::Uuid;

use errors::{AppError, Result};
use auth::Claims;
use dto::{
    RegisterRequest, LoginRequest, RefreshTokenRequest, 
    AuthResponse, UserResponse, TokenResponse, GenericResponse,
    UserConversationResponse
};
use crate::AppState;

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<UserResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let user = state.auth_service.register(payload).await?;
    Ok(Json(user))
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let res = state.auth_service.login(payload).await?;
    Ok(Json(res))
}

pub async fn refresh_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let res = state.auth_service.refresh_token(&payload.refresh_token).await?;
    Ok(Json(res))
}

pub async fn logout_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<GenericResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    state.auth_service.logout(&payload.refresh_token).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    }))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddFriendRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
}

pub async fn add_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AddFriendRequest>,
) -> Result<Json<models::Friendship>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let friendship = state.user_service.send_friend_request(claims.sub, &payload.username).await?;
    Ok(Json(friendship))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AcceptFriendRequest {
    pub friend_id: Uuid,
}

pub async fn accept_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AcceptFriendRequest>,
) -> Result<Json<GenericResponse>> {
    state.user_service.accept_friend_request(claims.sub, payload.friend_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Friend request accepted successfully".to_string(),
    }))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RejectFriendRequest {
    pub friend_id: Uuid,
}

pub async fn reject_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RejectFriendRequest>,
) -> Result<Json<GenericResponse>> {
    state.user_service.reject_friend_request(claims.sub, payload.friend_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Friend request rejected successfully".to_string(),
    }))
}

pub async fn list_friends_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<UserResponse>>> {
    let friends = state.user_service.get_friends_list(claims.sub).await?;
    let response: Vec<UserResponse> = friends
        .into_iter()
        .map(|u| UserResponse {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
            email: u.email,
            friendship_status: Some("accepted".to_string()),
            friendship_sender_id: None,
        })
        .collect();
    Ok(Json(response))
}

pub async fn list_friend_requests_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<UserResponse>>> {
    let requests = state.user_service.get_pending_requests(claims.sub).await?;
    let response: Vec<UserResponse> = requests
        .into_iter()
        .map(|u| UserResponse {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
            email: u.email,
            friendship_status: Some("pending".to_string()),
            friendship_sender_id: Some(u.id),
        })
        .collect();
    Ok(Json(response))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct SearchUsersQuery {
    #[validate(length(min = 1, message = "Search query cannot be empty"))]
    pub q: String,
}

pub async fn search_users_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<SearchUsersQuery>,
) -> Result<Json<Vec<UserResponse>>> {
    query.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let users = state.user_service.search_users(&query.q, claims.sub).await?;
    let response: Vec<UserResponse> = users
        .into_iter()
        .map(|(u, status, sender_id)| UserResponse {
            id: u.id,
            username: u.username,
            full_name: u.full_name,
            email: u.email,
            friendship_status: status,
            friendship_sender_id: sender_id,
        })
        .collect();
    Ok(Json(response))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreatePrivateChatRequest {
    pub friend_id: Uuid,
}

pub async fn create_private_chat_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreatePrivateChatRequest>,
) -> Result<Json<models::Conversation>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let conv = state.chat_service.create_private_chat(claims.sub, payload.friend_id).await?;
    Ok(Json(conv))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateGroupChatRequest {
    #[validate(length(min = 3, max = 50, message = "Group name must be between 3 and 50 characters"))]
    pub name: String,
    pub description: Option<String>,
    pub members: Vec<Uuid>,
}

pub async fn create_group_chat_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateGroupChatRequest>,
) -> Result<Json<models::Group>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let group = state.chat_service.create_group_chat(claims.sub, &payload.name, payload.description.as_deref(), payload.members).await?;
    Ok(Json(group))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetMessagesQuery {
    pub cursor: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn list_conversations_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<UserConversationResponse>>> {
    let conversations = state.chat_service.list_user_conversations(claims.sub).await?;
    let mut responses = Vec::new();
    for conv in conversations {
        let mut peer_id = None;
        if !conv.is_group {
            let members = state.chat_service.get_conversation_members(conv.id).await?;
            peer_id = members.into_iter().find(|&id| id != claims.sub);
        }
        responses.push(UserConversationResponse {
            id: conv.id,
            title: conv.title,
            is_group: conv.is_group,
            peer_id,
            created_at: conv.created_at,
        });
    }
    Ok(Json(responses))
}

pub async fn get_messages_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(conversation_id): Path<Uuid>,
    Query(query): Query<GetMessagesQuery>,
) -> Result<Json<Vec<models::Message>>> {
    let limit = query.limit.unwrap_or(20);
    let messages = state.chat_service.get_messages(claims.sub, conversation_id, query.cursor, limit).await?;
    Ok(Json(messages))
}

pub async fn get_calls_history_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<models::Call>>> {
    let history = state.call_service.get_user_call_history(claims.sub).await?;
    Ok(Json(history))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadResponse {
    pub file_url: String,
}

pub async fn upload_file_handler(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    if let Some(field) = multipart.next_field().await.map_err(|err| AppError::Validation(err.to_string()))? {
        let file_name = field.file_name().unwrap_or("file").to_string();
        let data = field.bytes().await.map_err(|err| AppError::Validation(err.to_string()))?.to_vec();
        
        let file_url = state.storage_provider.upload_file(&file_name, data).await?;
        return Ok(Json(UploadResponse { file_url }));
    }
    Err(AppError::Validation("No file provided".to_string()))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct RegisterDeviceRequest {
    pub token: String,
    pub platform: String, // 'android', 'ios'
}

pub async fn register_device_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RegisterDeviceRequest>,
) -> Result<Json<GenericResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    state.notification_service.register_device(claims.sub, &payload.token, &payload.platform).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Device registered successfully".to_string(),
    }))
}

pub async fn list_notifications_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<models::Notification>>> {
    let notifications = state.notification_service.get_notifications(claims.sub).await?;
    Ok(Json(notifications))
}

pub async fn mark_notifications_read_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<GenericResponse>> {
    state.notification_service.mark_all_read(claims.sub).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Notifications marked as read".to_string(),
    }))
}
