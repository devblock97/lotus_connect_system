use std::sync::Arc;
use uuid::Uuid;
use chrono::Utc;
use database::PgPool;
use errors::{AppError, Result};
use models::{
    LiveStream, LiveStreamWithHost, LiveStreamViewerWithUser,
    LiveStreamCommentWithUser, LiveStreamGiftWithSender,
    LiveStreamCohostWithUser, LiveStreamLeaderboardItem
};
use dto::{
    CreateLiveStreamRequest, UpdateLiveStreamRequest, EndLiveStreamRequest,
    LiveStreamQuery, LiveStreamResponse, LiveStreamDetailResponse,
    LiveStreamViewerResponse, SendLiveCommentRequest, LiveStreamCommentResponse,
    SendLiveReactionRequest, LiveStreamReactionResponse, SendLiveGiftRequest,
    LiveStreamGiftResponse, LiveStreamLeaderboardResponse, LiveStreamLeaderboardItemResponse,
    RespondLiveCohostRequest, LiveStreamCohostResponse, PostAuthorResponse
};
use rand::{distributions::Alphanumeric, Rng};

#[async_trait::async_trait]
pub trait LiveStreamRepository: Send + Sync {
    async fn create_stream(
        &self,
        id: Uuid,
        host_id: Uuid,
        title: &str,
        description: Option<&str>,
        thumbnail_url: Option<&str>,
        privacy: &str,
        stream_key: &str,
        ingest_url: Option<&str>,
        playback_url: Option<&str>,
        channel_id: Option<&str>,
    ) -> Result<LiveStream>;

    async fn find_stream_by_id(&self, stream_id: Uuid) -> Result<Option<LiveStreamWithHost>>;
    async fn find_stream_by_key(&self, stream_key: &str) -> Result<Option<LiveStreamWithHost>>;
    async fn update_stream(
        &self,
        stream_id: Uuid,
        host_id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        thumbnail_url: Option<Option<&str>>,
        privacy: Option<&str>,
    ) -> Result<LiveStreamWithHost>;

    async fn start_stream(&self, stream_id: Uuid, host_id: Uuid) -> Result<LiveStreamWithHost>;
    async fn end_stream(&self, stream_id: Uuid, host_id: Uuid, recording_url: Option<&str>) -> Result<LiveStreamWithHost>;

