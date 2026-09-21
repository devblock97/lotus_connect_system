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
    UserConversationResponse, SendMessageRequest, EditMessageRequest,
    AddReactionRequest, MessageReactionResponse,
    UpdateAvatarRequest, AvatarUploadResponse,
    CreatePostRequest, UpdatePostRequest, PostResponse, AddPostReactionRequest,
    PostReactionDetailResponse, CreateCommentRequest, CommentResponse, FeedQuery
};
use crate::AppState;
use crate::ws::types::WsMessage;

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

    // Trigger async push notification to the friend
    let sender_name = match state.user_service.get_user_by_id(claims.sub).await {
        Ok(u) => u.full_name.filter(|n| !n.trim().is_empty()).unwrap_or(u.username),
        Err(_) => "Someone".to_string(),
    };

    let notif_title = "New Friend Request".to_string();
    let notif_body = format!("{} sent you a friend request.", sender_name);
    let notif_data = serde_json::json!({
        "type": "friend_request",
        "senderId": claims.sub
    });

    let notif_service = state.notification_service.clone();
    let friend_id = friendship.friend_id;
    tokio::spawn(async move {
        let _ = notif_service.send_notification(friend_id, &notif_title, &notif_body, Some(notif_data)).await;
    });

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

    // Trigger async push notification to the friend
    let sender_name = match state.user_service.get_user_by_id(claims.sub).await {
        Ok(u) => u.full_name.filter(|n| !n.trim().is_empty()).unwrap_or(u.username),
        Err(_) => "Someone".to_string(),
    };

    let notif_title = "Friend Request Accepted".to_string();
    let notif_body = format!("{} accepted your friend request.", sender_name);
    let notif_data = serde_json::json!({
        "type": "friend_accept",
        "senderId": claims.sub
    });

    let notif_service = state.notification_service.clone();
    let friend_id = payload.friend_id;
    tokio::spawn(async move {
        let _ = notif_service.send_notification(friend_id, &notif_title, &notif_body, Some(notif_data)).await;
    });

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

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct DeleteFriendRequest {
    pub friend_id: Uuid,
}

pub async fn delete_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(friend_id): Path<Uuid>,
) -> Result<Json<GenericResponse>> {
    state.user_service.delete_friend(claims.sub, friend_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Friend removed successfully".to_string(),
    }))
}

pub async fn remove_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DeleteFriendRequest>,
) -> Result<Json<GenericResponse>> {
    state.user_service.delete_friend(claims.sub, payload.friend_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Friend removed successfully".to_string(),
    }))
}

pub async fn get_me_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<UserResponse>> {
    let user = state.user_service.get_user_by_id(claims.sub).await?;
    Ok(Json(UserResponse {
        id: user.id,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        avatar_url: user.avatar_url,
        friendship_status: None,
        friendship_sender_id: None,
    }))
}

pub async fn upload_avatar_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<AvatarUploadResponse>> {
    while let Some(field) = multipart.next_field().await.map_err(|err| AppError::Validation(err.to_string()))? {
        let file_name = field.file_name().unwrap_or("avatar.jpg").to_string();

        let ext = std::path::Path::new(&file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        let content_type = field.content_type().unwrap_or("").to_string();

        let is_image = match ext.as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "gif" => true,
            _ => content_type.starts_with("image/"),
        };

        if !is_image {
            return Err(AppError::Validation("Only image files (jpg, jpeg, png, webp, gif) are supported for avatars".to_string()));
        }

        let data = field.bytes().await.map_err(|err| AppError::Validation(err.to_string()))?.to_vec();
        
        if data.is_empty() {
            return Err(AppError::Validation("Avatar file cannot be empty".to_string()));
        }
        if data.len() > 10 * 1024 * 1024 {
            return Err(AppError::Validation("Avatar image size exceeds the maximum allowed limit of 10MB".to_string()));
        }

        let mut file_url = state.storage_provider.upload_file(&file_name, data).await?;
        if !file_url.starts_with("http://") && !file_url.starts_with("https://") {
            let base_name = std::path::Path::new(&file_url)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_name);
            
            let host = headers.get(axum::http::header::HOST)
                .and_then(|val| val.to_str().ok())
                .unwrap_or("localhost:8080");
                
            let proto = headers.get("x-forwarded-proto")
                .and_then(|val| val.to_str().ok())
                .unwrap_or("http");
                
            file_url = format!("{}://{}/uploads/{}", proto, host, base_name);
        }

        let updated_user = state.user_service.update_avatar(claims.sub, &file_url).await?;

        let user_response = UserResponse {
            id: updated_user.id,
            username: updated_user.username,
            full_name: updated_user.full_name,
            email: updated_user.email,
            avatar_url: updated_user.avatar_url,
            friendship_status: None,
            friendship_sender_id: None,
        };

        return Ok(Json(AvatarUploadResponse {
            avatar_url: file_url.clone(),
            file_url,
            user: user_response,
        }));
    }

    Err(AppError::Validation("No avatar file provided in multipart form data".to_string()))
}

