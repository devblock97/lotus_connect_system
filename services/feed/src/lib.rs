use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use database::PgPool;
use errors::{AppError, Result};
use models::{Post, PostWithAuthor, CommentWithAuthor};
use dto::{
    CreatePostRequest, UpdatePostRequest, PostResponse, PostAuthorResponse,
    CreateCommentRequest, CommentResponse, PostReactionDetailResponse
};

#[async_trait::async_trait]
pub trait FeedRepository: Send + Sync {
    async fn create_post(
        &self,
        id: Uuid,
        author_id: Uuid,
        content: &str,
        media_items: Option<sqlx::types::Json<Vec<models::MediaItem>>>,
        visibility: &str,
    ) -> Result<Post>;

    async fn find_post_by_id(&self, post_id: Uuid, viewer_id: Uuid) -> Result<Option<PostWithAuthor>>;

    async fn update_post(
        &self,
        post_id: Uuid,
        content: Option<&str>,
        media_items: Option<sqlx::types::Json<Vec<models::MediaItem>>>,
        visibility: Option<&str>,
    ) -> Result<Post>;

    async fn delete_post(&self, post_id: Uuid) -> Result<()>;

    async fn list_home_feed(
        &self,
        viewer_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<PostWithAuthor>>;

    async fn list_explore_feed(
        &self,
        viewer_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<PostWithAuthor>>;

    async fn list_user_posts(
        &self,
        viewer_id: Uuid,
        author_id: Uuid,
        is_friend: bool,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<PostWithAuthor>>;

    async fn add_or_update_reaction(
        &self,
        id: Uuid,
        post_id: Uuid,
        user_id: Uuid,
        reaction: &str,
    ) -> Result<models::PostReaction>;

    async fn remove_reaction(&self, post_id: Uuid, user_id: Uuid) -> Result<()>;

    async fn list_post_reactions(&self, post_id: Uuid) -> Result<Vec<PostReactionDetailResponse>>;

    async fn create_comment(
        &self,
        id: Uuid,
        post_id: Uuid,
        user_id: Uuid,
        parent_comment_id: Option<Uuid>,
        content: &str,
    ) -> Result<CommentWithAuthor>;

    async fn find_comment_by_id(&self, comment_id: Uuid) -> Result<Option<models::PostComment>>;

    async fn delete_comment(&self, comment_id: Uuid, post_id: Uuid) -> Result<()>;

    async fn list_comments(&self, post_id: Uuid) -> Result<Vec<CommentWithAuthor>>;

    async fn is_friend(&self, user1: Uuid, user2: Uuid) -> Result<bool>;
}

#[async_trait::async_trait]
pub trait FeedService: Send + Sync {
    async fn create_post(&self, author_id: Uuid, req: CreatePostRequest) -> Result<PostResponse>;
    async fn get_post(&self, viewer_id: Uuid, post_id: Uuid) -> Result<PostResponse>;
    async fn update_post(&self, user_id: Uuid, post_id: Uuid, req: UpdatePostRequest) -> Result<PostResponse>;
    async fn delete_post(&self, user_id: Uuid, post_id: Uuid) -> Result<()>;

    async fn get_home_feed(&self, viewer_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<PostResponse>>;
    async fn get_explore_feed(&self, viewer_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<PostResponse>>;
    async fn get_user_posts(&self, viewer_id: Uuid, author_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<PostResponse>>;

    async fn add_reaction(&self, user_id: Uuid, post_id: Uuid, reaction: &str) -> Result<PostReactionDetailResponse>;
    async fn remove_reaction(&self, user_id: Uuid, post_id: Uuid) -> Result<()>;
    async fn get_post_reactions(&self, post_id: Uuid) -> Result<Vec<PostReactionDetailResponse>>;

    async fn add_comment(&self, user_id: Uuid, post_id: Uuid, req: CreateCommentRequest) -> Result<CommentResponse>;
    async fn get_post_comments(&self, viewer_id: Uuid, post_id: Uuid) -> Result<Vec<CommentResponse>>;
    async fn delete_comment(&self, user_id: Uuid, post_id: Uuid, comment_id: Uuid) -> Result<()>;
}

pub struct FeedRepositoryImpl {
    pool: PgPool,
}

impl FeedRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl FeedRepository for FeedRepositoryImpl {
    async fn create_post(
        &self,
        id: Uuid,
        author_id: Uuid,
        content: &str,
        media_items: Option<sqlx::types::Json<Vec<models::MediaItem>>>,
        visibility: &str,
    ) -> Result<Post> {
        sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (id, author_id, content, media_items, visibility)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, author_id, content, media_items, visibility, like_count, comment_count, created_at, updated_at
            "#
        )
        .bind(id)
        .bind(author_id)
        .bind(content)
        .bind(media_items)
        .bind(visibility)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_post_by_id(&self, post_id: Uuid, viewer_id: Uuid) -> Result<Option<PostWithAuthor>> {
        sqlx::query_as::<_, PostWithAuthor>(
            r#"
            SELECT 
                p.id,
                p.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                p.content,
                p.media_items,
                p.visibility,
                p.like_count,
                p.comment_count,
                p.created_at,
                p.updated_at,
                (pr.id IS NOT NULL) AS user_has_liked,
                pr.reaction AS user_reaction
            FROM posts p
            JOIN users u ON p.author_id = u.id
            LEFT JOIN post_reactions pr ON pr.post_id = p.id AND pr.user_id = $2
            WHERE p.id = $1
            "#
        )
        .bind(post_id)
        .bind(viewer_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn update_post(
        &self,
        post_id: Uuid,
        content: Option<&str>,
        media_items: Option<sqlx::types::Json<Vec<models::MediaItem>>>,
        visibility: Option<&str>,
    ) -> Result<Post> {
        sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts
            SET 
                content = COALESCE($2, content),
                media_items = COALESCE($3, media_items),
                visibility = COALESCE($4, visibility),
                updated_at = NOW()
            WHERE id = $1
            RETURNING id, author_id, content, media_items, visibility, like_count, comment_count, created_at, updated_at
            "#
        )
        .bind(post_id)
        .bind(content)
        .bind(media_items)
        .bind(visibility)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn delete_post(&self, post_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(post_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn list_home_feed(
        &self,
        viewer_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<PostWithAuthor>> {
        let cursor_time = if let Some(cid) = cursor {
            sqlx::query_scalar::<_, DateTime<Utc>>("SELECT created_at FROM posts WHERE id = $1")
                .bind(cid)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            None
        };

        sqlx::query_as::<_, PostWithAuthor>(
            r#"
            SELECT 
                p.id,
                p.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                p.content,
                p.media_items,
                p.visibility,
                p.like_count,
                p.comment_count,
                p.created_at,
                p.updated_at,
                (pr.id IS NOT NULL) AS user_has_liked,
                pr.reaction AS user_reaction
            FROM posts p
            JOIN users u ON p.author_id = u.id
            LEFT JOIN post_reactions pr ON pr.post_id = p.id AND pr.user_id = $1
            WHERE (
                p.author_id = $1
                OR (
                    p.author_id IN (
                        SELECT friend_id FROM friendships WHERE user_id = $1 AND status = 'accepted'
                        UNION
                        SELECT user_id FROM friendships WHERE friend_id = $1 AND status = 'accepted'
                    )
                    AND p.visibility IN ('public', 'friends')
                )
                OR p.visibility = 'public'
            )
            AND ($2::timestamptz IS NULL OR p.created_at < $2 OR (p.created_at = $2 AND p.id < $3))
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $4
            "#
        )
        .bind(viewer_id)
        .bind(cursor_time)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_explore_feed(
        &self,
        viewer_id: Uuid,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<PostWithAuthor>> {
        let cursor_time = if let Some(cid) = cursor {
            sqlx::query_scalar::<_, DateTime<Utc>>("SELECT created_at FROM posts WHERE id = $1")
                .bind(cid)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            None
        };

        sqlx::query_as::<_, PostWithAuthor>(
            r#"
            SELECT 
                p.id,
                p.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                p.content,
                p.media_items,
                p.visibility,
                p.like_count,
                p.comment_count,
                p.created_at,
                p.updated_at,
                (pr.id IS NOT NULL) AS user_has_liked,
                pr.reaction AS user_reaction
            FROM posts p
            JOIN users u ON p.author_id = u.id
            LEFT JOIN post_reactions pr ON pr.post_id = p.id AND pr.user_id = $1
            WHERE p.visibility = 'public'
            AND ($2::timestamptz IS NULL OR p.created_at < $2 OR (p.created_at = $2 AND p.id < $3))
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $4
            "#
        )
        .bind(viewer_id)
        .bind(cursor_time)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_user_posts(
        &self,
        viewer_id: Uuid,
        author_id: Uuid,
        is_friend: bool,
        cursor: Option<Uuid>,
        limit: i64,
    ) -> Result<Vec<PostWithAuthor>> {
        let cursor_time = if let Some(cid) = cursor {
            sqlx::query_scalar::<_, DateTime<Utc>>("SELECT created_at FROM posts WHERE id = $1")
                .bind(cid)
                .fetch_optional(&self.pool)
                .await
                .map_err(AppError::Database)?
        } else {
            None
        };

        let is_self = viewer_id == author_id;

        sqlx::query_as::<_, PostWithAuthor>(
            r#"
            SELECT 
                p.id,
                p.author_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                p.content,
                p.media_items,
                p.visibility,
                p.like_count,
                p.comment_count,
                p.created_at,
                p.updated_at,
                (pr.id IS NOT NULL) AS user_has_liked,
                pr.reaction AS user_reaction
            FROM posts p
            JOIN users u ON p.author_id = u.id
            LEFT JOIN post_reactions pr ON pr.post_id = p.id AND pr.user_id = $1
            WHERE p.author_id = $2
            AND (
                $3 = true -- is_self, see all
                OR ($4 = true AND p.visibility IN ('public', 'friends')) -- is_friend
                OR p.visibility = 'public' -- others
            )
            AND ($5::timestamptz IS NULL OR p.created_at < $5 OR (p.created_at = $5 AND p.id < $6))
            ORDER BY p.created_at DESC, p.id DESC
            LIMIT $7
            "#
        )
        .bind(viewer_id)
        .bind(author_id)
        .bind(is_self)
        .bind(is_friend)
        .bind(cursor_time)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn add_or_update_reaction(
        &self,
        id: Uuid,
        post_id: Uuid,
        user_id: Uuid,
        reaction: &str,
    ) -> Result<models::PostReaction> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        let existing = sqlx::query_as::<_, models::PostReaction>(
            "SELECT id, post_id, user_id, reaction, created_at FROM post_reactions WHERE post_id = $1 AND user_id = $2"
        )
        .bind(post_id)
        .bind(user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        let result = match existing {
            Some(curr) => {
                if curr.reaction != reaction {
                    sqlx::query_as::<_, models::PostReaction>(
                        "UPDATE post_reactions SET reaction = $1 WHERE id = $2 RETURNING id, post_id, user_id, reaction, created_at"
                    )
                    .bind(reaction)
                    .bind(curr.id)
                    .fetch_one(&mut *tx)
                    .await
                    .map_err(AppError::Database)?
                } else {
                    curr
                }
            }
            None => {
                let inserted = sqlx::query_as::<_, models::PostReaction>(
                    "INSERT INTO post_reactions (id, post_id, user_id, reaction) VALUES ($1, $2, $3, $4) RETURNING id, post_id, user_id, reaction, created_at"
                )
                .bind(id)
                .bind(post_id)
                .bind(user_id)
                .bind(reaction)
                .fetch_one(&mut *tx)
                .await
                .map_err(AppError::Database)?;

                sqlx::query("UPDATE posts SET like_count = like_count + 1 WHERE id = $1")
                    .bind(post_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(AppError::Database)?;

                inserted
            }
        };

        tx.commit().await.map_err(AppError::Database)?;
        Ok(result)
    }

    async fn remove_reaction(&self, post_id: Uuid, user_id: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        let res = sqlx::query("DELETE FROM post_reactions WHERE post_id = $1 AND user_id = $2")
            .bind(post_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        if res.rows_affected() > 0 {
            sqlx::query("UPDATE posts SET like_count = GREATEST(0, like_count - 1) WHERE id = $1")
                .bind(post_id)
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;
        }

        tx.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    async fn list_post_reactions(&self, post_id: Uuid) -> Result<Vec<PostReactionDetailResponse>> {
        #[derive(sqlx::FromRow)]
        struct ReactionRow {
            id: Uuid,
            post_id: Uuid,
            user_id: Uuid,
            username: String,
            full_name: Option<String>,
            avatar_url: Option<String>,
            reaction: String,
            created_at: DateTime<Utc>,
        }

        let rows = sqlx::query_as::<_, ReactionRow>(
            r#"
            SELECT 
                pr.id,
                pr.post_id,
                pr.user_id,
                u.username,
                u.full_name,
                u.avatar_url,
                pr.reaction,
                pr.created_at
            FROM post_reactions pr
            JOIN users u ON pr.user_id = u.id
            WHERE pr.post_id = $1
            ORDER BY pr.created_at DESC
            "#
        )
        .bind(post_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(rows.into_iter().map(|r| PostReactionDetailResponse {
            id: r.id,
            post_id: r.post_id,
            user: PostAuthorResponse {
                id: r.user_id,
                username: r.username,
                full_name: r.full_name,
                avatar_url: r.avatar_url,
            },
            reaction: r.reaction,
            created_at: r.created_at,
        }).collect())
    }

    async fn create_comment(
        &self,
        id: Uuid,
        post_id: Uuid,
        user_id: Uuid,
        parent_comment_id: Option<Uuid>,
        content: &str,
    ) -> Result<CommentWithAuthor> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        sqlx::query(
            "INSERT INTO post_comments (id, post_id, user_id, parent_comment_id, content) VALUES ($1, $2, $3, $4, $5)"
        )
        .bind(id)
        .bind(post_id)
        .bind(user_id)
        .bind(parent_comment_id)
        .bind(content)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        sqlx::query("UPDATE posts SET comment_count = comment_count + 1 WHERE id = $1")
            .bind(post_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        let comment = sqlx::query_as::<_, CommentWithAuthor>(
            r#"
            SELECT 
                c.id,
                c.post_id,
                c.user_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                c.parent_comment_id,
                c.content,
                c.created_at,
                c.updated_at
            FROM post_comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.id = $1
            "#
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;
        Ok(comment)
    }

    async fn find_comment_by_id(&self, comment_id: Uuid) -> Result<Option<models::PostComment>> {
        sqlx::query_as::<_, models::PostComment>(
            "SELECT id, post_id, user_id, parent_comment_id, content, created_at, updated_at FROM post_comments WHERE id = $1"
        )
        .bind(comment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn delete_comment(&self, comment_id: Uuid, post_id: Uuid) -> Result<()> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        let res = sqlx::query("DELETE FROM post_comments WHERE id = $1")
            .bind(comment_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        if res.rows_affected() > 0 {
            sqlx::query("UPDATE posts SET comment_count = GREATEST(0, comment_count - $1) WHERE id = $2")
                .bind(res.rows_affected() as i64)
                .bind(post_id)
                .execute(&mut *tx)
                .await
                .map_err(AppError::Database)?;
        }

        tx.commit().await.map_err(AppError::Database)?;
        Ok(())
    }

    async fn list_comments(&self, post_id: Uuid) -> Result<Vec<CommentWithAuthor>> {
        sqlx::query_as::<_, CommentWithAuthor>(
            r#"
            SELECT 
                c.id,
                c.post_id,
                c.user_id,
                u.username AS author_username,
                u.full_name AS author_full_name,
                u.avatar_url AS author_avatar_url,
                c.parent_comment_id,
                c.content,
                c.created_at,
                c.updated_at
            FROM post_comments c
            JOIN users u ON c.user_id = u.id
            WHERE c.post_id = $1
            ORDER BY c.created_at ASC
            "#
        )
        .bind(post_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
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
}

pub struct FeedServiceImpl {
    repo: Arc<dyn FeedRepository>,
}

impl FeedServiceImpl {
    pub fn new(repo: Arc<dyn FeedRepository>) -> Self {
        Self { repo }
    }
}

fn map_post_to_response(post: PostWithAuthor) -> PostResponse {
    PostResponse {
        id: post.id,
        author: PostAuthorResponse {
            id: post.author_id,
            username: post.author_username,
            full_name: post.author_full_name,
            avatar_url: post.author_avatar_url,
        },
        content: post.content,
        media_items: post.media_items.map(|j| j.0).unwrap_or_default(),
        visibility: post.visibility,
        like_count: post.like_count,
        comment_count: post.comment_count,
        user_has_liked: post.user_has_liked,
        user_reaction: post.user_reaction,
        created_at: post.created_at,
        updated_at: post.updated_at,
    }
}

fn map_comment_to_response(c: CommentWithAuthor) -> CommentResponse {
    CommentResponse {
        id: c.id,
        post_id: c.post_id,
        author: PostAuthorResponse {
            id: c.user_id,
            username: c.author_username,
            full_name: c.author_full_name,
            avatar_url: c.author_avatar_url,
        },
        parent_comment_id: c.parent_comment_id,
        content: c.content,
        created_at: c.created_at,
        updated_at: c.updated_at,
    }
}

#[async_trait::async_trait]
impl FeedService for FeedServiceImpl {
    async fn create_post(&self, author_id: Uuid, req: CreatePostRequest) -> Result<PostResponse> {
        let content = req.content.unwrap_or_default();
        let visibility = match req.visibility.as_deref() {
            Some("friends") => "friends",
            Some("private") => "private",
            _ => "public",
        };

        let media_items_json = req.media_items.map(sqlx::types::Json);

        let id = Uuid::now_v7();
        self.repo.create_post(id, author_id, &content, media_items_json, visibility).await?;

        let post_with_author = self.repo.find_post_by_id(id, author_id).await?
            .ok_or_else(|| AppError::NotFound("Created post not found".to_string()))?;

        Ok(map_post_to_response(post_with_author))
    }

    async fn get_post(&self, viewer_id: Uuid, post_id: Uuid) -> Result<PostResponse> {
        let post = self.repo.find_post_by_id(post_id, viewer_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        if post.author_id != viewer_id {
            if post.visibility == "private" {
                return Err(AppError::Authorization("You are not authorized to view this post".to_string()));
            }
            if post.visibility == "friends" {
                let is_friend = self.repo.is_friend(viewer_id, post.author_id).await?;
                if !is_friend {
                    return Err(AppError::Authorization("This post is only visible to friends".to_string()));
                }
            }
        }

        Ok(map_post_to_response(post))
    }

    async fn update_post(&self, user_id: Uuid, post_id: Uuid, req: UpdatePostRequest) -> Result<PostResponse> {
        let post = self.repo.find_post_by_id(post_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        if post.author_id != user_id {
            return Err(AppError::Authorization("You can only edit your own posts".to_string()));
        }

        let media_items_json = req.media_items.map(sqlx::types::Json);
        let visibility = req.visibility.as_deref().map(|v| match v {
            "friends" => "friends",
            "private" => "private",
            _ => "public",
        });

        self.repo.update_post(post_id, req.content.as_deref(), media_items_json, visibility).await?;

        let updated = self.repo.find_post_by_id(post_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        Ok(map_post_to_response(updated))
    }

    async fn delete_post(&self, user_id: Uuid, post_id: Uuid) -> Result<()> {
        let post = self.repo.find_post_by_id(post_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        if post.author_id != user_id {
            return Err(AppError::Authorization("You can only delete your own posts".to_string()));
        }

        self.repo.delete_post(post_id).await
    }

    async fn get_home_feed(&self, viewer_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<PostResponse>> {
        let limit = limit.unwrap_or(20).clamp(1, 50);
        let posts = self.repo.list_home_feed(viewer_id, cursor, limit).await?;
        Ok(posts.into_iter().map(map_post_to_response).collect())
    }

    async fn get_explore_feed(&self, viewer_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<PostResponse>> {
        let limit = limit.unwrap_or(20).clamp(1, 50);
        let posts = self.repo.list_explore_feed(viewer_id, cursor, limit).await?;
        Ok(posts.into_iter().map(map_post_to_response).collect())
    }

    async fn get_user_posts(&self, viewer_id: Uuid, author_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<PostResponse>> {
        let limit = limit.unwrap_or(20).clamp(1, 50);
        let is_friend = if viewer_id == author_id {
            true
        } else {
            self.repo.is_friend(viewer_id, author_id).await?
        };

        let posts = self.repo.list_user_posts(viewer_id, author_id, is_friend, cursor, limit).await?;
        Ok(posts.into_iter().map(map_post_to_response).collect())
    }

    async fn add_reaction(&self, user_id: Uuid, post_id: Uuid, reaction: &str) -> Result<PostReactionDetailResponse> {
        let post = self.repo.find_post_by_id(post_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        if post.author_id != user_id && post.visibility != "public" {
            let is_friend = self.repo.is_friend(user_id, post.author_id).await?;
            if !is_friend {
                return Err(AppError::Authorization("You cannot react to this post".to_string()));
            }
        }

        let reaction_id = Uuid::now_v7();
        let r = self.repo.add_or_update_reaction(reaction_id, post_id, user_id, reaction).await?;

        // Retrieve user info for the response
        let reactions = self.repo.list_post_reactions(post_id).await?;
        if let Some(detail) = reactions.into_iter().find(|item| item.user.id == user_id) {
            Ok(detail)
        } else {
            Ok(PostReactionDetailResponse {
                id: r.id,
                post_id: r.post_id,
                user: PostAuthorResponse {
                    id: user_id,
                    username: "".to_string(),
                    full_name: None,
                    avatar_url: None,
                },
                reaction: r.reaction,
                created_at: r.created_at,
            })
        }
    }

    async fn remove_reaction(&self, user_id: Uuid, post_id: Uuid) -> Result<()> {
        self.repo.remove_reaction(post_id, user_id).await
    }

    async fn get_post_reactions(&self, post_id: Uuid) -> Result<Vec<PostReactionDetailResponse>> {
        self.repo.list_post_reactions(post_id).await
    }

    async fn add_comment(&self, user_id: Uuid, post_id: Uuid, req: CreateCommentRequest) -> Result<CommentResponse> {
        let post = self.repo.find_post_by_id(post_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        if post.author_id != user_id && post.visibility != "public" {
            let is_friend = self.repo.is_friend(user_id, post.author_id).await?;
            if !is_friend {
                return Err(AppError::Authorization("You cannot comment on this post".to_string()));
            }
        }

        if let Some(parent_id) = req.parent_comment_id {
            let parent = self.repo.find_comment_by_id(parent_id).await?
                .ok_or_else(|| AppError::NotFound("Parent comment not found".to_string()))?;
            if parent.post_id != post_id {
                return Err(AppError::Validation("Parent comment belongs to a different post".to_string()));
            }
        }

        let id = Uuid::now_v7();
        let comment = self.repo.create_comment(id, post_id, user_id, req.parent_comment_id, &req.content).await?;
        Ok(map_comment_to_response(comment))
    }

    async fn get_post_comments(&self, viewer_id: Uuid, post_id: Uuid) -> Result<Vec<CommentResponse>> {
        let post = self.repo.find_post_by_id(post_id, viewer_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        if post.author_id != viewer_id && post.visibility != "public" {
            let is_friend = self.repo.is_friend(viewer_id, post.author_id).await?;
            if !is_friend {
                return Err(AppError::Authorization("You cannot view comments on this post".to_string()));
            }
        }

        let comments = self.repo.list_comments(post_id).await?;
        Ok(comments.into_iter().map(map_comment_to_response).collect())
    }

    async fn delete_comment(&self, user_id: Uuid, post_id: Uuid, comment_id: Uuid) -> Result<()> {
        let post = self.repo.find_post_by_id(post_id, user_id).await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        let comment = self.repo.find_comment_by_id(comment_id).await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        if comment.post_id != post_id {
            return Err(AppError::Validation("Comment does not belong to this post".to_string()));
        }

        // Author of comment or author of post can delete the comment
        if comment.user_id != user_id && post.author_id != user_id {
            return Err(AppError::Authorization("You are not authorized to delete this comment".to_string()));
        }

        self.repo.delete_comment(comment_id, post_id).await
    }
}

