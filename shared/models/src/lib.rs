// Shared database entities/models
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub full_name: Option<String>,
    pub email: String,
    pub password_hash: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Friendship {
    pub id: Uuid,
    pub user_id: Uuid,
    pub friend_id: Uuid,
    pub status: String, // 'pending', 'accepted', 'blocked'
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub revoked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserPresence {
    pub user_id: Uuid,
    pub status: String, // 'online', 'offline', 'away', 'busy'
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub title: Option<String>,
    pub is_group: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ConversationMember {
    pub conversation_id: Uuid,
    pub user_id: Uuid,
    pub role: String, // 'member', 'admin'
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MediaItem {
    pub url: String,
    #[serde(alias = "thumbnail_url")]
    pub thumbnail_url: Option<String>,
    #[serde(alias = "file_name")]
    pub file_name: Option<String>,
    #[serde(alias = "file_size")]
    pub file_size: Option<i64>,
    #[serde(alias = "mime_type")]
    pub mime_type: Option<String>,
    pub duration: Option<i32>,
    pub width: Option<i32>,
    pub height: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub message_type: String, // 'text', 'image', 'video', 'audio', 'voice', 'file', 'call_log'
    pub reply_to_id: Option<Uuid>,
    pub media_url: Option<String>,
    pub thumbnail_url: Option<String>,
    pub file_name: Option<String>,
    pub file_size: Option<i64>,
    pub mime_type: Option<String>,
    pub duration: Option<i32>,
    pub media_items: Option<sqlx::types::Json<Vec<MediaItem>>>,
    pub is_edited: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[sqlx(skip)]
    pub reactions: Option<Vec<MessageReactionGroup>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRead {
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub read_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageReaction {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub reaction: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageReactionGroup {
    pub reaction: String,
    pub count: i64,
    pub users: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Attachment {
    pub id: Uuid,
    pub message_id: Uuid,
    pub file_path: String,
    pub file_name: String,
    pub file_type: String,
    pub file_size: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Group {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Call {
    pub id: Uuid,
    pub host_id: Uuid,
    #[sqlx(default)]
    pub host_name: Option<String>,
    #[sqlx(default)]
    pub username: Option<String>,
    pub conversation_id: Option<Uuid>,
    pub channel_id: String,
    pub is_video: bool,
    pub status: String, // 'initiated', 'active', 'ended', 'missed', 'rejected'
    pub created_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CallParticipant {
    pub call_id: Uuid,
    pub user_id: Uuid,
    pub joined_at: Option<DateTime<Utc>>,
    pub left_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct CallEvent {
    pub id: Uuid,
    pub call_id: Uuid,
    pub user_id: Uuid,
    pub event_type: String, // 'mute', 'unmute', 'camera_on', ...
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub body: String,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization_schema() {
        let msg_id = Uuid::now_v7();
        let conv_id = Uuid::now_v7();
        let sender_id = Uuid::now_v7();
        let now = Utc::now();

        let msg = Message {
            id: msg_id,
            conversation_id: conv_id,
            sender_id,
            content: "Hello".to_string(),
            message_type: "text".to_string(),
            reply_to_id: None,
            media_url: None,
            thumbnail_url: None,
            file_name: None,
            file_size: None,
            mime_type: None,
            duration: None,
            media_items: None,
            is_edited: false,
            created_at: now,
            updated_at: now,
            reactions: None,
        };

        let json_val = serde_json::to_value(&msg).unwrap();
        assert_eq!(json_val["id"], msg_id.to_string());
        assert_eq!(json_val["conversation_id"], conv_id.to_string());
        assert_eq!(json_val["sender_id"], sender_id.to_string());
        assert_eq!(json_val["content"], "Hello");
        assert_eq!(json_val["message_type"], "text");
        assert!(json_val["reactions"].is_null());

        let mut msg_with_reactions = msg;
        let u1 = Uuid::now_v7();
        msg_with_reactions.reactions = Some(vec![
            MessageReactionGroup {
                reaction: "👍".to_string(),
                count: 1,
                users: vec![u1],
            },
        ]);

        let json_val2 = serde_json::to_value(&msg_with_reactions).unwrap();
        assert!(json_val2["reactions"].is_array());
        assert_eq!(json_val2["reactions"][0]["reaction"], "👍");
        assert_eq!(json_val2["reactions"][0]["count"], 1);
        assert_eq!(json_val2["reactions"][0]["users"][0], u1.to_string());
    }
}
