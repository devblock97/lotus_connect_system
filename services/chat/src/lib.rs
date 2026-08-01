use std::sync::Arc;
use uuid::Uuid;
use database::PgPool;
use errors::{AppError, Result};
use models::{Conversation, Message, Group, MessageRead};

#[async_trait::async_trait]
pub trait ChatRepository: Send + Sync {
    async fn create_conversation(&self, id: Uuid, title: Option<&str>, is_group: bool) -> Result<Conversation>;
    async fn find_conversation(&self, id: Uuid) -> Result<Option<Conversation>>;
    async fn find_private_conversation(&self, user1: Uuid, user2: Uuid) -> Result<Option<Conversation>>;
    
    async fn add_member(&self, conversation_id: Uuid, user_id: Uuid, role: &str) -> Result<()>;
    async fn remove_member(&self, conversation_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn list_members(&self, conversation_id: Uuid) -> Result<Vec<Uuid>>;
    async fn is_member(&self, conversation_id: Uuid, user_id: Uuid) -> Result<bool>;
    
    async fn create_group(&self, id: Uuid, conversation_id: Uuid, name: &str, description: Option<&str>, created_by: Uuid) -> Result<Group>;
    async fn find_group_by_conversation(&self, conversation_id: Uuid) -> Result<Option<Group>>;

    async fn create_message(&self, id: Uuid, conversation_id: Uuid, sender_id: Uuid, content: &str, message_type: &str, reply_to_id: Option<Uuid>) -> Result<Message>;
    async fn find_message(&self, id: Uuid) -> Result<Option<Message>>;
    async fn update_message_content(&self, id: Uuid, content: &str) -> Result<Message>;
    async fn delete_message(&self, id: Uuid) -> Result<()>;
    async fn list_messages_paginated(&self, conversation_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<Message>>;
    
    async fn mark_as_read(&self, message_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn get_read_receipts(&self, message_id: Uuid) -> Result<Vec<MessageRead>>;
    async fn list_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>>;
}

#[async_trait::async_trait]
pub trait ChatService: Send + Sync {
    async fn create_private_chat(&self, creator_id: Uuid, friend_id: Uuid) -> Result<Conversation>;
    async fn create_group_chat(&self, creator_id: Uuid, name: &str, description: Option<&str>, members: Vec<Uuid>) -> Result<Group>;
    
    async fn send_message(&self, sender_id: Uuid, conversation_id: Uuid, content: &str, reply_to_id: Option<Uuid>) -> Result<Message>;
    async fn edit_message(&self, sender_id: Uuid, message_id: Uuid, content: &str) -> Result<Message>;
    async fn delete_message(&self, sender_id: Uuid, message_id: Uuid) -> Result<()>;
    
    async fn get_messages(&self, user_id: Uuid, conversation_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<Message>>;
    async fn read_message(&self, user_id: Uuid, message_id: Uuid) -> Result<()>;
    async fn get_conversation_members(&self, conversation_id: Uuid) -> Result<Vec<Uuid>>;
    async fn get_message_by_id(&self, user_id: Uuid, message_id: Uuid) -> Result<Message>;
    async fn list_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>>;
}

pub struct ChatRepositoryImpl {
    pool: PgPool,
}

impl ChatRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl ChatRepository for ChatRepositoryImpl {
    async fn create_conversation(&self, id: Uuid, title: Option<&str>, is_group: bool) -> Result<Conversation> {
        sqlx::query_as::<_, Conversation>(
            "INSERT INTO conversations (id, title, is_group) VALUES ($1, $2, $3) RETURNING id, title, is_group, created_at"
        )
        .bind(id)
        .bind(title)
        .bind(is_group)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_conversation(&self, id: Uuid) -> Result<Option<Conversation>> {
        sqlx::query_as::<_, Conversation>(
            "SELECT id, title, is_group, created_at FROM conversations WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_private_conversation(&self, user1: Uuid, user2: Uuid) -> Result<Option<Conversation>> {
        // Query to check if a 1-to-1 conversation exists containing both users
        sqlx::query_as::<_, Conversation>(
            r#"
            SELECT c.id, c.title, c.is_group, c.created_at
            FROM conversations c
            JOIN conversation_members m1 ON m1.conversation_id = c.id
            JOIN conversation_members m2 ON m2.conversation_id = c.id
            WHERE c.is_group = FALSE AND m1.user_id = $1 AND m2.user_id = $2
            "#
        )
        .bind(user1)
        .bind(user2)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn add_member(&self, conversation_id: Uuid, user_id: Uuid, role: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO conversation_members (conversation_id, user_id, role) VALUES ($1, $2, $3) ON CONFLICT DO NOTHING"
        )
        .bind(conversation_id)
        .bind(user_id)
        .bind(role)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn remove_member(&self, conversation_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "DELETE FROM conversation_members WHERE conversation_id = $1 AND user_id = $2"
        )
        .bind(conversation_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn list_members(&self, conversation_id: Uuid) -> Result<Vec<Uuid>> {
        let rows = sqlx::query!(
            "SELECT user_id FROM conversation_members WHERE conversation_id = $1",
            conversation_id
        )
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(|r| r.user_id).collect())
    }

    async fn is_member(&self, conversation_id: Uuid, user_id: Uuid) -> Result<bool> {
        let row = sqlx::query!(
            "SELECT 1 as has_member FROM conversation_members WHERE conversation_id = $1 AND user_id = $2",
            conversation_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.is_some())
    }

    async fn create_group(&self, id: Uuid, conversation_id: Uuid, name: &str, description: Option<&str>, created_by: Uuid) -> Result<Group> {
        sqlx::query_as::<_, Group>(
            "INSERT INTO groups (id, conversation_id, name, description, created_by) VALUES ($1, $2, $3, $4, $5) RETURNING id, conversation_id, name, description, created_by, created_at"
        )
        .bind(id)
        .bind(conversation_id)
        .bind(name)
        .bind(description)
        .bind(created_by)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_group_by_conversation(&self, conversation_id: Uuid) -> Result<Option<Group>> {
        sqlx::query_as::<_, Group>(
            "SELECT id, conversation_id, name, description, created_by, created_at FROM groups WHERE conversation_id = $1"
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn create_message(&self, id: Uuid, conversation_id: Uuid, sender_id: Uuid, content: &str, message_type: &str, reply_to_id: Option<Uuid>) -> Result<Message> {
        sqlx::query_as::<_, Message>(
            "INSERT INTO messages (id, conversation_id, sender_id, content, message_type, reply_to_id) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, conversation_id, sender_id, content, message_type, reply_to_id, is_edited, created_at, updated_at"
        )
        .bind(id)
        .bind(conversation_id)
        .bind(sender_id)
        .bind(content)
        .bind(message_type)
        .bind(reply_to_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_message(&self, id: Uuid) -> Result<Option<Message>> {
        sqlx::query_as::<_, Message>(
            "SELECT id, conversation_id, sender_id, content, message_type, reply_to_id, is_edited, created_at, updated_at FROM messages WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn update_message_content(&self, id: Uuid, content: &str) -> Result<Message> {
        sqlx::query_as::<_, Message>(
            "UPDATE messages SET content = $2, is_edited = TRUE, updated_at = NOW() WHERE id = $1 RETURNING id, conversation_id, sender_id, content, message_type, reply_to_id, is_edited, created_at, updated_at"
        )
        .bind(id)
        .bind(content)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn delete_message(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM messages WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn list_messages_paginated(&self, conversation_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<Message>> {
        match cursor {
            None => {
                sqlx::query_as::<_, Message>(
                    r#"
                    SELECT id, conversation_id, sender_id, content, message_type, reply_to_id, is_edited, created_at, updated_at
                    FROM messages
                    WHERE conversation_id = $1
                    ORDER BY created_at DESC
                    LIMIT $2
                    "#
                )
                .bind(conversation_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)
            }
            Some(cursor_id) => {
                sqlx::query_as::<_, Message>(
                    r#"
                    SELECT id, conversation_id, sender_id, content, message_type, reply_to_id, is_edited, created_at, updated_at
                    FROM messages
                    WHERE conversation_id = $1 
                      AND created_at < (SELECT created_at FROM messages WHERE id = $2)
                    ORDER BY created_at DESC
                    LIMIT $3
                    "#
                )
                .bind(conversation_id)
                .bind(cursor_id)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)
            }
        }
    }

    async fn mark_as_read(&self, message_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query(
            "INSERT INTO message_reads (message_id, user_id, read_at) VALUES ($1, $2, NOW()) ON CONFLICT DO NOTHING"
        )
        .bind(message_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn get_read_receipts(&self, message_id: Uuid) -> Result<Vec<MessageRead>> {
        sqlx::query_as::<_, MessageRead>(
            "SELECT message_id, user_id, read_at FROM message_reads WHERE message_id = $1"
        )
        .bind(message_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>> {
        sqlx::query_as::<_, Conversation>(
            r#"
            SELECT c.id, c.title, c.is_group, c.created_at
            FROM conversations c
            JOIN conversation_members cm ON cm.conversation_id = c.id
            WHERE cm.user_id = $1
            ORDER BY c.created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }
}

pub struct ChatServiceImpl {
    repo: Arc<dyn ChatRepository>,
}

impl ChatServiceImpl {
    pub fn new(repo: Arc<dyn ChatRepository>) -> Self {
        Self { repo }
    }
}

#[async_trait::async_trait]
impl ChatService for ChatServiceImpl {
    async fn create_private_chat(&self, creator_id: Uuid, friend_id: Uuid) -> Result<Conversation> {
        if let Some(existing) = self.repo.find_private_conversation(creator_id, friend_id).await? {
            return Ok(existing);
        }

        let conv_id = Uuid::now_v7();
        let conv = self.repo.create_conversation(conv_id, None, false).await?;
        
        self.repo.add_member(conv.id, creator_id, "member").await?;
        self.repo.add_member(conv.id, friend_id, "member").await?;

        Ok(conv)
    }

    async fn create_group_chat(&self, creator_id: Uuid, name: &str, description: Option<&str>, members: Vec<Uuid>) -> Result<Group> {
        let conv_id = Uuid::now_v7();
        let _conv = self.repo.create_conversation(conv_id, Some(name), true).await?;

        let group_id = Uuid::now_v7();
        let group = self.repo.create_group(group_id, conv_id, name, description, creator_id).await?;

        // Add creator as owner
        self.repo.add_member(conv_id, creator_id, "admin").await?;

        // Add other members
        for member_id in members {
            self.repo.add_member(conv_id, member_id, "member").await?;
        }

        Ok(group)
    }

    async fn send_message(&self, sender_id: Uuid, conversation_id: Uuid, content: &str, reply_to_id: Option<Uuid>) -> Result<Message> {
        if !self.repo.is_member(conversation_id, sender_id).await? {
            return Err(AppError::Authorization("User is not a member of this conversation".to_string()));
        }

        let message_id = Uuid::now_v7();
        self.repo.create_message(message_id, conversation_id, sender_id, content, "text", reply_to_id).await
    }

    async fn edit_message(&self, sender_id: Uuid, message_id: Uuid, content: &str) -> Result<Message> {
        let message = self.repo.find_message(message_id).await?
            .ok_or_else(|| AppError::NotFound("Message not found".to_string()))?;

        if message.sender_id != sender_id {
            return Err(AppError::Authorization("Only the message sender can edit this message".to_string()));
        }

        self.repo.update_message_content(message_id, content).await
    }

    async fn delete_message(&self, sender_id: Uuid, message_id: Uuid) -> Result<()> {
        let message = self.repo.find_message(message_id).await?
            .ok_or_else(|| AppError::NotFound("Message not found".to_string()))?;

        if message.sender_id != sender_id {
            return Err(AppError::Authorization("Only the message sender can delete this message".to_string()));
        }

        self.repo.delete_message(message_id).await
    }

    async fn get_messages(&self, user_id: Uuid, conversation_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<Message>> {
        if !self.repo.is_member(conversation_id, user_id).await? {
            return Err(AppError::Authorization("User is not a member of this conversation".to_string()));
        }

        self.repo.list_messages_paginated(conversation_id, cursor, limit).await
    }

    async fn read_message(&self, user_id: Uuid, message_id: Uuid) -> Result<()> {
        let message = self.repo.find_message(message_id).await?
            .ok_or_else(|| AppError::NotFound("Message not found".to_string()))?;

        if !self.repo.is_member(message.conversation_id, user_id).await? {
            return Err(AppError::Authorization("User is not a member of this conversation".to_string()));
        }

        self.repo.mark_as_read(message_id, user_id).await
    }

    async fn get_conversation_members(&self, conversation_id: Uuid) -> Result<Vec<Uuid>> {
        self.repo.list_members(conversation_id).await
    }

    async fn get_message_by_id(&self, user_id: Uuid, message_id: Uuid) -> Result<Message> {
        let message = self.repo.find_message(message_id).await?
            .ok_or_else(|| AppError::NotFound("Message not found".to_string()))?;

        if !self.repo.is_member(message.conversation_id, user_id).await? {
            return Err(AppError::Authorization("User is not a member of this conversation".to_string()));
        }

        Ok(message)
    }

    async fn list_user_conversations(&self, user_id: Uuid) -> Result<Vec<Conversation>> {
        self.repo.list_user_conversations(user_id).await
    }
}