    async fn list_streams(&self, status: Option<&str>, cursor: Option<Uuid>, limit: i64) -> Result<Vec<LiveStreamWithHost>>;
    async fn list_user_streams(&self, host_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<LiveStreamWithHost>>;

    async fn join_stream(&self, id: Uuid, stream_id: Uuid, user_id: Uuid) -> Result<i32>;
    async fn leave_stream(&self, stream_id: Uuid, user_id: Uuid) -> Result<i32>;
    async fn list_active_viewers(&self, stream_id: Uuid, limit: i64) -> Result<Vec<LiveStreamViewerWithUser>>;

    async fn add_comment(&self, id: Uuid, stream_id: Uuid, user_id: Uuid, content: &str) -> Result<LiveStreamCommentWithUser>;
    async fn list_comments(&self, stream_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<LiveStreamCommentWithUser>>;
    async fn pin_comment(&self, stream_id: Uuid, host_id: Uuid, comment_id: Uuid) -> Result<()>;
    async fn unpin_comment(&self, stream_id: Uuid, host_id: Uuid) -> Result<()>;

    async fn add_reactions(&self, stream_id: Uuid, count: i32) -> Result<i64>;

    async fn send_gift(
        &self,
        id: Uuid,
        stream_id: Uuid,
        sender_id: Uuid,
        gift_type: &str,
        gift_count: i32,
        coin_amount: i32,
        combo_count: i32,
    ) -> Result<LiveStreamGiftWithSender>;
    async fn list_gifts(&self, stream_id: Uuid, limit: i64) -> Result<Vec<LiveStreamGiftWithSender>>;
    async fn get_gift_leaderboard(&self, stream_id: Uuid, limit: i64) -> Result<Vec<LiveStreamLeaderboardItem>>;

    async fn request_cohost(&self, id: Uuid, stream_id: Uuid, user_id: Uuid) -> Result<LiveStreamCohostWithUser>;
    async fn respond_cohost(&self, stream_id: Uuid, host_id: Uuid, user_id: Uuid, status: &str) -> Result<LiveStreamCohostWithUser>;
    async fn leave_cohost(&self, stream_id: Uuid, user_id: Uuid) -> Result<()>;
    async fn list_cohosts(&self, stream_id: Uuid) -> Result<Vec<LiveStreamCohostWithUser>>;

    async fn find_user_by_id(&self, user_id: Uuid) -> Result<Option<PostAuthorResponse>>;
}

#[async_trait::async_trait]
pub trait LiveStreamService: Send + Sync {
    async fn create_stream(&self, host_id: Uuid, req: CreateLiveStreamRequest, base_url: &str) -> Result<LiveStreamDetailResponse>;
    async fn get_stream(&self, viewer_id: Uuid, stream_id: Uuid) -> Result<LiveStreamDetailResponse>;
    async fn update_stream(&self, host_id: Uuid, stream_id: Uuid, req: UpdateLiveStreamRequest) -> Result<LiveStreamResponse>;
    async fn start_stream(&self, host_id: Uuid, stream_id: Uuid) -> Result<LiveStreamResponse>;
    async fn end_stream(&self, host_id: Uuid, stream_id: Uuid, req: EndLiveStreamRequest) -> Result<LiveStreamResponse>;

    async fn list_streams(&self, viewer_id: Uuid, query: LiveStreamQuery) -> Result<Vec<LiveStreamResponse>>;
    async fn list_user_streams(&self, viewer_id: Uuid, host_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<LiveStreamResponse>>;

    async fn join_stream(&self, viewer_id: Uuid, stream_id: Uuid) -> Result<(i32, PostAuthorResponse)>;
    async fn leave_stream(&self, viewer_id: Uuid, stream_id: Uuid) -> Result<i32>;
    async fn list_viewers(&self, stream_id: Uuid, limit: Option<i64>) -> Result<Vec<LiveStreamViewerResponse>>;

    async fn add_comment(&self, user_id: Uuid, stream_id: Uuid, req: SendLiveCommentRequest) -> Result<LiveStreamCommentResponse>;
    async fn list_comments(&self, stream_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<LiveStreamCommentResponse>>;
    async fn pin_comment(&self, host_id: Uuid, stream_id: Uuid, comment_id: Uuid) -> Result<()>;
    async fn unpin_comment(&self, host_id: Uuid, stream_id: Uuid) -> Result<()>;

    async fn add_reaction(&self, user_id: Uuid, stream_id: Uuid, req: SendLiveReactionRequest) -> Result<LiveStreamReactionResponse>;

    async fn send_gift(&self, user_id: Uuid, stream_id: Uuid, req: SendLiveGiftRequest) -> Result<LiveStreamGiftResponse>;
    async fn get_leaderboard(&self, stream_id: Uuid) -> Result<LiveStreamLeaderboardResponse>;

    async fn request_cohost(&self, user_id: Uuid, stream_id: Uuid) -> Result<LiveStreamCohostResponse>;
    async fn respond_cohost(&self, host_id: Uuid, stream_id: Uuid, req: RespondLiveCohostRequest) -> Result<LiveStreamCohostResponse>;
    async fn leave_cohost(&self, user_id: Uuid, stream_id: Uuid) -> Result<()>;
    async fn list_cohosts(&self, stream_id: Uuid) -> Result<Vec<LiveStreamCohostResponse>>;
}

pub struct LiveStreamRepositoryImpl {
    pool: PgPool,
}

impl LiveStreamRepositoryImpl {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl LiveStreamRepository for LiveStreamRepositoryImpl {
    async fn create_stream(
        &self,
        id: Uuid,
        host_id: Uuid,
        title: &str,
        description: Option<&str>,
        thumbnail_url: Option<&str>,
        privacy: &str,
        stream_key: &str,
        ingest_url: Option<&str>,
        playback_url: Option<&str>,
        channel_id: Option<&str>,
    ) -> Result<LiveStream> {
        sqlx::query_as::<_, LiveStream>(
            r#"
            INSERT INTO live_streams (
                id, host_id, title, description, thumbnail_url,
                privacy, stream_key, ingest_url, playback_url, channel_id,
                status
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 'created')
            RETURNING
                id, host_id, title, description, thumbnail_url,
                status, privacy, stream_key, ingest_url, playback_url, channel_id,
                viewer_count, peak_viewers, total_views, like_count, pinned_comment_id,
                started_at, ended_at, duration_seconds, recording_url,
                created_at, updated_at
            "#
        )
        .bind(id)
        .bind(host_id)
        .bind(title)
        .bind(description)
        .bind(thumbnail_url)
        .bind(privacy)
        .bind(stream_key)
        .bind(ingest_url)
        .bind(playback_url)
        .bind(channel_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_stream_by_id(&self, stream_id: Uuid) -> Result<Option<LiveStreamWithHost>> {
        sqlx::query_as::<_, LiveStreamWithHost>(
            r#"
            SELECT
                s.id, s.host_id,
                u.username AS host_username,
                u.full_name AS host_full_name,
                u.avatar_url AS host_avatar_url,
                s.title, s.description, s.thumbnail_url,
                s.status, s.privacy, s.stream_key,
                s.ingest_url, s.playback_url, s.channel_id,
                s.viewer_count, s.peak_viewers, s.total_views, s.like_count,
                s.pinned_comment_id, s.started_at, s.ended_at,
                s.duration_seconds, s.recording_url,
                s.created_at, s.updated_at
            FROM live_streams s
            JOIN users u ON u.id = s.host_id
            WHERE s.id = $1
            "#
        )
        .bind(stream_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn find_stream_by_key(&self, stream_key: &str) -> Result<Option<LiveStreamWithHost>> {
        sqlx::query_as::<_, LiveStreamWithHost>(
            r#"
            SELECT
                s.id, s.host_id,
                u.username AS host_username,
                u.full_name AS host_full_name,
                u.avatar_url AS host_avatar_url,
                s.title, s.description, s.thumbnail_url,
                s.status, s.privacy, s.stream_key,
                s.ingest_url, s.playback_url, s.channel_id,
                s.viewer_count, s.peak_viewers, s.total_views, s.like_count,
                s.pinned_comment_id, s.started_at, s.ended_at,
                s.duration_seconds, s.recording_url,
                s.created_at, s.updated_at
            FROM live_streams s
            JOIN users u ON u.id = s.host_id
            WHERE s.stream_key = $1
            "#
        )
        .bind(stream_key)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn update_stream(
        &self,
        stream_id: Uuid,
        host_id: Uuid,
        title: Option<&str>,
        description: Option<Option<&str>>,
        thumbnail_url: Option<Option<&str>>,
        privacy: Option<&str>,
    ) -> Result<LiveStreamWithHost> {
        let current = self.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        if current.host_id != host_id {
            return Err(AppError::Authorization("Only the host can update live stream settings".to_string()));
        }

        let new_title = title.unwrap_or(&current.title);
        let new_desc = match description {
            Some(d) => d.map(|s| s.to_string()),
            None => current.description.clone(),
        };
        let new_thumb = match thumbnail_url {
            Some(t) => t.map(|s| s.to_string()),
            None => current.thumbnail_url.clone(),
        };
        let new_privacy = privacy.unwrap_or(&current.privacy);

        sqlx::query_as::<_, LiveStreamWithHost>(
            r#"
            UPDATE live_streams
            SET title = $1, description = $2, thumbnail_url = $3, privacy = $4, updated_at = NOW()
            WHERE id = $5 AND host_id = $6
            RETURNING
                id, host_id,
                $7::VARCHAR AS host_username,
                $8::VARCHAR AS host_full_name,
                $9::VARCHAR AS host_avatar_url,
                title, description, thumbnail_url,
                status, privacy, stream_key,
                ingest_url, playback_url, channel_id,
                viewer_count, peak_viewers, total_views, like_count,
                pinned_comment_id, started_at, ended_at,
                duration_seconds, recording_url,
                created_at, updated_at
            "#
        )
        .bind(new_title)
        .bind(new_desc)
        .bind(new_thumb)
        .bind(new_privacy)
        .bind(stream_id)
        .bind(host_id)
        .bind(&current.host_username)
        .bind(&current.host_full_name)
        .bind(&current.host_avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn start_stream(&self, stream_id: Uuid, host_id: Uuid) -> Result<LiveStreamWithHost> {
        let stream = self.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        if stream.host_id != host_id {
            return Err(AppError::Authorization("Only the host can start the stream".to_string()));
        }

        if stream.status == "ended" {
            return Err(AppError::Validation("Cannot restart an ended live stream. Please create a new stream.".to_string()));
        }

        let updated = sqlx::query_as::<_, LiveStreamWithHost>(
            r#"
            UPDATE live_streams
            SET status = 'live', started_at = COALESCE(started_at, NOW()), updated_at = NOW()
            WHERE id = $1 AND host_id = $2
            RETURNING
                id, host_id,
                $3::VARCHAR AS host_username,
                $4::VARCHAR AS host_full_name,
                $5::VARCHAR AS host_avatar_url,
                title, description, thumbnail_url,
                status, privacy, stream_key,
                ingest_url, playback_url, channel_id,
                viewer_count, peak_viewers, total_views, like_count,
                pinned_comment_id, started_at, ended_at,
                duration_seconds, recording_url,
                created_at, updated_at
            "#
        )
        .bind(stream_id)
        .bind(host_id)
        .bind(&stream.host_username)
        .bind(&stream.host_full_name)
        .bind(&stream.host_avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(updated)
    }

    async fn end_stream(&self, stream_id: Uuid, host_id: Uuid, recording_url: Option<&str>) -> Result<LiveStreamWithHost> {
        let stream = self.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        if stream.host_id != host_id {
            return Err(AppError::Authorization("Only the host can end the stream".to_string()));
        }

        let now = Utc::now();
        let started = stream.started_at.unwrap_or(now);
        let duration = (now - started).num_seconds().max(0) as i32;

        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        // Close all viewer sessions
        sqlx::query("UPDATE live_stream_viewers SET is_active = FALSE, left_at = NOW() WHERE stream_id = $1 AND is_active = TRUE")
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        let updated = sqlx::query_as::<_, LiveStreamWithHost>(
            r#"
            UPDATE live_streams
            SET status = 'ended', ended_at = $1, duration_seconds = $2, recording_url = COALESCE($3, recording_url), viewer_count = 0, updated_at = NOW()
            WHERE id = $4 AND host_id = $5
            RETURNING
                id, host_id,
                $6::VARCHAR AS host_username,
                $7::VARCHAR AS host_full_name,
                $8::VARCHAR AS host_avatar_url,
                title, description, thumbnail_url,
                status, privacy, stream_key,
                ingest_url, playback_url, channel_id,
                viewer_count, peak_viewers, total_views, like_count,
                pinned_comment_id, started_at, ended_at,
                duration_seconds, recording_url,
                created_at, updated_at
            "#
        )
        .bind(now)
        .bind(duration)
        .bind(recording_url)
        .bind(stream_id)
        .bind(host_id)
        .bind(&stream.host_username)
        .bind(&stream.host_full_name)
        .bind(&stream.host_avatar_url)
        .fetch_one(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        Ok(updated)
    }

    async fn list_streams(&self, status: Option<&str>, cursor: Option<Uuid>, limit: i64) -> Result<Vec<LiveStreamWithHost>> {
        let filter_status = status.unwrap_or("live");
        let query_str = if filter_status == "all" {
            r#"
            SELECT
                s.id, s.host_id,
                u.username AS host_username,
                u.full_name AS host_full_name,
                u.avatar_url AS host_avatar_url,
                s.title, s.description, s.thumbnail_url,
                s.status, s.privacy, s.stream_key,
                s.ingest_url, s.playback_url, s.channel_id,
                s.viewer_count, s.peak_viewers, s.total_views, s.like_count,
                s.pinned_comment_id, s.started_at, s.ended_at,
                s.duration_seconds, s.recording_url,
                s.created_at, s.updated_at
            FROM live_streams s
            JOIN users u ON u.id = s.host_id
            WHERE ($1::UUID IS NULL OR s.created_at < (SELECT created_at FROM live_streams WHERE id = $1))
            ORDER BY (s.status = 'live') DESC, s.viewer_count DESC, s.created_at DESC
            LIMIT $2
            "#
        } else {
            r#"
            SELECT
                s.id, s.host_id,
                u.username AS host_username,
                u.full_name AS host_full_name,
                u.avatar_url AS host_avatar_url,
                s.title, s.description, s.thumbnail_url,
                s.status, s.privacy, s.stream_key,
                s.ingest_url, s.playback_url, s.channel_id,
                s.viewer_count, s.peak_viewers, s.total_views, s.like_count,
                s.pinned_comment_id, s.started_at, s.ended_at,
                s.duration_seconds, s.recording_url,
                s.created_at, s.updated_at
            FROM live_streams s
            JOIN users u ON u.id = s.host_id
            WHERE s.status = $3
              AND ($1::UUID IS NULL OR s.created_at < (SELECT created_at FROM live_streams WHERE id = $1))
            ORDER BY s.viewer_count DESC, s.created_at DESC
            LIMIT $2
            "#
        };

        if filter_status == "all" {
            sqlx::query_as::<_, LiveStreamWithHost>(query_str)
                .bind(cursor)
                .bind(limit)
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)
        } else {
            sqlx::query_as::<_, LiveStreamWithHost>(query_str)
                .bind(cursor)
                .bind(limit)
                .bind(filter_status)
                .fetch_all(&self.pool)
                .await
                .map_err(AppError::Database)
        }
    }

    async fn list_user_streams(&self, host_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<LiveStreamWithHost>> {
        sqlx::query_as::<_, LiveStreamWithHost>(
            r#"
            SELECT
                s.id, s.host_id,
                u.username AS host_username,
                u.full_name AS host_full_name,
                u.avatar_url AS host_avatar_url,
                s.title, s.description, s.thumbnail_url,
                s.status, s.privacy, s.stream_key,
                s.ingest_url, s.playback_url, s.channel_id,
                s.viewer_count, s.peak_viewers, s.total_views, s.like_count,
                s.pinned_comment_id, s.started_at, s.ended_at,
                s.duration_seconds, s.recording_url,
                s.created_at, s.updated_at
            FROM live_streams s
            JOIN users u ON u.id = s.host_id
            WHERE s.host_id = $1
              AND ($2::UUID IS NULL OR s.created_at < (SELECT created_at FROM live_streams WHERE id = $2))
            ORDER BY s.created_at DESC
            LIMIT $3
            "#
        )
        .bind(host_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn join_stream(&self, id: Uuid, stream_id: Uuid, user_id: Uuid) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        // Record viewer entry
        sqlx::query(
            r#"
            INSERT INTO live_stream_viewers (id, stream_id, user_id, joined_at, is_active, left_at)
            VALUES ($1, $2, $3, NOW(), TRUE, NULL)
            ON CONFLICT (stream_id, user_id)
            DO UPDATE SET is_active = TRUE, left_at = NULL, joined_at = NOW()
            "#
        )
        .bind(id)
        .bind(stream_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        // Update active count & total views & peak viewers
        let count_row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM live_stream_viewers WHERE stream_id = $1 AND is_active = TRUE")
            .bind(stream_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        let current_count = count_row.0 as i32;

        sqlx::query(
            r#"
            UPDATE live_streams
            SET viewer_count = $1,
                total_views = total_views + 1,
                peak_viewers = GREATEST(peak_viewers, $1),
                updated_at = NOW()
            WHERE id = $2
            "#
        )
        .bind(current_count)
        .bind(stream_id)
        .execute(&mut *tx)
        .await
        .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        Ok(current_count)
    }

    async fn leave_stream(&self, stream_id: Uuid, user_id: Uuid) -> Result<i32> {
        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        sqlx::query("UPDATE live_stream_viewers SET is_active = FALSE, left_at = NOW() WHERE stream_id = $1 AND user_id = $2")
            .bind(stream_id)
            .bind(user_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        let count_row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM live_stream_viewers WHERE stream_id = $1 AND is_active = TRUE")
            .bind(stream_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        let current_count = count_row.0 as i32;

        sqlx::query("UPDATE live_streams SET viewer_count = $1, updated_at = NOW() WHERE id = $2")
            .bind(current_count)
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        Ok(current_count)
    }

    async fn list_active_viewers(&self, stream_id: Uuid, limit: i64) -> Result<Vec<LiveStreamViewerWithUser>> {
        sqlx::query_as::<_, LiveStreamViewerWithUser>(
            r#"
            SELECT
                v.id, v.stream_id, v.user_id,
                u.username, u.full_name, u.avatar_url,
                v.joined_at, v.is_active
            FROM live_stream_viewers v
            JOIN users u ON u.id = v.user_id
            WHERE v.stream_id = $1 AND v.is_active = TRUE
            ORDER BY v.joined_at DESC
            LIMIT $2
            "#
        )
        .bind(stream_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn add_comment(&self, id: Uuid, stream_id: Uuid, user_id: Uuid, content: &str) -> Result<LiveStreamCommentWithUser> {
        let user = self.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        sqlx::query_as::<_, LiveStreamCommentWithUser>(
            r#"
            INSERT INTO live_stream_comments (id, stream_id, user_id, content)
            VALUES ($1, $2, $3, $4)
            RETURNING
                id, stream_id, user_id,
                $5::VARCHAR AS username,
                $6::VARCHAR AS full_name,
                $7::VARCHAR AS avatar_url,
                content, is_pinned, created_at
            "#
        )
        .bind(id)
        .bind(stream_id)
        .bind(user_id)
        .bind(content)
        .bind(&user.username)
        .bind(&user.full_name)
        .bind(&user.avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_comments(&self, stream_id: Uuid, cursor: Option<Uuid>, limit: i64) -> Result<Vec<LiveStreamCommentWithUser>> {
        sqlx::query_as::<_, LiveStreamCommentWithUser>(
            r#"
            SELECT
                c.id, c.stream_id, c.user_id,
                u.username, u.full_name, u.avatar_url,
                c.content, c.is_pinned, c.created_at
            FROM live_stream_comments c
            JOIN users u ON u.id = c.user_id
            WHERE c.stream_id = $1
              AND ($2::UUID IS NULL OR c.created_at > (SELECT created_at FROM live_stream_comments WHERE id = $2))
            ORDER BY c.created_at ASC
            LIMIT $3
            "#
        )
        .bind(stream_id)
        .bind(cursor)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn pin_comment(&self, stream_id: Uuid, host_id: Uuid, comment_id: Uuid) -> Result<()> {
        let stream = self.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        if stream.host_id != host_id {
            return Err(AppError::Authorization("Only the host can pin a comment".to_string()));
        }

        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        sqlx::query("UPDATE live_stream_comments SET is_pinned = FALSE WHERE stream_id = $1")
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        sqlx::query("UPDATE live_stream_comments SET is_pinned = TRUE WHERE id = $1 AND stream_id = $2")
            .bind(comment_id)
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        sqlx::query("UPDATE live_streams SET pinned_comment_id = $1, updated_at = NOW() WHERE id = $2")
            .bind(comment_id)
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        Ok(())
    }

    async fn unpin_comment(&self, stream_id: Uuid, host_id: Uuid) -> Result<()> {
        let stream = self.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        if stream.host_id != host_id {
            return Err(AppError::Authorization("Only the host can unpin a comment".to_string()));
        }

        let mut tx = self.pool.begin().await.map_err(AppError::Database)?;

        sqlx::query("UPDATE live_stream_comments SET is_pinned = FALSE WHERE stream_id = $1")
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        sqlx::query("UPDATE live_streams SET pinned_comment_id = NULL, updated_at = NOW() WHERE id = $1")
            .bind(stream_id)
            .execute(&mut *tx)
            .await
            .map_err(AppError::Database)?;

        tx.commit().await.map_err(AppError::Database)?;

        Ok(())
    }

    async fn add_reactions(&self, stream_id: Uuid, count: i32) -> Result<i64> {
        let row: (i64,) = sqlx::query_as(
            "UPDATE live_streams SET like_count = like_count + $1, updated_at = NOW() WHERE id = $2 RETURNING like_count"
        )
        .bind(count as i64)
        .bind(stream_id)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)?;

        Ok(row.0)
    }

    async fn send_gift(
        &self,
        id: Uuid,
        stream_id: Uuid,
        sender_id: Uuid,
        gift_type: &str,
        gift_count: i32,
        coin_amount: i32,
        combo_count: i32,
    ) -> Result<LiveStreamGiftWithSender> {
        let user = self.find_user_by_id(sender_id).await?
            .ok_or_else(|| AppError::NotFound("Sender not found".to_string()))?;

        sqlx::query_as::<_, LiveStreamGiftWithSender>(
            r#"
            INSERT INTO live_stream_gifts (id, stream_id, sender_id, gift_type, gift_count, coin_amount, combo_count)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING
                id, stream_id, sender_id,
                $8::VARCHAR AS sender_username,
                $9::VARCHAR AS sender_full_name,
                $10::VARCHAR AS sender_avatar_url,
                gift_type, gift_count, coin_amount, combo_count, created_at
            "#
        )
        .bind(id)
        .bind(stream_id)
        .bind(sender_id)
        .bind(gift_type)
        .bind(gift_count)
        .bind(coin_amount)
        .bind(combo_count)
        .bind(&user.username)
        .bind(&user.full_name)
        .bind(&user.avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn list_gifts(&self, stream_id: Uuid, limit: i64) -> Result<Vec<LiveStreamGiftWithSender>> {
        sqlx::query_as::<_, LiveStreamGiftWithSender>(
            r#"
            SELECT
                g.id, g.stream_id, g.sender_id,
                u.username AS sender_username,
                u.full_name AS sender_full_name,
                u.avatar_url AS sender_avatar_url,
                g.gift_type, g.gift_count, g.coin_amount, g.combo_count, g.created_at
            FROM live_stream_gifts g
            JOIN users u ON u.id = g.sender_id
            WHERE g.stream_id = $1
            ORDER BY g.created_at DESC
            LIMIT $2
            "#
        )
        .bind(stream_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn get_gift_leaderboard(&self, stream_id: Uuid, limit: i64) -> Result<Vec<LiveStreamLeaderboardItem>> {
        sqlx::query_as::<_, LiveStreamLeaderboardItem>(
            r#"
            SELECT
                g.sender_id AS user_id,
                u.username,
                u.full_name,
                u.avatar_url,
                SUM(g.coin_amount * g.gift_count)::BIGINT AS total_coins,
                SUM(g.gift_count)::BIGINT AS gift_count
            FROM live_stream_gifts g
            JOIN users u ON u.id = g.sender_id
            WHERE g.stream_id = $1
            GROUP BY g.sender_id, u.username, u.full_name, u.avatar_url
            ORDER BY total_coins DESC
            LIMIT $2
            "#
        )
        .bind(stream_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn request_cohost(&self, id: Uuid, stream_id: Uuid, user_id: Uuid) -> Result<LiveStreamCohostWithUser> {
        let user = self.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        sqlx::query_as::<_, LiveStreamCohostWithUser>(
            r#"
            INSERT INTO live_stream_cohosts (id, stream_id, user_id, status)
            VALUES ($1, $2, $3, 'pending')
            ON CONFLICT (stream_id, user_id)
            DO UPDATE SET status = 'pending', created_at = NOW()
            RETURNING
                id, stream_id, user_id,
                $4::VARCHAR AS username,
                $5::VARCHAR AS full_name,
                $6::VARCHAR AS avatar_url,
                status, created_at
            "#
        )
        .bind(id)
        .bind(stream_id)
        .bind(user_id)
        .bind(&user.username)
        .bind(&user.full_name)
        .bind(&user.avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn respond_cohost(&self, stream_id: Uuid, host_id: Uuid, user_id: Uuid, status: &str) -> Result<LiveStreamCohostWithUser> {
        let stream = self.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        if stream.host_id != host_id {
            return Err(AppError::Authorization("Only the stream host can accept/reject co-hosts".to_string()));
        }

        let user = self.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        sqlx::query_as::<_, LiveStreamCohostWithUser>(
            r#"
            UPDATE live_stream_cohosts
            SET status = $1
            WHERE stream_id = $2 AND user_id = $3
            RETURNING
                id, stream_id, user_id,
                $4::VARCHAR AS username,
                $5::VARCHAR AS full_name,
                $6::VARCHAR AS avatar_url,
                status, created_at
            "#
        )
        .bind(status)
        .bind(stream_id)
        .bind(user_id)
        .bind(&user.username)
        .bind(&user.full_name)
        .bind(&user.avatar_url)
        .fetch_one(&self.pool)
        .await
        .map_err(AppError::Database)
    }

    async fn leave_cohost(&self, stream_id: Uuid, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE live_stream_cohosts SET status = 'ended' WHERE stream_id = $1 AND user_id = $2")
            .bind(stream_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map(|_| ())
            .map_err(AppError::Database)
    }

    async fn list_cohosts(&self, stream_id: Uuid) -> Result<Vec<LiveStreamCohostWithUser>> {
        sqlx::query_as::<_, LiveStreamCohostWithUser>(
            r#"
            SELECT
                c.id, c.stream_id, c.user_id,
                u.username, u.full_name, u.avatar_url,
                c.status, c.created_at
            FROM live_stream_cohosts c
            JOIN users u ON u.id = c.user_id
            WHERE c.stream_id = $1 AND c.status = 'accepted'
            ORDER BY c.created_at ASC
            "#
        )
        .bind(stream_id)
        .fetch_all(&self.pool)
        .await
        .map_err(AppError::Database)
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

pub struct LiveStreamServiceImpl {
    repo: Arc<dyn LiveStreamRepository>,
}

impl LiveStreamServiceImpl {
    pub fn new(repo: Arc<dyn LiveStreamRepository>) -> Self {
        Self { repo }
    }

    fn generate_stream_key() -> String {
        let random_part: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(24)
            .map(char::from)
            .collect();
        format!("live_sk_{}", random_part)
    }

    fn gift_coin_value(gift_type: &str) -> i32 {
        match gift_type.to_lowercase().as_str() {
            "rose" => 1,
            "heart" => 5,
            "coffee" => 10,
            "diamond" => 50,
            "firework" => 100,
            "rocket" => 500,
            "crown" => 1000,
            _ => 1,
        }
    }

    fn to_stream_response(s: LiveStreamWithHost, viewer_id: Uuid) -> LiveStreamResponse {
        LiveStreamResponse {
            id: s.id,
            host: PostAuthorResponse {
                id: s.host_id,
                username: s.host_username,
                full_name: s.host_full_name,
                avatar_url: s.host_avatar_url,
            },
            title: s.title,
            description: s.description,
            thumbnail_url: s.thumbnail_url,
            status: s.status,
            privacy: s.privacy,
            playback_url: s.playback_url,
            channel_id: s.channel_id,
            viewer_count: s.viewer_count,
            peak_viewers: s.peak_viewers,
            total_views: s.total_views,
            like_count: s.like_count,
            pinned_comment_id: s.pinned_comment_id,
            started_at: s.started_at,
            ended_at: s.ended_at,
            duration_seconds: s.duration_seconds,
            recording_url: s.recording_url,
            is_host: s.host_id == viewer_id,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}

#[async_trait::async_trait]
impl LiveStreamService for LiveStreamServiceImpl {
    async fn create_stream(&self, host_id: Uuid, req: CreateLiveStreamRequest, base_url: &str) -> Result<LiveStreamDetailResponse> {
        let stream_id = Uuid::now_v7();
        let stream_key = Self::generate_stream_key();
        let channel_id = format!("live-room-{}", stream_id);
        let ingest_url = format!("rtmp://{}/live/{}", base_url.replace("http://", "").replace("https://", ""), stream_key);
        let playback_url = format!("{}/api/v1/livestreams/{}/play.m3u8", base_url, stream_id);
        let privacy = req.privacy.unwrap_or_else(|| "public".to_string());

        let _ = self.repo.create_stream(
            stream_id,
            host_id,
            &req.title,
            req.description.as_deref(),
            req.thumbnail_url.as_deref(),
            &privacy,
            &stream_key,
            Some(&ingest_url),
            Some(&playback_url),
            Some(&channel_id),
        ).await?;

        let stream_with_host = self.repo.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Created live stream could not be loaded".to_string()))?;

        let stream_resp = Self::to_stream_response(stream_with_host, host_id);

        Ok(LiveStreamDetailResponse {
            stream: stream_resp,
            stream_key: Some(stream_key),
            ingest_url: Some(ingest_url),
        })
    }

    async fn get_stream(&self, viewer_id: Uuid, stream_id: Uuid) -> Result<LiveStreamDetailResponse> {
        let stream = self.repo.find_stream_by_id(stream_id).await?
            .ok_or_else(|| AppError::NotFound("Live stream not found".to_string()))?;

        let is_host = stream.host_id == viewer_id;
        let stream_key = if is_host { Some(stream.stream_key.clone()) } else { None };
        let ingest_url = if is_host { stream.ingest_url.clone() } else { None };

        let stream_resp = Self::to_stream_response(stream, viewer_id);

        Ok(LiveStreamDetailResponse {
            stream: stream_resp,
            stream_key,
            ingest_url,
        })
    }

    async fn update_stream(&self, host_id: Uuid, stream_id: Uuid, req: UpdateLiveStreamRequest) -> Result<LiveStreamResponse> {
        let desc_trimmed = req.description.as_deref().map(|d| if d.trim().is_empty() { None } else { Some(d) });
        let thumb_trimmed = req.thumbnail_url.as_deref().map(|t| if t.trim().is_empty() { None } else { Some(t) });
        let updated = self.repo.update_stream(
            stream_id,
            host_id,
            req.title.as_deref(),
            desc_trimmed,
            thumb_trimmed,
            req.privacy.as_deref(),
        ).await?;

        Ok(Self::to_stream_response(updated, host_id))
    }

    async fn start_stream(&self, host_id: Uuid, stream_id: Uuid) -> Result<LiveStreamResponse> {
        let updated = self.repo.start_stream(stream_id, host_id).await?;
        Ok(Self::to_stream_response(updated, host_id))
    }

    async fn end_stream(&self, host_id: Uuid, stream_id: Uuid, req: EndLiveStreamRequest) -> Result<LiveStreamResponse> {
        let updated = self.repo.end_stream(stream_id, host_id, req.recording_url.as_deref()).await?;
        Ok(Self::to_stream_response(updated, host_id))
    }

    async fn list_streams(&self, viewer_id: Uuid, query: LiveStreamQuery) -> Result<Vec<LiveStreamResponse>> {
        let limit = query.limit.unwrap_or(20).clamp(1, 50);
        let streams = self.repo.list_streams(query.status.as_deref(), query.cursor, limit).await?;

        Ok(streams.into_iter().map(|s| Self::to_stream_response(s, viewer_id)).collect())
    }

    async fn list_user_streams(&self, viewer_id: Uuid, host_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<LiveStreamResponse>> {
        let l = limit.unwrap_or(20).clamp(1, 50);
        let streams = self.repo.list_user_streams(host_id, cursor, l).await?;
        Ok(streams.into_iter().map(|s| Self::to_stream_response(s, viewer_id)).collect())
    }

    async fn join_stream(&self, viewer_id: Uuid, stream_id: Uuid) -> Result<(i32, PostAuthorResponse)> {
        let user = self.repo.find_user_by_id(viewer_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let join_id = Uuid::now_v7();
        let new_count = self.repo.join_stream(join_id, stream_id, viewer_id).await?;
        Ok((new_count, user))
    }

    async fn leave_stream(&self, viewer_id: Uuid, stream_id: Uuid) -> Result<i32> {
        self.repo.leave_stream(stream_id, viewer_id).await
    }

    async fn list_viewers(&self, stream_id: Uuid, limit: Option<i64>) -> Result<Vec<LiveStreamViewerResponse>> {
        let l = limit.unwrap_or(50).clamp(1, 100);
        let viewers = self.repo.list_active_viewers(stream_id, l).await?;
        Ok(viewers.into_iter().map(|v| LiveStreamViewerResponse {
            id: v.id,
            user: PostAuthorResponse {
                id: v.user_id,
                username: v.username,
                full_name: v.full_name,
                avatar_url: v.avatar_url,
            },
            joined_at: v.joined_at,
            is_active: v.is_active,
        }).collect())
    }

    async fn add_comment(&self, user_id: Uuid, stream_id: Uuid, req: SendLiveCommentRequest) -> Result<LiveStreamCommentResponse> {
        let comment_id = Uuid::now_v7();
        let comment = self.repo.add_comment(comment_id, stream_id, user_id, &req.content).await?;
        Ok(LiveStreamCommentResponse {
            id: comment.id,
            stream_id: comment.stream_id,
            user: PostAuthorResponse {
                id: comment.user_id,
                username: comment.username,
                full_name: comment.full_name,
                avatar_url: comment.avatar_url,
            },
            content: comment.content,
            is_pinned: comment.is_pinned,
            created_at: comment.created_at,
        })
    }

    async fn list_comments(&self, stream_id: Uuid, cursor: Option<Uuid>, limit: Option<i64>) -> Result<Vec<LiveStreamCommentResponse>> {
        let l = limit.unwrap_or(50).clamp(1, 100);
        let comments = self.repo.list_comments(stream_id, cursor, l).await?;
        Ok(comments.into_iter().map(|c| LiveStreamCommentResponse {
            id: c.id,
            stream_id: c.stream_id,
            user: PostAuthorResponse {
                id: c.user_id,
                username: c.username,
                full_name: c.full_name,
                avatar_url: c.avatar_url,
            },
            content: c.content,
            is_pinned: c.is_pinned,
            created_at: c.created_at,
        }).collect())
    }

    async fn pin_comment(&self, host_id: Uuid, stream_id: Uuid, comment_id: Uuid) -> Result<()> {
        self.repo.pin_comment(stream_id, host_id, comment_id).await
    }

    async fn unpin_comment(&self, host_id: Uuid, stream_id: Uuid) -> Result<()> {
        self.repo.unpin_comment(stream_id, host_id).await
    }

    async fn add_reaction(&self, user_id: Uuid, stream_id: Uuid, req: SendLiveReactionRequest) -> Result<LiveStreamReactionResponse> {
        let user = self.repo.find_user_by_id(user_id).await?
            .ok_or_else(|| AppError::NotFound("User not found".to_string()))?;

        let count = req.count.unwrap_or(1).clamp(1, 50);
        let reaction = req.reaction.unwrap_or_else(|| "heart".to_string());
        let total_likes = self.repo.add_reactions(stream_id, count).await?;

        Ok(LiveStreamReactionResponse {
            stream_id,
            user,
            reaction,
            count,
            total_likes,
        })
    }

    async fn send_gift(&self, user_id: Uuid, stream_id: Uuid, req: SendLiveGiftRequest) -> Result<LiveStreamGiftResponse> {
        let gift_id = Uuid::now_v7();
        let count = req.count.unwrap_or(1).clamp(1, 100);
        let coin_val = Self::gift_coin_value(&req.gift_type);
        let gift = self.repo.send_gift(
            gift_id,
            stream_id,
            user_id,
            &req.gift_type,
            count,
            coin_val,
            count,
        ).await?;

        Ok(LiveStreamGiftResponse {
            id: gift.id,
            stream_id: gift.stream_id,
            sender: PostAuthorResponse {
                id: gift.sender_id,
                username: gift.sender_username,
                full_name: gift.sender_full_name,
                avatar_url: gift.sender_avatar_url,
            },
            gift_type: gift.gift_type,
            gift_count: gift.gift_count,
            coin_amount: gift.coin_amount,
            combo_count: gift.combo_count,
            created_at: gift.created_at,
        })
    }

    async fn get_leaderboard(&self, stream_id: Uuid) -> Result<LiveStreamLeaderboardResponse> {
        let items = self.repo.get_gift_leaderboard(stream_id, 20).await?;
        let supporters = items.into_iter().map(|item| LiveStreamLeaderboardItemResponse {
            user: PostAuthorResponse {
                id: item.user_id,
                username: item.username,
                full_name: item.full_name,
                avatar_url: item.avatar_url,
            },
            total_coins: item.total_coins,
            gift_count: item.gift_count,
        }).collect();

        Ok(LiveStreamLeaderboardResponse {
            stream_id,
            supporters,
        })
    }

    async fn request_cohost(&self, user_id: Uuid, stream_id: Uuid) -> Result<LiveStreamCohostResponse> {
        let id = Uuid::now_v7();
        let cohost = self.repo.request_cohost(id, stream_id, user_id).await?;
        Ok(LiveStreamCohostResponse {
            id: cohost.id,
            stream_id: cohost.stream_id,
            user: PostAuthorResponse {
                id: cohost.user_id,
                username: cohost.username,
                full_name: cohost.full_name,
                avatar_url: cohost.avatar_url,
            },
            status: cohost.status,
            created_at: cohost.created_at,
        })
    }

    async fn respond_cohost(&self, host_id: Uuid, stream_id: Uuid, req: RespondLiveCohostRequest) -> Result<LiveStreamCohostResponse> {
        let action = if req.action.to_lowercase() == "accept" { "accepted" } else { "rejected" };
        let cohost = self.repo.respond_cohost(stream_id, host_id, req.user_id, action).await?;
        Ok(LiveStreamCohostResponse {
            id: cohost.id,
            stream_id: cohost.stream_id,
            user: PostAuthorResponse {
                id: cohost.user_id,
                username: cohost.username,
                full_name: cohost.full_name,
                avatar_url: cohost.avatar_url,
            },
            status: cohost.status,
            created_at: cohost.created_at,
        })
    }

    async fn leave_cohost(&self, user_id: Uuid, stream_id: Uuid) -> Result<()> {
        self.repo.leave_cohost(stream_id, user_id).await
    }

    async fn list_cohosts(&self, stream_id: Uuid) -> Result<Vec<LiveStreamCohostResponse>> {
        let list = self.repo.list_cohosts(stream_id).await?;
        Ok(list.into_iter().map(|c| LiveStreamCohostResponse {
            id: c.id,
            stream_id: c.stream_id,
            user: PostAuthorResponse {
                id: c.user_id,
                username: c.username,
                full_name: c.full_name,
                avatar_url: c.avatar_url,
            },
            status: c.status,
            created_at: c.created_at,
        }).collect())
    }
}
