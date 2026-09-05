use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;
use chrono::Utc;

use crate::AppState;
use errors::AppError;
use crate::ws::types::WsMessage;

pub mod manager;
pub mod types;

#[derive(Deserialize)]
pub struct WsAuthQuery {
    pub token: String,
}

/// HTTP upgrade handler to negotiate WebSocket connection
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsAuthQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    match auth::verify_token(&query.token, &state.config.jwt_secret) {
        Ok(claims) => {
            ws.on_upgrade(move |socket| handle_socket(socket, claims.sub, state))
        }
        Err(err) => {
            tracing::error!("WebSocket connection rejected: invalid authentication token: {:?}", err);
            axum::http::StatusCode::UNAUTHORIZED.into_response()
        }
    }
}

/// Handle upgraded WebSocket session
async fn handle_socket(socket: WebSocket, user_id: Uuid, state: AppState) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    // 1. Register connection in the connection manager pool
    state.ws_manager.add_client(user_id, tx).await;

    // 2. Set presence status to online
    if let Err(err) = state.presence_service.set_status(user_id, "online").await {
        tracing::error!("Failed to set user {} presence to online: {:?}", user_id, err);
    }

    if let Ok(friends) = state.user_service.get_friends_list(user_id).await {
        let friend_ids: Vec<uuid::Uuid> = friends.into_iter().map(|f| f.id).collect();
        state.ws_manager.broadcast_to_users(&friend_ids, WsMessage {
            event: "presence:status".to_string(),
            payload: serde_json::json!({
                "userId": user_id,
                "isOnline": true,
                "lastSeen": chrono::Utc::now(),
            }),
        }).await;

        // Sync statuses of currently online friends to this newly connected user
        for friend_id in friend_ids {
            if state.ws_manager.is_connected(friend_id).await {
                state.ws_manager.send_to_user(user_id, WsMessage {
                    event: "presence:status".to_string(),
                    payload: serde_json::json!({
                        "userId": friend_id,
                        "isOnline": true,
                        "lastSeen": chrono::Utc::now(),
                    }),
                }).await;
            }
        }
    }

    // 3. Outbound write loop task
    let write_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // 4. Inbound read loop task
    let state_clone = state.clone();
    let read_task = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_receiver.next().await {
            match message {
                Message::Text(text) => {
                    if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                        if let Err(err) = route_ws_event(user_id, ws_msg, &state_clone).await {
                            tracing::error!("Error routing WebSocket event from user {}: {:?}", user_id, err);
                        }
                    }
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }
    });

    // 5. Wait for connection loops to end (disconnect)
    tokio::select! {
        _ = read_task => {},
        _ = write_task => {},
    }

    // 6. Cleanup connection
    state.ws_manager.remove_client(user_id).await;
    if let Err(err) = state.presence_service.set_status(user_id, "offline").await {
        tracing::error!("Failed to set user {} presence to offline: {:?}", user_id, err);
    }

    if let Ok(friends) = state.user_service.get_friends_list(user_id).await {
        let friend_ids: Vec<uuid::Uuid> = friends.into_iter().map(|f| f.id).collect();
        state.ws_manager.broadcast_to_users(&friend_ids, WsMessage {
            event: "presence:status".to_string(),
            payload: serde_json::json!({
                "userId": user_id,
                "isOnline": false,
                "lastSeen": chrono::Utc::now(),
            }),
        }).await;
    }
}

