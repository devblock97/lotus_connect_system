use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;
use axum::extract::ws::Message;

use crate::ws::types::WsMessage;

#[derive(Clone)]
pub struct WsManager {
    clients: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, mpsc::UnboundedSender<Message>>>>>,
    active_conversations: Arc<RwLock<HashMap<Uuid, Uuid>>>,
    livestream_rooms: Arc<RwLock<HashMap<Uuid, HashSet<Uuid>>>>,
    user_livestreams: Arc<RwLock<HashMap<Uuid, HashSet<Uuid>>>>,
}

impl WsManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            active_conversations: Arc::new(RwLock::new(HashMap::new())),
            livestream_rooms: Arc::new(RwLock::new(HashMap::new())),
            user_livestreams: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a new client connection sender by User ID and Connection ID
    pub async fn add_client(&self, user_id: Uuid, conn_id: Uuid, tx: mpsc::UnboundedSender<Message>) {
        let mut clients = self.clients.write().await;
        clients.entry(user_id).or_default().insert(conn_id, tx);
        let total_conns: usize = clients.values().map(|c| c.len()).sum();
        tracing::info!("User {} connected to WebSocket (conn {}). Active users: {}, total conns: {}", user_id, conn_id, clients.len(), total_conns);
    }

    /// Remove a client connection by User ID and Connection ID
    pub async fn remove_client(&self, user_id: Uuid, conn_id: Uuid) {
        let mut should_cleanup = false;
        {
            let mut clients = self.clients.write().await;
            if let Some(user_conns) = clients.get_mut(&user_id) {
                user_conns.remove(&conn_id);
                if user_conns.is_empty() {
                    clients.remove(&user_id);
                    should_cleanup = true;
                }
            }
            let total_conns: usize = clients.values().map(|c| c.len()).sum();
            tracing::info!("User {} disconnected from WebSocket (conn {}). Active users: {}, total conns: {}", user_id, conn_id, clients.len(), total_conns);
        }

        if should_cleanup {
            let mut active = self.active_conversations.write().await;
            active.remove(&user_id);
            self.leave_all_livestreams(user_id).await;
        }
    }

    /// Set the active conversation for a user
    pub async fn set_active_conversation(&self, user_id: Uuid, conversation_id: Option<Uuid>) {
        let mut active = self.active_conversations.write().await;
        if let Some(conv_id) = conversation_id {
            active.insert(user_id, conv_id);
        } else {
            active.remove(&user_id);
        }
    }

    /// Check if a user is actively looking at a conversation
    pub async fn is_viewing_conversation(&self, user_id: Uuid, conversation_id: Uuid) -> bool {
        let active = self.active_conversations.read().await;
        if let Some(active_conv_id) = active.get(&user_id) {
            *active_conv_id == conversation_id
        } else {
            false
        }
    }

    /// Send a message directly to a single online user
    pub async fn send_to_user(&self, user_id: Uuid, msg: WsMessage) -> bool {
        self.send_to_user_except(user_id, None, msg).await
    }

    /// Send a message directly to a user, optionally excluding a specific connection (e.g. sender's own device)
    pub async fn send_to_user_except(&self, user_id: Uuid, exclude_conn_id: Option<Uuid>, msg: WsMessage) -> bool {
        let clients = self.clients.read().await;
        if let Some(user_conns) = clients.get(&user_id) {
            if let Ok(json_str) = serde_json::to_string(&msg) {
                let mut sent = false;
                for (cid, tx) in user_conns {
                    if let Some(exclude) = exclude_conn_id {
                        if *cid == exclude {
                            continue;
                        }
                    }
                    if tx.send(Message::Text(json_str.clone().into())).is_ok() {
                        sent = true;
                    }
                }
                return sent;
            }
        }
        false
    }

    /// Check if a user is currently connected to WebSocket
    pub async fn is_connected(&self, user_id: Uuid) -> bool {
        let clients = self.clients.read().await;
        clients.get(&user_id).map(|c| !c.is_empty()).unwrap_or(false)
    }

    /// Broadcast a message to a list of user IDs (e.g. members of a conversation)
    pub async fn broadcast_to_users(&self, user_ids: &[Uuid], msg: WsMessage) {
        let clients = self.clients.read().await;
        if let Ok(json_str) = serde_json::to_string(&msg) {
            for user_id in user_ids {
                if let Some(user_conns) = clients.get(user_id) {
                    for tx in user_conns.values() {
                        let _ = tx.send(Message::Text(json_str.clone().into()));
                    }
                }
            }
        }
    }

    /// Join a live stream room
    pub async fn join_livestream_room(&self, stream_id: Uuid, user_id: Uuid) {
        let mut rooms = self.livestream_rooms.write().await;
        rooms.entry(stream_id).or_default().insert(user_id);

        let mut user_streams = self.user_livestreams.write().await;
        user_streams.entry(user_id).or_default().insert(stream_id);
    }

    /// Leave a live stream room
    pub async fn leave_livestream_room(&self, stream_id: Uuid, user_id: Uuid) {
        let mut rooms = self.livestream_rooms.write().await;
        if let Some(viewers) = rooms.get_mut(&stream_id) {
            viewers.remove(&user_id);
            if viewers.is_empty() {
                rooms.remove(&stream_id);
            }
        }

        let mut user_streams = self.user_livestreams.write().await;
        if let Some(streams) = user_streams.get_mut(&user_id) {
            streams.remove(&stream_id);
            if streams.is_empty() {
                user_streams.remove(&user_id);
            }
        }
    }

    /// Leave all livestream rooms for a user upon disconnect
    pub async fn leave_all_livestreams(&self, user_id: Uuid) -> Vec<Uuid> {
        let mut user_streams = self.user_livestreams.write().await;
        let streams = user_streams.remove(&user_id).unwrap_or_default();

        let mut rooms = self.livestream_rooms.write().await;
        let mut left_streams = Vec::new();
        for stream_id in streams {
            if let Some(viewers) = rooms.get_mut(&stream_id) {
                viewers.remove(&user_id);
                if viewers.is_empty() {
                    rooms.remove(&stream_id);
                }
            }
            left_streams.push(stream_id);
        }
        left_streams
    }

    /// Broadcast a message to all active viewers in a live stream room
    pub async fn broadcast_to_livestream(&self, stream_id: Uuid, msg: WsMessage) {
        let viewers = {
            let rooms = self.livestream_rooms.read().await;
            rooms.get(&stream_id).cloned().unwrap_or_default()
        };

        if viewers.is_empty() {
            return;
        }

        let clients = self.clients.read().await;
        if let Ok(json_str) = serde_json::to_string(&msg) {
            for viewer_id in viewers {
                if let Some(user_conns) = clients.get(&viewer_id) {
                    for tx in user_conns.values() {
                        let _ = tx.send(Message::Text(json_str.clone().into()));
                    }
                }
            }
        }
    }

    /// Get current active viewers in a live stream room
    pub async fn get_livestream_viewers(&self, stream_id: Uuid) -> Vec<Uuid> {
        let rooms = self.livestream_rooms.read().await;
        rooms.get(&stream_id).map(|v| v.iter().copied().collect()).unwrap_or_default()
    }

    /// Broadcast a message to all connected clients
    pub async fn broadcast_all(&self, msg: WsMessage) {
        let clients = self.clients.read().await;
        if let Ok(json_str) = serde_json::to_string(&msg) {
            for user_conns in clients.values() {
                for tx in user_conns.values() {
                    let _ = tx.send(Message::Text(json_str.clone().into()));
                }
            }
        }
    }
}
