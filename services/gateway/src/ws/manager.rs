use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use axum::extract::ws::Message;

use crate::ws::types::WsMessage;

#[derive(Clone)]
pub struct WsManager {
    clients: Arc<RwLock<HashMap<Uuid, mpsc::UnboundedSender<Message>>>>,
}

impl WsManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new client connection sender
    pub async fn add_client(&self, user_id: Uuid, tx: mpsc::UnboundedSender<Message>) {
        let mut clients = self.clients.write().await;
        clients.insert(user_id, tx);
        tracing::info!("User {} connected to WebSocket. Active connections: {}", user_id, clients.len());
    }

    /// Remove a client connection by User ID
    pub async fn remove_client(&self, user_id: Uuid) {
        let mut clients = self.clients.write().await;
        clients.remove(&user_id);
        tracing::info!("User {} disconnected from WebSocket. Active connections: {}", user_id, clients.len());
    }

    /// Send a message directly to a single online user
    pub async fn send_to_user(&self, user_id: Uuid, msg: WsMessage) -> bool {
        let clients = self.clients.read().await;
        if let Some(tx) = clients.get(&user_id) {
            if let Ok(json_str) = serde_json::to_string(&msg) {
                if tx.send(Message::Text(json_str.into())).is_ok() {
                    return true;
                }
            }
        }
        false
    }

    /// Broadcast a message to a list of user IDs (e.g. members of a conversation)
    pub async fn broadcast_to_users(&self, user_ids: &[Uuid], msg: WsMessage) {
        let clients = self.clients.read().await;
        if let Ok(json_str) = serde_json::to_string(&msg) {
            for user_id in user_ids {
                if let Some(tx) = clients.get(user_id) {
                    let _ = tx.send(Message::Text(json_str.clone().into()));
                }
            }
        }
    }
}