/// Routes incoming WebSocket messages to corresponding receivers
async fn route_ws_event(
    sender_id: Uuid,
    msg: WsMessage,
    state: &AppState,
) -> Result<(), AppError> {
    match msg.event.as_str() {
        // Heartbeat update
        "heartbeat" => {
            state.presence_service.heartbeat(sender_id).await?;
            Ok(())
        }
        
        // Typing status update
        "typing" => {
            if let Some(recipient_id_str) = msg.payload.get("recipientId").and_then(|v| v.as_str()) {
                if let Ok(recipient_id) = Uuid::parse_str(recipient_id_str) {
                    state.ws_manager.send_to_user(recipient_id, WsMessage {
                        event: "typing".to_string(),
                        payload: serde_json::json!({
                            "senderId": sender_id,
                            "conversationId": msg.payload.get("conversationId"),
                            "isTyping": msg.payload.get("isTyping")
                        }),
                    }).await;
                }
            }
            Ok(())
        }

        // New real-time chat message delivery
        "chat:message" => {
            let payload = msg.payload;
            let conv_id_str = payload.get("conversationId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing conversationId".to_string()))?;
            let conversation_id = Uuid::parse_str(conv_id_str)
                .map_err(|_| AppError::Validation("Invalid conversationId".to_string()))?;

            let content = payload.get("content").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing content".to_string()))?;

            let reply_to_id = payload.get("replyToId").and_then(|v| v.as_str())
                .and_then(|id_str| Uuid::parse_str(id_str).ok());

            let req = dto::SendMessageRequest {
                content: Some(content.to_string()),
                message_type: Some("text".to_string()),
                reply_to_id,
                media_url: None,
                thumbnail_url: None,
                file_name: None,
                file_size: None,
                mime_type: None,
                duration: None,
                media_items: None,
            };
            let message = state.chat_service.send_message(sender_id, conversation_id, req).await?;
            let members = state.chat_service.get_conversation_members(conversation_id).await?;

            state.ws_manager.broadcast_to_users(&members, WsMessage {
                event: "chat:message".to_string(),
                payload: serde_json::to_value(&message).unwrap_or(serde_json::Value::Null),
            }).await;

            // Fetch sender profile name for notification display
            let sender_name = match state.user_service.get_user_by_id(sender_id).await {
                Ok(u) => u.full_name.unwrap_or(u.username),
                Err(_) => "Someone".to_string(),
            };

            // Trigger notification for other members about the unread message
            for member_id in &members {
                if *member_id != sender_id {
                    // Only send notification if they are NOT currently viewing the conversation
                    if !state.ws_manager.is_viewing_conversation(*member_id, conversation_id).await {
                        let title = format!("New message from {}", sender_name);
                        let body = content.to_string();
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

                        // Also send a real-time event to the recipient so the Alerts list updates immediately
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
            
            Ok(())
        }

        // Read receipt indicator
        "chat:read" => {
            let payload = msg.payload;
            let msg_id_str = payload.get("messageId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing messageId".to_string()))?;
            let message_id = Uuid::parse_str(msg_id_str)
                .map_err(|_| AppError::Validation("Invalid messageId".to_string()))?;

            state.chat_service.read_message(sender_id, message_id).await?;

            let message = state.chat_service.get_message_by_id(sender_id, message_id).await?;
            let members = state.chat_service.get_conversation_members(message.conversation_id).await?;

            state.ws_manager.broadcast_to_users(&members, WsMessage {
                event: "chat:read".to_string(),
                payload: serde_json::json!({
                    "messageId": message_id,
                    "userId": sender_id,
                    "readAt": Utc::now(),
                }),
            }).await;

            Ok(())
        }

        // Edit/update message content
        "chat:edit" => {
            let payload = msg.payload;
            let msg_id_str = payload.get("messageId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing messageId".to_string()))?;
            let message_id = Uuid::parse_str(msg_id_str)
                .map_err(|_| AppError::Validation("Invalid messageId".to_string()))?;

            let content = payload.get("content").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing content".to_string()))?;

            let message = state.chat_service.edit_message(sender_id, message_id, content).await?;
            let members = state.chat_service.get_conversation_members(message.conversation_id).await?;

            state.ws_manager.broadcast_to_users(&members, WsMessage {
                event: "chat:edit".to_string(),
                payload: serde_json::json!({
                    "messageId": message_id,
                    "content": message.content.clone(),
                    "isEdited": true,
                }),
            }).await;

            Ok(())
        }

        // Delete message
        "chat:delete" => {
            let payload = msg.payload;
            let msg_id_str = payload.get("messageId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing messageId".to_string()))?;
            let message_id = Uuid::parse_str(msg_id_str)
                .map_err(|_| AppError::Validation("Invalid messageId".to_string()))?;

            let message = state.chat_service.get_message_by_id(sender_id, message_id).await?;
            let conversation_id = message.conversation_id;

            state.chat_service.delete_message(sender_id, message_id).await?;
            let members = state.chat_service.get_conversation_members(conversation_id).await?;

            state.ws_manager.broadcast_to_users(&members, WsMessage {
                event: "chat:delete".to_string(),
                payload: serde_json::json!({
                    "messageId": message_id,
                    "conversationId": conversation_id,
                }),
            }).await;

            Ok(())
        }

        // Active conversation focus tracking
        "chat:focus" => {
            let conversation_id = msg.payload.get("conversationId")
                .and_then(|v| v.as_str())
                .and_then(|id_str| Uuid::parse_str(id_str).ok());

            state.ws_manager.set_active_conversation(sender_id, conversation_id).await;
            Ok(())
        }

        // Call Signalling: Invite (Starts Call Record)
        "call:invite" => {
            let payload = msg.payload;
            let recipient_id_str = payload.get("recipientId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing recipientId".to_string()))?;
            let recipient_id = Uuid::parse_str(recipient_id_str)
                .map_err(|_| AppError::Validation("Invalid recipientId".to_string()))?;

            let channel_id = payload.get("channelId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing channelId".to_string()))?;

            let is_video = payload.get("isVideo").and_then(|v| v.as_bool()).unwrap_or(false);
            
            let conv_id = payload.get("conversationId").and_then(|v| v.as_str())
                .and_then(|id_str| Uuid::parse_str(id_str).ok());

            // Initialize call tracking in DB
            let call = state.call_service.start_call(sender_id, conv_id, channel_id, is_video).await?;

            // Forward invite with DB-created callId to recipient
            state.ws_manager.send_to_user(recipient_id, WsMessage {
                event: "call:invite".to_string(),
                payload: serde_json::json!({
                    "callId": call.id,
                    "senderId": sender_id,
                    "conversationId": conv_id,
                    "channelId": channel_id,
                    "isVideo": is_video,
                }),
            }).await;

            // Acknowledge the call initiation back to the sender
            state.ws_manager.send_to_user(sender_id, WsMessage {
                event: "call:invite_ack".to_string(),
                payload: serde_json::json!({
                    "callId": call.id,
                    "channelId": channel_id,
                }),
            }).await;

            Ok(())
        }

        // Call Signalling: Accept
        "call:accept" => {
            let payload = msg.payload;
            let call_id_str = payload.get("callId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing callId".to_string()))?;

            let recipient_id_str = payload.get("recipientId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing recipientId".to_string()))?;
            let recipient_id = Uuid::parse_str(recipient_id_str)
                .map_err(|_| AppError::Validation("Invalid recipientId".to_string()))?;

            if let Ok(call_id) = Uuid::parse_str(call_id_str) {
                let _ = state.call_service.join_call(call_id, sender_id).await;
            } else {
                tracing::warn!("call:accept received non-UUID callId: {}", call_id_str);
            }

            // Forward to recipient
            state.ws_manager.send_to_user(recipient_id, WsMessage {
                event: "call:accept".to_string(),
                payload: serde_json::json!({
                    "callId": call_id_str,
                    "senderId": sender_id,
                }),
            }).await;

            Ok(())
        }

        // Call Signalling: Ended
        "call:ended" => {
            let payload = msg.payload;
            let call_id_str = payload.get("callId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing callId".to_string()))?;

            let recipient_id_str = payload.get("recipientId").and_then(|v| v.as_str())
                .ok_or_else(|| AppError::Validation("Missing recipientId".to_string()))?;
            let recipient_id = Uuid::parse_str(recipient_id_str)
                .map_err(|_| AppError::Validation("Invalid recipientId".to_string()))?;

            if let Ok(call_id) = Uuid::parse_str(call_id_str) {
                // Send notification only in the event of a missed call (i.e. status is still 'initiated')
                if let Ok(Some(call)) = state.call_service.get_call(call_id).await {
                    if call.status == "initiated" {
                        let caller_name = match state.user_service.get_user_by_id(sender_id).await {
                            Ok(u) => u.full_name.unwrap_or(u.username),
                            Err(_) => "Someone".to_string(),
                        };
                        let title = "Missed Call".to_string();
                        let body = format!("You missed a call from {}", caller_name);
                        let payload_data = serde_json::json!({
                            "type": "missed_call",
                            "callId": call_id,
                            "callerId": sender_id,
                        });
                        let _ = state.notification_service.send_notification(
                            recipient_id,
                            &title,
                            &body,
                            Some(payload_data.clone())
                        ).await;

                        // Also send a real-time event to the recipient so the Alerts list updates immediately
                        state.ws_manager.send_to_user(
                            recipient_id,
                            WsMessage {
                                event: "notification:new".to_string(),
                                payload: serde_json::json!({
                                    "id": uuid::Uuid::now_v7(),
                                    "userId": recipient_id,
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

                let _ = state.call_service.leave_call(call_id, sender_id).await;
                let _ = state.call_service.end_call(call_id).await;
            } else {
                tracing::warn!("call:ended received non-UUID callId: {}", call_id_str);
            }

            // Forward to recipient
            state.ws_manager.send_to_user(recipient_id, WsMessage {
                event: "call:ended".to_string(),
                payload: serde_json::json!({
                    "callId": call_id_str,
                    "senderId": sender_id,
                }),
            }).await;

            Ok(())
        }

        // General WebRTC signaling events (Offer, Answer, IceCandidate, mute/unmute, etc.)
        event if event.starts_with("signaling:") || event.starts_with("call:") => {
            if let Some(recipient_id_str) = msg.payload.get("recipientId").and_then(|v| v.as_str()) {
                if let Ok(recipient_id) = Uuid::parse_str(recipient_id_str) {
                    let mut payload = msg.payload.clone();
                    if let Some(obj) = payload.as_object_mut() {
                        obj.insert("senderId".to_string(), serde_json::json!(sender_id));
                    }
                    state.ws_manager.send_to_user(recipient_id, WsMessage {
                        event: event.to_string(),
                        payload,
                    }).await;
                }
            }
            Ok(())
        }

        _ => {
            tracing::warn!("Unhandled WebSocket event from {}: {}", sender_id, msg.event);
            Ok(())
        }
    }
}
