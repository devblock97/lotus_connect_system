// Shared Internal Domain Events
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DomainEvent {
    UserRegistered {
        user_id: Uuid,
        username: String,
        email: String,
        registered_at: DateTime<Utc>,
    },
    MessageCreated {
        message_id: Uuid,
        conversation_id: Uuid,
        sender_id: Uuid,
        created_at: DateTime<Utc>,
    },
}
