use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use database::PgPool;
use errors::{AppError, Result};
use models::{Story, StoryWithAuthor, StoryReaction, StoryReactionWithUser, StoryViewerWithUser, CloseFriend, CloseFriendWithUser};
use dto::{
    CreateStoryRequest, StoryResponse, StoryTrayItemResponse, StoryViewerResponse,
    StoryReactionResponse, PostAuthorResponse, CloseFriendResponse
};

#[async_trait::async_trait]
pub trait StoryRepository: Send + Sync {
    async fn create_story(
        &self,
        id: Uuid,
        author_id: Uuid,
        media_type: &str,
        media_url: &str,
        thumbnail_url: Option<&str>,
        caption: Option<&str>,
        duration: f64,
        visibility: &str,
        background_color: Option<&str>,
        metadata: Option<sqlx::types::Json<serde_json::Value>>,
        expires_at: DateTime<Utc>,
    ) -> Result<Story>;

    async fn find_story_by_id(&self, story_id: Uuid, viewer_id: Uuid) -> Result<Option<StoryWithAuthor>>;
    async fn delete_story(&self, story_id: Uuid, author_id: Uuid) -> Result<bool>;

    async fn list_active_stories_for_tray(&self, viewer_id: Uuid) -> Result<Vec<StoryWithAuthor>>;
    async fn list_user_active_stories(&self, author_id: Uuid, viewer_id: Uuid) -> Result<Vec<StoryWithAuthor>>;
    async fn list_my_active_stories(&self, author_id: Uuid) -> Result<Vec<StoryWithAuthor>>;
    async fn list_my_archived_stories(&self, author_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<StoryWithAuthor>>;

    async fn record_view(&self, id: Uuid, story_id: Uuid, viewer_id: Uuid) -> Result<()>;
    async fn list_story_viewers(&self, story_id: Uuid) -> Result<Vec<StoryViewerWithUser>>;

    async fn add_or_update_reaction(&self, id: Uuid, story_id: Uuid, user_id: Uuid, reaction: &str) -> Result<StoryReaction>;
    async fn remove_reaction(&self, story_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn list_story_reactions(&self, story_id: Uuid) -> Result<Vec<StoryReactionWithUser>>;

    async fn add_close_friend(&self, id: Uuid, user_id: Uuid, friend_id: Uuid) -> Result<CloseFriend>;
    async fn remove_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()>;
    async fn list_close_friends(&self, user_id: Uuid) -> Result<Vec<CloseFriendWithUser>>;
    async fn is_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<bool>;
    async fn is_friend(&self, user1: Uuid, user2: Uuid) -> Result<bool>;
    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<PostAuthorResponse>>;
}

#[async_trait::async_trait]
pub trait StoryService: Send + Sync {
    async fn create_story(&self, author_id: Uuid, req: CreateStoryRequest) -> Result<StoryResponse>;
    async fn get_story(&self, viewer_id: Uuid, story_id: Uuid) -> Result<StoryResponse>;
    async fn delete_story(&self, user_id: Uuid, story_id: Uuid) -> Result<()>;

    async fn get_stories_tray(&self, viewer_id: Uuid) -> Result<Vec<StoryTrayItemResponse>>;
    async fn get_user_stories(&self, viewer_id: Uuid, author_id: Uuid) -> Result<Vec<StoryResponse>>;
    async fn get_my_active_stories(&self, author_id: Uuid) -> Result<Vec<StoryResponse>>;
    async fn get_my_archived_stories(&self, author_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<StoryResponse>>;

    async fn mark_story_viewed(&self, viewer_id: Uuid, story_id: Uuid) -> Result<()>;
    async fn get_story_viewers(&self, author_id: Uuid, story_id: Uuid) -> Result<Vec<StoryViewerResponse>>;

    async fn add_reaction(&self, user_id: Uuid, story_id: Uuid, reaction: &str) -> Result<StoryReactionResponse>;
    async fn remove_reaction(&self, user_id: Uuid, story_id: Uuid) -> Result<()>;
    async fn get_story_reactions(&self, author_id: Uuid, story_id: Uuid) -> Result<Vec<StoryReactionResponse>>;

    async fn add_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()>;
    async fn remove_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()>;
    async fn list_close_friends(&self, user_id: Uuid) -> Result<Vec<CloseFriendResponse>>;
}

pub struct StoryRepositoryImpl {
    pool: PgPool,
}

impl StoryRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl StoryRepository for StoryRepositoryImpl {
    async fn create_story(
        &self,
        id: Uuid,
        author_id: Uuid,
        media_type: &str,
        media_url: &str,
        thumbnail_url: Option<&str>,
        caption: Option<&str>,
        duration: f64,
        visibility: &str,
        background_color: Option<&str>,
        metadata: Option<sqlx::types::Json<serde_json::Value>>,
        expires_at: DateTime<Utc>,
    ) -> Result<Story> {
        sqlx::query_as::<_, Story>(
            r#"
            INSERT INTO stories (
                id, author_id, media_type, media_url, thumbnail_url, caption,
                duration, visibility, background_color, metadata, expires_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id, author_id, media_type, media_url, thumbnail_url, caption,
                      duration, visibility, background_color, metadata, created_at, expires_at
            "#
        )
        .bind(id)
        .bind(author_id)
        .bind(media_type)
        .bind(media_url)
        .bind(thumbnail_url)
        .bind(caption)
        .bind(duration)
        .bind(visibility)
        .bind(background_color)
        .bind(metadata)
        .bind(expires_at)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_story_by_id(&self, story_id: Uuid, viewer_id: Uuid) -> Result<Option<StoryWithAuthor>> {
        sqlx::query_as::<_, StoryWithAuthor>(
            r#"
            SELECT 
                s.id,
                s.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                s.media_type,
                s.media_url,
                s.thumbnail_url,
                s.caption,
                s.duration,
                s.visibility,
                s.background_color,
                s.metadata,
                s.created_at,
                s.expires_at,
                (SELECT COUNT(*) FROM story_views sv WHERE sv.story_id = s.id) AS view_count,
                EXISTS(SELECT 1 FROM story_views sv WHERE sv.story_id = s.id AND sv.viewer_id = $2) AS has_viewed,
                (SELECT reaction FROM story_reactions sr WHERE sr.story_id = s.id AND sr.user_id = $2 LIMIT 1) AS viewer_reaction,
                EXISTS(SELECT 1 FROM close_friends cf WHERE cf.user_id = s.author_id AND cf.friend_id = $2) AS is_close_friend
            FROM stories s
            JOIN users u ON s.author_id = u.id
            WHERE s.id = $1
            "#
        )
        .bind(story_id)
        .bind(viewer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn delete_story(&self, story_id: Uuid, author_id: Uuid) -> Result<bool> {
        let result = sqlx::query("DELETE FROM stories WHERE id = $1 AND author_id = $2")
            .bind(story_id)
            .bind(author_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(result.rows_affected() > 0)
    }

    async fn list_active_stories_for_tray(&self, viewer_id: Uuid) -> Result<Vec<StoryWithAuthor>> {
        sqlx::query_as::<_, StoryWithAuthor>(
            r#"
            SELECT 
                s.id,
                s.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                s.media_type,
                s.media_url,
                s.thumbnail_url,
                s.caption,
                s.duration,
                s.visibility,
                s.background_color,
                s.metadata,
                s.created_at,
                s.expires_at,
                (SELECT COUNT(*) FROM story_views sv WHERE sv.story_id = s.id) AS view_count,
                EXISTS(SELECT 1 FROM story_views sv WHERE sv.story_id = s.id AND sv.viewer_id = $1) AS has_viewed,
                (SELECT reaction FROM story_reactions sr WHERE sr.story_id = s.id AND sr.user_id = $1 LIMIT 1) AS viewer_reaction,
                EXISTS(SELECT 1 FROM close_friends cf WHERE cf.user_id = s.author_id AND cf.friend_id = $1) AS is_close_friend
            FROM stories s
            JOIN users u ON s.author_id = u.id
            WHERE s.expires_at > NOW()
              AND (
                s.author_id = $1
                OR s.visibility = 'public'
                OR (
                    s.visibility = 'friends'
                    AND s.author_id IN (
                        SELECT friend_id FROM friendships WHERE user_id = $1 AND status = 'accepted'
                        UNION
                        SELECT user_id FROM friendships WHERE friend_id = $1 AND status = 'accepted'
                    )
                )
                OR (
                    s.visibility = 'close_friends'
                    AND EXISTS(SELECT 1 FROM close_friends cf WHERE cf.user_id = s.author_id AND cf.friend_id = $1)
                )
              )
            ORDER BY s.created_at ASC
            "#
        )
        .bind(viewer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_user_active_stories(&self, author_id: Uuid, viewer_id: Uuid) -> Result<Vec<StoryWithAuthor>> {
        sqlx::query_as::<_, StoryWithAuthor>(
            r#"
            SELECT 
                s.id,
                s.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                s.media_type,
                s.media_url,
                s.thumbnail_url,
                s.caption,
                s.duration,
                s.visibility,
                s.background_color,
                s.metadata,
                s.created_at,
                s.expires_at,
                (SELECT COUNT(*) FROM story_views sv WHERE sv.story_id = s.id) AS view_count,
                EXISTS(SELECT 1 FROM story_views sv WHERE sv.story_id = s.id AND sv.viewer_id = $2) AS has_viewed,
                (SELECT reaction FROM story_reactions sr WHERE sr.story_id = s.id AND sr.user_id = $2 LIMIT 1) AS viewer_reaction,
                EXISTS(SELECT 1 FROM close_friends cf WHERE cf.user_id = s.author_id AND cf.friend_id = $2) AS is_close_friend
            FROM stories s
            JOIN users u ON s.author_id = u.id
            WHERE s.author_id = $1
              AND s.expires_at > NOW()
              AND (
                $1 = $2
                OR s.visibility = 'public'
                OR (
                    s.visibility = 'friends'
                    AND $1 IN (
                        SELECT friend_id FROM friendships WHERE user_id = $2 AND status = 'accepted'
                        UNION
                        SELECT user_id FROM friendships WHERE friend_id = $2 AND status = 'accepted'
                    )
                )
                OR (
                    s.visibility = 'close_friends'
                    AND EXISTS(SELECT 1 FROM close_friends cf WHERE cf.user_id = $1 AND cf.friend_id = $2)
                )
              )
            ORDER BY s.created_at ASC
            "#
        )
        .bind(author_id)
        .bind(viewer_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_my_active_stories(&self, author_id: Uuid) -> Result<Vec<StoryWithAuthor>> {
        sqlx::query_as::<_, StoryWithAuthor>(
            r#"
            SELECT 
                s.id,
                s.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                s.media_type,
                s.media_url,
                s.thumbnail_url,
                s.caption,
                s.duration,
                s.visibility,
                s.background_color,
                s.metadata,
                s.created_at,
                s.expires_at,
                (SELECT COUNT(*) FROM story_views sv WHERE sv.story_id = s.id) AS view_count,
                TRUE AS has_viewed,
                NULL::VARCHAR(32) AS viewer_reaction,
                TRUE AS is_close_friend
            FROM stories s
            JOIN users u ON s.author_id = u.id
            WHERE s.author_id = $1
              AND s.expires_at > NOW()
            ORDER BY s.created_at ASC
            "#
        )
        .bind(author_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_my_archived_stories(&self, author_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<StoryWithAuthor>> {
        let cursor_time = if let Some(cid) = cursor {
            sqlx::query_scalar::<_, DateTime<Utc>>("SELECT created_at FROM stories WHERE id = $1")
                .bind(cid)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            None
        };

        sqlx::query_as::<_, StoryWithAuthor>(
            r#"
            SELECT 
                s.id,
                s.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                s.media_type,
                s.media_url,
                s.thumbnail_url,
                s.caption,
                s.duration,
                s.visibility,
                s.background_color,
                s.metadata,
                s.created_at,
                s.expires_at,
                (SELECT COUNT(*) FROM story_views sv WHERE sv.story_id = s.id) AS view_count,
                TRUE AS has_viewed,
                NULL::VARCHAR(32) AS viewer_reaction,
                TRUE AS is_close_friend
            FROM stories s
            JOIN users u ON s.author_id = u.id
            WHERE s.author_id = $1
              AND s.expires_at <= NOW()
              AND ($2::TIMESTAMPTZ IS NULL OR s.created_at < $2)
            ORDER BY s.created_at DESC
            LIMIT $3
            "#
        )
        .bind(author_id)
        .bind(cursor_time)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn record_view(&self, id: Uuid, story_id: Uuid, viewer_id: Uuid) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO story_views (id, story_id, viewer_id, viewed_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (story_id, viewer_id) DO UPDATE SET viewed_at = NOW()
            "#
        )
        .bind(id)
        .bind(story_id)
        .bind(viewer_id)
        .execute(&self.pool)
        .await
        .map(|_| ())
        .map_err(AppError::Database)
    }

    async fn list_story_viewers(&self, story_id: Uuid) -> Result<Vec<StoryViewerWithUser>> {
        sqlx::query_as::<_, StoryViewerWithUser>(
            r#"
            SELECT 
                sv.id,
                sv.story_id,
                sv.viewer_id,
                u.username AS viewer_username,
                u.full_name AS viewer_full_name,
                u.avatar_url AS viewer_avatar_url,
                sv.viewed_at,
                sr.reaction
            FROM story_views sv
            JOIN users u ON sv.viewer_id = u.id
            LEFT JOIN story_reactions sr ON sr.story_id = sv.story_id AND sr.user_id = sv.viewer_id
            WHERE sv.story_id = $1
            ORDER BY sv.viewed_at DESC
            "#
        )
        .bind(story_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn add_or_update_reaction(&self, id: Uuid, story_id: Uuid, user_id: Uuid, reaction: &str) -> Result<StoryReaction> {
        sqlx::query_as::<_, StoryReaction>(
            r#"
            INSERT INTO story_reactions (id, story_id, user_id, reaction, created_at)
            VALUES ($1, $2, $3, $4, NOW())
            ON CONFLICT (story_id, user_id)
            DO UPDATE SET reaction = $4, created_at = NOW()
            RETURNING id, story_id, user_id, reaction, created_at
            "#
        )
        .bind(id)
        .bind(story_id)
        .bind(user_id)
        .bind(reaction)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn remove_reaction(&self, story_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM story_reactions WHERE story_id = $1 AND user_id = $2")
            .bind(story_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn list_story_reactions(&self, story_id: Uuid) -> Result<Vec<StoryReactionWithUser>> {
        sqlx::query_as::<_, StoryReactionWithUser>(
            r#"
            SELECT 
                sr.id,
                sr.story_id,
                sr.user_id,
                u.username,
                u.full_name,
                u.avatar_url,
                sr.reaction,
                sr.created_at
            FROM story_reactions sr
            JOIN users u ON sr.user_id = u.id
            WHERE sr.story_id = $1
            ORDER BY sr.created_at DESC
            "#
        )
        .bind(story_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn add_close_friend(&self, id: Uuid, user_id: Uuid, friend_id: Uuid) -> Result<CloseFriend> {
        sqlx::query_as::<_, CloseFriend>(
            r#"
            INSERT INTO close_friends (id, user_id, friend_id, created_at)
            VALUES ($1, $2, $3, NOW())
            ON CONFLICT (user_id, friend_id) DO UPDATE SET created_at = close_friends.created_at
            RETURNING id, user_id, friend_id, created_at
            "#
        )
        .bind(id)
        .bind(user_id)
        .bind(friend_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn remove_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM close_friends WHERE user_id = $1 AND friend_id = $2")
            .bind(user_id)
            .bind(friend_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn list_close_friends(&self, user_id: Uuid) -> Result<Vec<CloseFriendWithUser>> {
        sqlx::query_as::<_, CloseFriendWithUser>(
            r#"
            SELECT 
                cf.id,
                cf.user_id,
                cf.friend_id,
                u.username AS friend_username,
                u.full_name AS friend_full_name,
                u.avatar_url AS friend_avatar_url,
                cf.created_at
            FROM close_friends cf
            JOIN users u ON cf.friend_id = u.id
            WHERE cf.user_id = $1
            ORDER BY cf.created_at DESC
            "#
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn is_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM close_friends WHERE user_id = $1 AND friend_id = $2"
        )
        .bind(user_id)
        .bind(friend_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(count > 0)
    }

    async fn is_friend(&self, user1: Uuid, user2: Uuid) -> Result<bool> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(*) FROM friendships 
            WHERE ((user_id = $1 AND friend_id = $2) OR (user_id = $2 AND friend_id = $1))
              AND status = 'accepted'
            "#
        )
        .bind(user1)
        .bind(user2)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(count > 0)
    }

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<PostAuthorResponse>> {
        let row = sqlx::query_as::<_, (Uuid, String, Option<String>, Option<String>)>(
            "SELECT id, username, full_name, avatar_url FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.map(|(id, username, full_name, avatar_url)| PostAuthorResponse {
            id,
            username,
            full_name,
            avatar_url,
        }))
    }
}

pub struct StoryServiceImpl {
    repo: Arc<dyn StoryRepository>,
}

impl StoryServiceImpl {
    pub fn new(repo: Arc<dyn StoryRepository>) -> Self {
        Self { repo }
    }

    fn to_story_response(s: StoryWithAuthor) -> StoryResponse {
        StoryResponse {
            id: s.id,
            author: PostAuthorResponse {
                id: s.author_id,
                username: s.author_username,
                full_name: s.author_full_name,
                avatar_url: s.author_avatar_url,
            },
            media_type: s.media_type,
            media_url: s.media_url,
            thumbnail_url: s.thumbnail_url,
            caption: s.caption,
            duration: s.duration,
            visibility: s.visibility,
            background_color: s.background_color,
            metadata: s.metadata.map(|j| j.0),
            created_at: s.created_at,
            expires_at: s.expires_at,
            view_count: s.view_count,
            has_viewed: s.has_viewed,
            viewer_reaction: s.viewer_reaction,
            is_close_friend: s.is_close_friend,
        }
    }
}

#[async_trait::async_trait]
impl StoryService for StoryServiceImpl {
    async fn create_story(&self, author_id: Uuid, req: CreateStoryRequest) -> Result<StoryResponse> {
        let story_id = Uuid::now_v7();
        let media_type = req.media_type.unwrap_or_else(|| "image".to_string());
        let duration = req.duration.unwrap_or(5.0);
        let visibility = req.visibility.unwrap_or_else(|| "friends".to_string());
        let metadata_json = req.metadata.map(sqlx::types::Json);
        let expires_at = Utc::now() + chrono::Duration::hours(24);

        self.repo.create_story(
            story_id,
            author_id,
            &media_type,
            &req.media_url,
            req.thumbnail_url.as_deref(),
            req.caption.as_deref(),
            duration,
            &visibility,
            req.background_color.as_deref(),
            metadata_json,
            expires_at,
        ).await?;

        // Retrieve created story with author details
        let story = self.repo.find_story_by_id(story_id, author_id).await?
            .ok_or_else(|| AppError::Internal("Created story not found".to_string()))?;

        Ok(Self::to_story_response(story))
    }

    async fn get_story(&self, viewer_id: Uuid, story_id: Uuid) -> Result<StoryResponse> {
        let story = self.repo.find_story_by_id(story_id, viewer_id).await?
            .ok_or_else(|| AppError::NotFound("Story not found".to_string()))?;

        // Permission check
        if story.author_id != viewer_id {
            if story.expires_at <= Utc::now() {
                return Err(AppError::NotFound("Story has expired".to_string()));
            }

            match story.visibility.as_str() {
                "public" => {},
                "friends" => {
                    let is_friend = self.repo.is_friend(story.author_id, viewer_id).await?;
                    if !is_friend {
                        return Err(AppError::Authorization("Only friends can view this story".to_string()));
                    }
                },
                "close_friends" => {
                    let is_cf = self.repo.is_close_friend(story.author_id, viewer_id).await?;
                    if !is_cf {
                        return Err(AppError::Authorization("Only close friends can view this story".to_string()));
                    }
                },
                _ => return Err(AppError::Authorization("Cannot view this story".to_string())),
            }
        }

        Ok(Self::to_story_response(story))
    }

    async fn delete_story(&self, user_id: Uuid, story_id: Uuid) -> Result<()> {
        let deleted = self.repo.delete_story(story_id, user_id).await?;
        if !deleted {
            return Err(AppError::NotFound("Story not found or you do not have permission to delete it".to_string()));
        }
        Ok(())
    }

    async fn get_stories_tray(&self, viewer_id: Uuid) -> Result<Vec<StoryTrayItemResponse>> {
        let stories = self.repo.list_active_stories_for_tray(viewer_id).await?;

        // Group stories by author_id
        let mut grouped: std::collections::BTreeMap<Uuid, (PostAuthorResponse, Vec<StoryResponse>, bool, bool, DateTime<Utc>)> = std::collections::BTreeMap::new();

        for s in stories {
            let author_id = s.author_id;
            let author_info = PostAuthorResponse {
                id: s.author_id,
                username: s.author_username.clone(),
                full_name: s.author_full_name.clone(),
                avatar_url: s.author_avatar_url.clone(),
            };
            let is_close_friends_story = s.visibility == "close_friends";
            let has_viewed = s.has_viewed;
            let created_at = s.created_at;
            let resp = Self::to_story_response(s);

            let entry = grouped.entry(author_id).or_insert_with(|| {
                (author_info, Vec::new(), false, false, created_at)
            });

            if !has_viewed {
                entry.2 = true; // has_unseen = true
            }
            if is_close_friends_story {
                entry.3 = true; // has_close_friends_story = true
            }
            if created_at > entry.4 {
                entry.4 = created_at; // latest_story_created_at
            }
            entry.1.push(resp);
        }

        // Ensure viewer (self) is always included in the tray (e.g. for the "Your Story" item in mobile UI)
        if !grouped.contains_key(&viewer_id) {
            if let Ok(Some(me)) = self.repo.find_user_by_id(viewer_id).await {
                grouped.insert(viewer_id, (me, Vec::new(), false, false, Utc::now()));
            }
        }

        let mut tray_items: Vec<StoryTrayItemResponse> = grouped.into_iter().map(|(author_id, (user, stories, has_unseen, has_close_friends_story, latest_time))| {
            let is_self = author_id == viewer_id;
            let total_stories = stories.len();
            StoryTrayItemResponse {
                user,
                stories,
                has_unseen,
                total_stories,
                latest_story_created_at: latest_time,
                has_close_friends_story,
                is_self,
            }
        }).collect();

        // Sort tray items:
        // Current user (self) first
        // Users with unseen stories first, ordered by latest story desc
        // Users with all seen stories, ordered by latest story desc
        tray_items.sort_by(|a, b| {
            if a.is_self {
                return std::cmp::Ordering::Less;
            }
            if b.is_self {
                return std::cmp::Ordering::Greater;
            }

            match (a.has_unseen, b.has_unseen) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => b.latest_story_created_at.cmp(&a.latest_story_created_at),
            }
        });

        Ok(tray_items)
    }

    async fn get_user_stories(&self, viewer_id: Uuid, author_id: Uuid) -> Result<Vec<StoryResponse>> {
        let stories = self.repo.list_user_active_stories(author_id, viewer_id).await?;
        Ok(stories.into_iter().map(Self::to_story_response).collect())
    }

    async fn get_my_active_stories(&self, author_id: Uuid) -> Result<Vec<StoryResponse>> {
        let stories = self.repo.list_my_active_stories(author_id).await?;
        Ok(stories.into_iter().map(Self::to_story_response).collect())
    }

    async fn get_my_archived_stories(&self, author_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<StoryResponse>> {
        let page_limit = limit.unwrap_or(20).clamp(1, 50);
        let stories = self.repo.list_my_archived_stories(author_id, cursor, page_limit).await?;
        Ok(stories.into_iter().map(Self::to_story_response).collect())
    }

    async fn mark_story_viewed(&self, viewer_id: Uuid, story_id: Uuid) -> Result<()> {
        let story = self.repo.find_story_by_id(story_id, viewer_id).await?
            .ok_or_else(|| AppError::NotFound("Story not found".to_string()))?;

        if story.author_id != viewer_id {
            // Verify permission
            if story.visibility == "close_friends" {
                let is_cf = self.repo.is_close_friend(story.author_id, viewer_id).await?;
                if !is_cf {
                    return Err(AppError::Authorization("Not authorized to view close friends story".to_string()));
                }
            } else if story.visibility == "friends" {
                let is_f = self.repo.is_friend(story.author_id, viewer_id).await?;
                if !is_f {
                    return Err(AppError::Authorization("Not authorized to view friends-only story".to_string()));
                }
            }
            let view_id = Uuid::now_v7();
            self.repo.record_view(view_id, story_id, viewer_id).await?;
        }
        Ok(())
    }

    async fn get_story_viewers(&self, author_id: Uuid, story_id: Uuid) -> Result<Vec<StoryViewerResponse>> {
        let story = self.repo.find_story_by_id(story_id, author_id).await?
            .ok_or_else(|| AppError::NotFound("Story not found".to_string()))?;

        if story.author_id != author_id {
            return Err(AppError::Authorization("Only the author can view story viewers".to_string()));
        }

        let viewers = self.repo.list_story_viewers(story_id).await?;
        let res = viewers.into_iter().map(|v| StoryViewerResponse {
            id: v.id,
            viewer: PostAuthorResponse {
                id: v.viewer_id,
                username: v.viewer_username,
                full_name: v.viewer_full_name,
                avatar_url: v.viewer_avatar_url,
            },
            viewed_at: v.viewed_at,
            reaction: v.reaction,
        }).collect();

        Ok(res)
    }

    async fn add_reaction(&self, user_id: Uuid, story_id: Uuid, reaction: &str) -> Result<StoryReactionResponse> {
        let story = self.repo.find_story_by_id(story_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Story not found".to_string()))?;

        if story.author_id != user_id {
            // Verify permission
            if story.visibility == "close_friends" {
                let is_cf = self.repo.is_close_friend(story.author_id, user_id).await?;
                if !is_cf {
                    return Err(AppError::Authorization("Cannot react to close friends story".to_string()));
                }
            } else if story.visibility == "friends" {
                let is_f = self.repo.is_friend(story.author_id, user_id).await?;
                if !is_f {
                    return Err(AppError::Authorization("Cannot react to friends-only story".to_string()));
                }
            }
        }

        // Record reaction
        let reaction_id = Uuid::now_v7();
        let saved = self.repo.add_or_update_reaction(reaction_id, story_id, user_id, reaction).await?;

        // Automatically mark story as viewed as well
        if story.author_id != user_id {
            let view_id = Uuid::now_v7();
            let _ = self.repo.record_view(view_id, story_id, user_id).await;
        }

        // Get user details
        let user_author = self.repo.find_user_by_id(user_id).await?
            .unwrap_or(PostAuthorResponse {
                id: user_id,
                username: "".to_string(),
                full_name: None,
                avatar_url: None,
            });

        Ok(StoryReactionResponse {
            id: saved.id,
            story_id: saved.story_id,
            user: user_author,
            reaction: saved.reaction,
            created_at: saved.created_at,
        })
    }

    async fn remove_reaction(&self, user_id: Uuid, story_id: Uuid) -> Result<()> {
        self.repo.remove_reaction(story_id, user_id).await
    }

    async fn get_story_reactions(&self, author_id: Uuid, story_id: Uuid) -> Result<Vec<StoryReactionResponse>> {
        let story = self.repo.find_story_by_id(story_id, author_id).await?
            .ok_or_else(|| AppError::NotFound("Story not found".to_string()))?;

        if story.author_id != author_id {
            return Err(AppError::Authorization("Only the author can view story reactions".to_string()));
        }

        let reactions = self.repo.list_story_reactions(story_id).await?;
        let res = reactions.into_iter().map(|r| StoryReactionResponse {
            id: r.id,
            story_id: r.story_id,
            user: PostAuthorResponse {
                id: r.user_id,
                username: r.username,
                full_name: r.full_name,
                avatar_url: r.avatar_url,
            },
            reaction: r.reaction,
            created_at: r.created_at,
        }).collect();

        Ok(res)
    }

    async fn add_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()> {
        if user_id == friend_id {
            return Err(AppError::Validation("Cannot add yourself as close friend".to_string()));
        }
        let id = Uuid::now_v7();
        self.repo.add_close_friend(id, user_id, friend_id).await?;
        Ok(())
    }

    async fn remove_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()> {
        self.repo.remove_close_friend(user_id, friend_id).await
    }

    async fn list_close_friends(&self, user_id: Uuid) -> Result<Vec<CloseFriendResponse>> {
        let friends = self.repo.list_close_friends(user_id).await?;
        Ok(friends.into_iter().map(|cf| CloseFriendResponse {
            id: cf.id,
            friend: PostAuthorResponse {
                id: cf.friend_id,
                username: cf.friend_username,
                full_name: cf.friend_full_name,
                avatar_url: cf.friend_avatar_url,
            },
            created_at: cf.created_at,
        }).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockStoryRepository {
        pub stories: Mutex<Vec<StoryWithAuthor>>,
        pub close_friends: Mutex<Vec<(Uuid, Uuid)>>,
        pub friends: Mutex<Vec<(Uuid, Uuid)>>,
        pub recorded_views: Mutex<Vec<(Uuid, Uuid)>>,
    }

    impl MockStoryRepository {
        fn new() -> Self {
            Self {
                stories: Mutex::new(Vec::new()),
                close_friends: Mutex::new(Vec::new()),
                friends: Mutex::new(Vec::new()),
                recorded_views: Mutex::new(Vec::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl StoryRepository for MockStoryRepository {
        async fn create_story(
            &self,
            id: Uuid,
            author_id: Uuid,
            media_type: &str,
            media_url: &str,
            thumbnail_url: Option<&str>,
            caption: Option<&str>,
            duration: f64,
            visibility: &str,
            background_color: Option<&str>,
            metadata: Option<sqlx::types::Json<serde_json::Value>>,
            expires_at: DateTime<Utc>,
        ) -> Result<Story> {
            let story = Story {
                id,
                author_id,
                media_type: media_type.to_string(),
                media_url: media_url.to_string(),
                thumbnail_url: thumbnail_url.map(|s| s.to_string()),
                caption: caption.map(|s| s.to_string()),
                duration,
                visibility: visibility.to_string(),
                background_color: background_color.map(|s| s.to_string()),
                metadata,
                created_at: Utc::now(),
                expires_at,
            };
            Ok(story)
        }

        async fn find_story_by_id(&self, story_id: Uuid, viewer_id: Uuid) -> Result<Option<StoryWithAuthor>> {
            let stories = self.stories.lock().unwrap();
            let story = stories.iter().find(|s| s.id == story_id).cloned();
            if let Some(mut s) = story {
                let cf = self.close_friends.lock().unwrap();
                s.is_close_friend = cf.contains(&(s.author_id, viewer_id));
                let views = self.recorded_views.lock().unwrap();
                s.has_viewed = views.contains(&(story_id, viewer_id));
                Ok(Some(s))
            } else {
                Ok(None)
            }
        }

        async fn delete_story(&self, story_id: Uuid, author_id: Uuid) -> Result<bool> {
            let mut stories = self.stories.lock().unwrap();
            let initial = stories.len();
            stories.retain(|s| !(s.id == story_id && s.author_id == author_id));
            Ok(stories.len() < initial)
        }

        async fn list_active_stories_for_tray(&self, _viewer_id: Uuid) -> Result<Vec<StoryWithAuthor>> {
            let stories = self.stories.lock().unwrap();
            Ok(stories.clone())
        }

        async fn list_user_active_stories(&self, author_id: Uuid, _viewer_id: Uuid) -> Result<Vec<StoryWithAuthor>> {
            let stories = self.stories.lock().unwrap();
            Ok(stories.iter().filter(|s| s.author_id == author_id).cloned().collect())
        }

        async fn list_my_active_stories(&self, author_id: Uuid) -> Result<Vec<StoryWithAuthor>> {
            let stories = self.stories.lock().unwrap();
            Ok(stories.iter().filter(|s| s.author_id == author_id).cloned().collect())
        }

        async fn list_my_archived_stories(&self, _author_id: Uuid, _cursor: Option<Uuid>, _limit: i64) -> Result<Vec<StoryWithAuthor>> {
            Ok(Vec::new())
        }

        async fn record_view(&self, _id: Uuid, story_id: Uuid, viewer_id: Uuid) -> Result<()> {
            let mut views = self.recorded_views.lock().unwrap();
            views.push((story_id, viewer_id));
            Ok(())
        }

        async fn list_story_viewers(&self, story_id: Uuid) -> Result<Vec<StoryViewerWithUser>> {
            let views = self.recorded_views.lock().unwrap();
            let res = views.iter()
                .filter(|(sid, _)| *sid == story_id)
                .map(|&(sid, vid)| StoryViewerWithUser {
                    id: Uuid::now_v7(),
                    story_id: sid,
                    viewer_id: vid,
                    viewer_username: format!("user_{}", vid),
                    viewer_full_name: Some("Viewer Name".to_string()),
                    viewer_avatar_url: None,
                    viewed_at: Utc::now(),
                    reaction: None,
                })
                .collect();
            Ok(res)
        }

        async fn add_or_update_reaction(&self, id: Uuid, story_id: Uuid, user_id: Uuid, reaction: &str) -> Result<StoryReaction> {
            Ok(StoryReaction {
                id,
                story_id,
                user_id,
                reaction: reaction.to_string(),
                created_at: Utc::now(),
            })
        }

        async fn remove_reaction(&self, _story_id: Uuid, _user_id: Uuid) -> Result<()> {
            Ok(())
        }

        async fn list_story_reactions(&self, _story_id: Uuid) -> Result<Vec<StoryReactionWithUser>> {
            Ok(Vec::new())
        }

        async fn add_close_friend(&self, id: Uuid, user_id: Uuid, friend_id: Uuid) -> Result<CloseFriend> {
            let mut cf = self.close_friends.lock().unwrap();
            cf.push((user_id, friend_id));
            Ok(CloseFriend {
                id,
                user_id,
                friend_id,
                created_at: Utc::now(),
            })
        }

        async fn remove_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<()> {
            let mut cf = self.close_friends.lock().unwrap();
            cf.retain(|&(u, f)| !(u == user_id && f == friend_id));
            Ok(())
        }

        async fn list_close_friends(&self, user_id: Uuid) -> Result<Vec<CloseFriendWithUser>> {
            let cf = self.close_friends.lock().unwrap();
            let res = cf.iter()
                .filter(|(u, _)| *u == user_id)
                .map(|&(u, f)| CloseFriendWithUser {
                    id: Uuid::now_v7(),
                    user_id: u,
                    friend_id: f,
                    friend_username: format!("friend_{}", f),
                    friend_full_name: None,
                    friend_avatar_url: None,
                    created_at: Utc::now(),
                })
                .collect();
            Ok(res)
        }

        async fn is_close_friend(&self, user_id: Uuid, friend_id: Uuid) -> Result<bool> {
            let cf = self.close_friends.lock().unwrap();
            Ok(cf.contains(&(user_id, friend_id)))
        }

        async fn is_friend(&self, user1: Uuid, user2: Uuid) -> Result<bool> {
            let friends = self.friends.lock().unwrap();
            Ok(friends.contains(&(user1, user2)) || friends.contains(&(user2, user1)))
        }

        async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<PostAuthorResponse>> {
            Ok(Some(PostAuthorResponse {
                id: user_id,
                username: format!("user_{}", user_id),
                full_name: Some("Test User".to_string()),
                avatar_url: None,
            }))
        }
    }

    #[tokio::test]
    async fn test_cannot_add_self_as_close_friend() {
        let repo = Arc::new(MockStoryRepository::new());
        let service = StoryServiceImpl::new(repo);
        let user_id = Uuid::now_v7();

        let result = service.add_close_friend(user_id, user_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_stories_tray_ordering_and_grouping() {
        let repo = Arc::new(MockStoryRepository::new());
        let viewer_id = Uuid::now_v7();
        let friend1_id = Uuid::now_v7();
        let friend2_id = Uuid::now_v7();

        let now = Utc::now();

        // Viewer story (seen)
        let my_story = StoryWithAuthor {
            id: Uuid::now_v7(),
            author_id: viewer_id,
            author_username: "viewer".to_string(),
            author_full_name: None,
            author_avatar_url: None,
            media_type: "image".to_string(),
            media_url: "url0".to_string(),
            thumbnail_url: None,
            caption: None,
            duration: 5.0,
            visibility: "friends".to_string(),
            background_color: None,
            metadata: None,
            created_at: now - chrono::Duration::hours(2),
            expires_at: now + chrono::Duration::hours(22),
            view_count: 5,
            has_viewed: true,
            viewer_reaction: None,
            is_close_friend: true,
        };

        // Friend 1 story (all seen)
        let friend1_story = StoryWithAuthor {
            id: Uuid::now_v7(),
            author_id: friend1_id,
            author_username: "friend1".to_string(),
            author_full_name: None,
            author_avatar_url: None,
            media_type: "image".to_string(),
            media_url: "url1".to_string(),
            thumbnail_url: None,
            caption: None,
            duration: 5.0,
            visibility: "friends".to_string(),
            background_color: None,
            metadata: None,
            created_at: now - chrono::Duration::hours(1),
            expires_at: now + chrono::Duration::hours(23),
            view_count: 10,
            has_viewed: true,
            viewer_reaction: None,
            is_close_friend: false,
        };

        // Friend 2 story (unseen + close_friends!)
        let friend2_story = StoryWithAuthor {
            id: Uuid::now_v7(),
            author_id: friend2_id,
            author_username: "friend2".to_string(),
            author_full_name: None,
            author_avatar_url: None,
            media_type: "video".to_string(),
            media_url: "url2".to_string(),
            thumbnail_url: None,
            caption: Some("Secret story".to_string()),
            duration: 15.0,
            visibility: "close_friends".to_string(),
            background_color: None,
            metadata: None,
            created_at: now - chrono::Duration::minutes(30),
            expires_at: now + chrono::Duration::hours(23),
            view_count: 2,
            has_viewed: false,
            viewer_reaction: None,
            is_close_friend: true,
        };

        repo.stories.lock().unwrap().extend(vec![friend1_story, friend2_story, my_story]);

        let service = StoryServiceImpl::new(repo);
        let tray = service.get_stories_tray(viewer_id).await.unwrap();

        assert_eq!(tray.len(), 3);
        // Self is always first in tray
        assert!(tray[0].is_self);
        assert_eq!(tray[0].user.id, viewer_id);

        // Unseen stories come before seen stories
        assert_eq!(tray[1].user.id, friend2_id);
        assert!(tray[1].has_unseen);
        assert!(tray[1].has_close_friends_story); // Green ring indicator!

        // All seen stories come after
        assert_eq!(tray[2].user.id, friend1_id);
        assert!(!tray[2].has_unseen);
        assert!(!tray[2].has_close_friends_story);
    }

    #[tokio::test]
    async fn test_close_friends_story_permission_enforcement() {
        let repo = Arc::new(MockStoryRepository::new());
        let author_id = Uuid::now_v7();
        let stranger_id = Uuid::now_v7();
        let close_friend_id = Uuid::now_v7();

        let story_id = Uuid::now_v7();
        let story = StoryWithAuthor {
            id: story_id,
            author_id,
            author_username: "author".to_string(),
            author_full_name: None,
            author_avatar_url: None,
            media_type: "image".to_string(),
            media_url: "url".to_string(),
            thumbnail_url: None,
            caption: None,
            duration: 5.0,
            visibility: "close_friends".to_string(),
            background_color: None,
            metadata: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            view_count: 0,
            has_viewed: false,
            viewer_reaction: None,
            is_close_friend: false,
        };

        repo.stories.lock().unwrap().push(story);
        repo.close_friends.lock().unwrap().push((author_id, close_friend_id));

        let service = StoryServiceImpl::new(repo);

        // Stranger should be rejected
        let res_stranger = service.get_story(stranger_id, story_id).await;
        assert!(res_stranger.is_err());

        // Close friend should succeed
        let res_friend = service.get_story(close_friend_id, story_id).await;
        assert!(res_friend.is_ok());

        // Author viewing their own story should succeed
        let res_author = service.get_story(author_id, story_id).await;
        assert!(res_author.is_ok());
    }

    #[tokio::test]
    async fn test_mark_story_viewed_recording() {
        let repo = Arc::new(MockStoryRepository::new());
        let author_id = Uuid::now_v7();
        let viewer_id = Uuid::now_v7();
        let story_id = Uuid::now_v7();

        let story = StoryWithAuthor {
            id: story_id,
            author_id,
            author_username: "author".to_string(),
            author_full_name: None,
            author_avatar_url: None,
            media_type: "image".to_string(),
            media_url: "url".to_string(),
            thumbnail_url: None,
            caption: None,
            duration: 5.0,
            visibility: "public".to_string(),
            background_color: None,
            metadata: None,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
            view_count: 0,
            has_viewed: false,
            viewer_reaction: None,
            is_close_friend: false,
        };

        repo.stories.lock().unwrap().push(story);
        let service = StoryServiceImpl::new(repo.clone());

        // Author viewing own story shouldn't record a view
        let _ = service.mark_story_viewed(author_id, story_id).await;
        assert_eq!(repo.recorded_views.lock().unwrap().len(), 0);

        // Another viewer viewing story records a view
        let _ = service.mark_story_viewed(viewer_id, story_id).await;
        assert_eq!(repo.recorded_views.lock().unwrap().len(), 1);
        assert_eq!(repo.recorded_views.lock().unwrap()[0], (story_id, viewer_id));
    }
}