pub async fn update_avatar_url_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateAvatarRequest>,
) -> Result<Json<AvatarUploadResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let updated_user = state.user_service.update_avatar(claims.sub, &payload.avatar_url).await?;

    let user_response = UserResponse {
        id: updated_user.id,
        username: updated_user.username,
        full_name: updated_user.full_name,
        email: updated_user.email,
        avatar_url: updated_user.avatar_url,
        friendship_status: None,
        friendship_sender_id: None,
    };

    Ok(Json(AvatarUploadResponse {
        avatar_url: payload.avatar_url.clone(),
        file_url: payload.avatar_url,
        user: user_response,
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
            avatar_url: u.avatar_url,
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
            avatar_url: u.avatar_url,
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
            avatar_url: u.avatar_url,
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
) -> Result<Json<UserConversationResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let conv = state.chat_service.create_private_chat(claims.sub, payload.friend_id).await?;

    let mut title = conv.title.clone();
    if title.is_none() || title.as_deref().unwrap_or("").trim().is_empty() {
        if let Ok(peer_user) = state.user_service.get_user_by_id(payload.friend_id).await {
            let display_name = peer_user
                .full_name
                .filter(|n| !n.trim().is_empty())
                .unwrap_or(peer_user.username);
            title = Some(display_name);
        }
    }

    Ok(Json(UserConversationResponse {
        id: conv.id,
        title,
        is_group: conv.is_group,
        peer_id: Some(payload.friend_id),
        created_at: conv.created_at,
        updated_at: conv.updated_at,
    }))
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

#[derive(Debug, serde::Deserialize)]
pub struct GetMessagesQuery {
    #[serde(alias = "cursor_id", alias = "cursorId")]
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
        let mut title = conv.title;
        if !conv.is_group {
            let members = state.chat_service.get_conversation_members(conv.id).await?;
            peer_id = members.into_iter().find(|&id| id != claims.sub);
            let target_user_id = peer_id.unwrap_or(claims.sub);
            if title.is_none() || title.as_deref().unwrap_or("").trim().is_empty() {
                if let Ok(peer_user) = state.user_service.get_user_by_id(target_user_id).await {
                    let display_name = peer_user
                        .full_name
                        .filter(|n| !n.trim().is_empty())
                        .unwrap_or(peer_user.username);
                    title = Some(display_name);
                } else {
                    tracing::warn!("Failed to fetch user info for peer_id {}", target_user_id);
                }
            }
        }
        responses.push(UserConversationResponse {
            id: conv.id,
            title,
            is_group: conv.is_group,
            peer_id,
            created_at: conv.created_at,
            updated_at: conv.updated_at,
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
    let limit = query.limit.unwrap_or(25).clamp(1, 100);
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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UploadedFileItem {
    pub url: String,
    pub thumbnail_url: Option<String>,
    pub file_name: String,
    pub file_size: i64,
    pub mime_type: Option<String>,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MultipleUploadResponse {
    pub files: Vec<UploadedFileItem>,
    pub file_urls: Vec<String>,
}

pub async fn upload_file_handler(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>> {
    if let Some(field) = multipart.next_field().await.map_err(|err| AppError::Validation(err.to_string()))? {
        let file_name = field.file_name().unwrap_or("file").to_string();
        let data = field.bytes().await.map_err(|err| AppError::Validation(err.to_string()))?.to_vec();
        
        let mut file_url = state.storage_provider.upload_file(&file_name, data).await?;
        if !file_url.starts_with("http://") && !file_url.starts_with("https://") {
            let base_name = std::path::Path::new(&file_url)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_name);
            
            let host = headers.get(axum::http::header::HOST)
                .and_then(|val| val.to_str().ok())
                .unwrap_or("localhost:8080");
                
            let proto = headers.get("x-forwarded-proto")
                .and_then(|val| val.to_str().ok())
                .unwrap_or("http");
                
            file_url = format!("{}://{}/uploads/{}", proto, host, base_name);
        }
        return Ok(Json(UploadResponse { file_url }));
    }
    Err(AppError::Validation("No file provided".to_string()))
}

pub async fn upload_multiple_files_handler(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    headers: axum::http::HeaderMap,
    mut multipart: Multipart,
) -> Result<Json<MultipleUploadResponse>> {
    let mut files = Vec::new();
    let mut file_urls = Vec::new();

    let host = headers.get(axum::http::header::HOST)
        .and_then(|val| val.to_str().ok())
        .unwrap_or("localhost:8080");
        
    let proto = headers.get("x-forwarded-proto")
        .and_then(|val| val.to_str().ok())
        .unwrap_or("http");

    while let Some(field) = multipart.next_field().await.map_err(|err| AppError::Validation(err.to_string()))? {
        let file_name = field.file_name().unwrap_or("file").to_string();
        let mime_type = field.content_type().map(|s| s.to_string()).or_else(|| {
            let ext = std::path::Path::new(&file_name).extension()?.to_str()?;
            match ext.to_lowercase().as_str() {
                "mov" => Some("video/quicktime".to_string()),
                "mp4" => Some("video/mp4".to_string()),
                "m4a" => Some("audio/m4a".to_string()),
                "aac" => Some("audio/aac".to_string()),
                "mp3" => Some("audio/mpeg".to_string()),
                "jpg" | "jpeg" => Some("image/jpeg".to_string()),
                "png" => Some("image/png".to_string()),
                "gif" => Some("image/gif".to_string()),
                "webp" => Some("image/webp".to_string()),
                _ => None,
            }
        });
        let data = field.bytes().await.map_err(|err| AppError::Validation(err.to_string()))?.to_vec();
        let file_size = data.len() as i64;
        
        let mut file_url = state.storage_provider.upload_file(&file_name, data).await?;
        if !file_url.starts_with("http://") && !file_url.starts_with("https://") {
            let base_name = std::path::Path::new(&file_url)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(&file_name);
                
            file_url = format!("{}://{}/uploads/{}", proto, host, base_name);
        }

        file_urls.push(file_url.clone());
        files.push(UploadedFileItem {
            url: file_url,
            thumbnail_url: None,
            file_name,
            file_size,
            mime_type,
        });
    }

    if files.is_empty() {
        return Err(AppError::Validation("No files provided".to_string()));
    }

    Ok(Json(MultipleUploadResponse { files, file_urls }))
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

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UnregisterDeviceRequest {
    pub token: String,
}

pub async fn unregister_device_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UnregisterDeviceRequest>,
) -> Result<Json<GenericResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    state.notification_service.unregister_device(claims.sub, &payload.token).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Device unregistered successfully".to_string(),
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

pub async fn mark_notification_read_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<GenericResponse>> {
    state.notification_service.mark_read(claims.sub, notification_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Notification marked as read".to_string(),
    }))
}

pub async fn delete_notification_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(notification_id): Path<Uuid>,
) -> Result<Json<GenericResponse>> {
    state.notification_service.delete_notification(claims.sub, notification_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Notification deleted successfully".to_string(),
    }))
}

pub async fn send_message_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(conversation_id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<models::Message>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    
    let content_text = payload.content.clone().unwrap_or_default();
    let message = state.chat_service.send_message(
        claims.sub,
        conversation_id,
        payload,
    ).await?;

    // Broadcast to other connected WebSocket clients so they get it in real-time
    let members = state.chat_service.get_conversation_members(conversation_id).await?;
    state.ws_manager.broadcast_to_users(&members, WsMessage {
        event: "chat:message".to_string(),
        payload: serde_json::to_value(&message).unwrap_or(serde_json::Value::Null),
    }).await;

    // Broadcast new notification trigger
    let sender_name = match state.user_service.get_user_by_id(claims.sub).await {
        Ok(u) => u.full_name.unwrap_or(u.username),
        Err(_) => "Someone".to_string(),
    };
    for member_id in &members {
        if *member_id != claims.sub {
            if !state.ws_manager.is_viewing_conversation(*member_id, conversation_id).await {
                let title = format!("New message from {}", sender_name);
                let body = if !content_text.is_empty() { content_text.clone() } else { format!("[{}]", message.message_type) };
                let payload_data = serde_json::json!({
                    "type": "chat",
                    "conversationId": conversation_id,
                    "messageId": message.id,
                });
                let _ = state.notification_service.send_notification(
                    *member_id,
                    &title,
                    &body,
                    Some(payload_data.clone())
                ).await;

                state.ws_manager.send_to_user(
                    *member_id,
                    WsMessage {
                        event: "notification:new".to_string(),
                        payload: serde_json::json!({
                            "id": uuid::Uuid::now_v7(),
                            "userId": *member_id,
                            "title": title,
                            "body": body,
                            "data": payload_data,
                            "isRead": false,
                            "createdAt": chrono::Utc::now(),
                        }),
                    }
                ).await;
            }
        }
    }

    Ok(Json(message))
}

pub async fn add_reaction_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<AddReactionRequest>,
) -> Result<Json<MessageReactionResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    
    let reaction_model = state.chat_service.add_reaction(claims.sub, message_id, &payload.reaction).await?;
    
    if let Ok(msg) = state.chat_service.get_message_by_id(claims.sub, message_id).await {
        if let Ok(members) = state.chat_service.get_conversation_members(msg.conversation_id).await {
            state.ws_manager.broadcast_to_users(&members, WsMessage {
                event: "chat:reaction_add".to_string(),
                payload: serde_json::json!({
                    "messageId": message_id,
                    "conversationId": msg.conversation_id,
                    "userId": claims.sub,
                    "reaction": payload.reaction,
                }),
            }).await;
        }
    }

    Ok(Json(MessageReactionResponse {
        message_id: reaction_model.message_id,
        user_id: reaction_model.user_id,
        reaction: reaction_model.reaction,
        created_at: reaction_model.created_at,
    }))
}

