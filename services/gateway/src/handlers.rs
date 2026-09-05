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
    AddReactionRequest, MessageReactionResponse
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
        let mime_type = field.content_type().map(|s| s.to_string());
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
}