pub async fn remove_reaction_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((message_id, reaction)): Path<(Uuid, String)>,
) -> Result<Json<GenericResponse>> {
    state.chat_service.remove_reaction(claims.sub, message_id, &reaction).await?;
    
    if let Ok(msg) = state.chat_service.get_message_by_id(claims.sub, message_id).await {
        if let Ok(members) = state.chat_service.get_conversation_members(msg.conversation_id).await {
            state.ws_manager.broadcast_to_users(&members, WsMessage {
                event: "chat:reaction_remove".to_string(),
                payload: serde_json::json!({
                    "messageId": message_id,
                    "conversationId": msg.conversation_id,
                    "userId": claims.sub,
                    "reaction": reaction,
                }),
            }).await;
        }
    }

    Ok(Json(GenericResponse {
        success: true,
        message: "Reaction removed".to_string(),
    }))
}

pub async fn get_reactions_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<Vec<MessageReactionResponse>>> {
    let reactions = state.chat_service.get_message_reactions(claims.sub, message_id).await?;
    let response = reactions.into_iter().map(|r| MessageReactionResponse {
        message_id: r.message_id,
        user_id: r.user_id,
        reaction: r.reaction,
        created_at: r.created_at,
    }).collect();

    Ok(Json(response))
}

pub async fn edit_message_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(message_id): Path<Uuid>,
    Json(payload): Json<EditMessageRequest>,
) -> Result<Json<models::Message>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    
    let message = state.chat_service.edit_message(
        claims.sub,
        message_id,
        &payload.content,
    ).await?;

    // Broadcast the edit update to active chat participants
    let members = state.chat_service.get_conversation_members(message.conversation_id).await?;
    state.ws_manager.broadcast_to_users(&members, WsMessage {
        event: "chat:edit".to_string(),
        payload: serde_json::json!({
            "messageId": message_id,
            "content": message.content.clone(),
            "isEdited": true,
        }),
    }).await;

    Ok(Json(message))
}

pub async fn delete_message_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(message_id): Path<Uuid>,
) -> Result<Json<GenericResponse>> {
    let message = state.chat_service.get_message_by_id(claims.sub, message_id).await?;
    let conversation_id = message.conversation_id;

    state.chat_service.delete_message(claims.sub, message_id).await?;

    // Broadcast delete to active chat participants
    let members = state.chat_service.get_conversation_members(conversation_id).await?;
    state.ws_manager.broadcast_to_users(&members, WsMessage {
        event: "chat:delete".to_string(),
        payload: serde_json::json!({
            "messageId": message_id,
            "conversationId": conversation_id,
        }),
    }).await;

    Ok(Json(GenericResponse {
        success: true,
        message: "Message deleted successfully".to_string(),
    }))
}

// ==================== FEED & POST HANDLERS ====================

pub async fn create_post_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<Json<PostResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let post = state.feed_service.create_post(claims.sub, payload).await?;
    Ok(Json(post))
}

pub async fn get_post_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<PostResponse>> {
    let post = state.feed_service.get_post(claims.sub, post_id).await?;
    Ok(Json(post))
}

pub async fn update_post_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<UpdatePostRequest>,
) -> Result<Json<PostResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let post = state.feed_service.update_post(claims.sub, post_id, payload).await?;
    Ok(Json(post))
}

pub async fn delete_post_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<GenericResponse>> {
    state.feed_service.delete_post(claims.sub, post_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Post deleted successfully".to_string(),
    }))
}

pub async fn get_home_feed_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<Vec<PostResponse>>> {
    let posts = state.feed_service.get_home_feed(claims.sub, query.cursor, query.limit).await?;
    Ok(Json(posts))
}

pub async fn get_explore_feed_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<Vec<PostResponse>>> {
    let posts = state.feed_service.get_explore_feed(claims.sub, query.cursor, query.limit).await?;
    Ok(Json(posts))
}

pub async fn get_user_posts_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<FeedQuery>,
) -> Result<Json<Vec<PostResponse>>> {
    let posts = state.feed_service.get_user_posts(claims.sub, user_id, query.cursor, query.limit).await?;
    Ok(Json(posts))
}

pub async fn add_post_reaction_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<AddPostReactionRequest>,
) -> Result<Json<PostReactionDetailResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let reaction = payload.reaction.unwrap_or_else(|| "like".to_string());
    let res = state.feed_service.add_reaction(claims.sub, post_id, &reaction).await?;

    // Push notification to post author asynchronously
    if let Ok(post) = state.feed_service.get_post(claims.sub, post_id).await {
        if post.author.id != claims.sub {
            let sender_name = match state.user_service.get_user_by_id(claims.sub).await {
                Ok(u) => u.full_name.filter(|n| !n.trim().is_empty()).unwrap_or(u.username),
                Err(_) => "Someone".to_string(),
            };
            let title = "New Reaction".to_string();
            let body = format!("{} reacted {} to your post", sender_name, reaction);
            let notif_data = serde_json::json!({
                "type": "post_reaction",
                "postId": post_id,
                "userId": claims.sub,
                "reaction": reaction,
            });
            let notif_service = state.notification_service.clone();
            let author_id = post.author.id;
            tokio::spawn(async move {
                let _ = notif_service.send_notification(author_id, &title, &body, Some(notif_data)).await;
            });
        }
    }

    Ok(Json(res))
}

pub async fn remove_post_reaction_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<GenericResponse>> {
    state.feed_service.remove_reaction(claims.sub, post_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Reaction removed successfully".to_string(),
    }))
}

pub async fn get_post_reactions_handler(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<Vec<PostReactionDetailResponse>>> {
    let reactions = state.feed_service.get_post_reactions(post_id).await?;
    Ok(Json(reactions))
}

pub async fn create_comment_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<Json<CommentResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let comment = state.feed_service.add_comment(claims.sub, post_id, payload).await?;

    // Push notification to post author asynchronously
    if let Ok(post) = state.feed_service.get_post(claims.sub, post_id).await {
        if post.author.id != claims.sub {
            let sender_name = match state.user_service.get_user_by_id(claims.sub).await {
                Ok(u) => u.full_name.filter(|n| !n.trim().is_empty()).unwrap_or(u.username),
                Err(_) => "Someone".to_string(),
            };
            let title = "New Comment".to_string();
            let preview = if comment.content.len() > 60 {
                format!("{}...", &comment.content[..60])
            } else {
                comment.content.clone()
            };
            let body = format!("{}: {}", sender_name, preview);
            let notif_data = serde_json::json!({
                "type": "post_comment",
                "postId": post_id,
                "commentId": comment.id,
                "userId": claims.sub,
            });
            let notif_service = state.notification_service.clone();
            let author_id = post.author.id;
            tokio::spawn(async move {
                let _ = notif_service.send_notification(author_id, &title, &body, Some(notif_data)).await;
            });
        }
    }

    Ok(Json(comment))
}

pub async fn get_post_comments_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<Vec<CommentResponse>>> {
    let comments = state.feed_service.get_post_comments(claims.sub, post_id).await?;
    Ok(Json(comments))
}

pub async fn delete_comment_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path((post_id, comment_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<GenericResponse>> {
    state.feed_service.delete_comment(claims.sub, post_id, comment_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Comment deleted successfully".to_string(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_messages_query_defaults_and_limits() {
        let query: GetMessagesQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.cursor, None);
        assert_eq!(query.limit, None);
        let effective_limit = query.limit.unwrap_or(25).clamp(1, 100);
        assert_eq!(effective_limit, 25);

        let query: GetMessagesQuery = serde_json::from_str(r#"{"limit": 500}"#).unwrap();
        let effective_limit = query.limit.unwrap_or(25).clamp(1, 100);
        assert_eq!(effective_limit, 100);

        let query: GetMessagesQuery = serde_json::from_str(r#"{"limit": 0}"#).unwrap();
        let effective_limit = query.limit.unwrap_or(25).clamp(1, 100);
        assert_eq!(effective_limit, 1);

        let cursor_uuid = Uuid::now_v7();
        let query: GetMessagesQuery = serde_json::from_str(&format!(r#"{{"cursor": "{}"}}"#, cursor_uuid)).unwrap();
        assert_eq!(query.cursor, Some(cursor_uuid));

        let query: GetMessagesQuery = serde_json::from_str(&format!(r#"{{"cursor_id": "{}"}}"#, cursor_uuid)).unwrap();
        assert_eq!(query.cursor, Some(cursor_uuid));
    }

    #[test]
    fn test_feed_query_deserialization_and_limits() {
        let query: FeedQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.cursor, None);
        assert_eq!(query.limit, None);
        let effective_limit = query.limit.unwrap_or(20).clamp(1, 50);
        assert_eq!(effective_limit, 20);

        let query: FeedQuery = serde_json::from_str(r#"{"limit": 100}"#).unwrap();
        let effective_limit = query.limit.unwrap_or(20).clamp(1, 50);
        assert_eq!(effective_limit, 50);

        let query: FeedQuery = serde_json::from_str(r#"{"limit": 0}"#).unwrap();
        let effective_limit = query.limit.unwrap_or(20).clamp(1, 50);
        assert_eq!(effective_limit, 1);

        let cursor_uuid = Uuid::now_v7();
        let query: FeedQuery = serde_json::from_str(&format!(r#"{{"cursor": "{}"}}"#, cursor_uuid)).unwrap();
        assert_eq!(query.cursor, Some(cursor_uuid));
    }

    #[test]
    fn test_create_post_request_deserialization() {
        let json_str = r#"{
            "content": "Exploring the mountains! 🏔️",
            "mediaItems": [
                {
                    "url": "http://localhost:8080/uploads/mountain.jpg",
                    "mimeType": "image/jpeg",
                    "width": 1080,
                    "height": 1350
                }
            ],
            "visibility": "public"
        }"#;

        let req: CreatePostRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.content, Some("Exploring the mountains! 🏔️".to_string()));
        assert_eq!(req.visibility, Some("public".to_string()));
        let media = req.media_items.unwrap();
        assert_eq!(media.len(), 1);
        assert_eq!(media[0].url, "http://localhost:8080/uploads/mountain.jpg");
        assert_eq!(media[0].width, Some(1080));
    }

    #[test]
    fn test_create_comment_request_validation() {
        let valid_comment = CreateCommentRequest {
            content: "Awesome picture!".to_string(),
            parent_comment_id: None,
        };
        assert!(valid_comment.validate().is_ok());

        let invalid_comment = CreateCommentRequest {
            content: "".to_string(),
            parent_comment_id: None,
        };
        assert!(invalid_comment.validate().is_err());
    }

    #[test]
    fn test_add_post_reaction_request_validation() {
        let default_reaction = AddPostReactionRequest { reaction: None };
        assert!(default_reaction.validate().is_ok());

        let valid_reaction = AddPostReactionRequest {
            reaction: Some("love".to_string()),
        };
        assert!(valid_reaction.validate().is_ok());

        let invalid_reaction = AddPostReactionRequest {
            reaction: Some("".to_string()),
        };
        assert!(invalid_reaction.validate().is_err());
    }
}

