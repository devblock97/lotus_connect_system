use std::sync::Arc;
use config_crate::AppConfig;
use dto::{
    CreateLiveStreamRequest, UpdateLiveStreamRequest, EndLiveStreamRequest,
    SendLiveCommentRequest, SendLiveReactionRequest, SendLiveGiftRequest,
    RespondLiveCohostRequest, LiveStreamQuery
};
use service_livestream::{LiveStreamRepositoryImpl, LiveStreamServiceImpl, LiveStreamService};
use uuid::Uuid;

async fn setup_test_context() -> (database::PgPool, Arc<LiveStreamServiceImpl>) {
    let config = AppConfig::load().expect("Failed to load config");
    let pool = database::init_db(&config).await.expect("Failed to connect to db");
    let livestream_repo = Arc::new(LiveStreamRepositoryImpl::new(pool.clone()));
    let livestream_service = Arc::new(LiveStreamServiceImpl::new(livestream_repo));
    (pool, livestream_service)
}

async fn create_test_user(pool: &database::PgPool) -> Uuid {
    let user_id = Uuid::now_v7();
    let username = format!("streamer_{}", user_id.simple());
    let email = format!("streamer_{}@example.com", user_id.simple());
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, full_name, avatar_url) VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(user_id)
    .bind(&username)
    .bind(&email)
    .bind("dummy_hash")
    .bind(format!("Full Name {}", &username))
    .bind("https://lotus.local/avatar.png")
    .execute(pool)
    .await
    .expect("Failed to create test user");

    user_id
}

async fn cleanup_user(pool: &database::PgPool, user_id: Uuid) {
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

#[tokio::test]
async fn test_livestream_full_lifecycle_tiktok_youtube_facebook_style() {
    let (pool, service) = setup_test_context().await;

    let host_id = create_test_user(&pool).await;
    let viewer1_id = create_test_user(&pool).await;
    let viewer2_id = create_test_user(&pool).await;

    // Host creates a live stream session
    let create_req = CreateLiveStreamRequest {
        title: "Gaming & Music Chill Room 🎮🎵".to_string(),
        description: Some("Hang out with me as we play and answer viewer questions live!".to_string()),
        thumbnail_url: Some("https://lotus.local/thumbnails/stream1.jpg".to_string()),
        privacy: Some("public".to_string()),
    };

    let stream_detail = service.create_stream(host_id, create_req, "http://localhost:8080")
        .await
        .expect("Failed to create live stream");

    let stream_id = stream_detail.stream.id;
    assert_eq!(stream_detail.stream.title, "Gaming & Music Chill Room 🎮🎵");
    assert_eq!(stream_detail.stream.status, "created");
    assert!(stream_detail.stream.is_host);
    assert!(stream_detail.stream_key.is_some());
    assert!(stream_detail.stream_key.as_ref().unwrap().starts_with("live_sk_"));
    assert!(stream_detail.ingest_url.is_some());

    // Viewer fetches live stream details (verifies sanitized view: stream_key is hidden)
    let viewer_view = service.get_stream(viewer1_id, stream_id)
        .await
        .expect("Failed to fetch stream details as viewer");
    assert!(!viewer_view.stream.is_host);
    assert_eq!(viewer_view.stream_key, None);
    assert_eq!(viewer_view.ingest_url, None);

    // Host updates stream metadata
    let update_req = UpdateLiveStreamRequest {
        title: Some("Gaming & Music Chill Room [Q&A Live] 🎮🎵".to_string()),
        description: None,
        thumbnail_url: None,
        privacy: None,
    };
    let updated_stream = service.update_stream(host_id, stream_id, update_req)
        .await
        .expect("Failed to update stream");
    assert_eq!(updated_stream.title, "Gaming & Music Chill Room [Q&A Live] 🎮🎵");

    // Host goes live (Start Stream)
    let started_stream = service.start_stream(host_id, stream_id)
        .await
        .expect("Failed to start stream");
    assert_eq!(started_stream.status, "live");
    assert!(started_stream.started_at.is_some());

    // Viewers join the live stream
    let (v1_count, _u1) = service.join_stream(viewer1_id, stream_id)
        .await
        .expect("Viewer 1 failed to join");
    assert_eq!(v1_count, 1);

    let (v2_count, _u2) = service.join_stream(viewer2_id, stream_id)
        .await
        .expect("Viewer 2 failed to join");
    assert_eq!(v2_count, 2);

    let viewers = service.list_viewers(stream_id, Some(10))
        .await
        .expect("Failed to list active viewers");
    assert_eq!(viewers.len(), 2);

    // Viewers send live comments in chat
    let comment1 = service.add_comment(viewer1_id, stream_id, SendLiveCommentRequest {
        content: "Hello everyone! Loving the background music! 🔥".to_string(),
    }).await.expect("Failed to send live comment");
    assert_eq!(comment1.content, "Hello everyone! Loving the background music! 🔥");
    assert!(!comment1.is_pinned);

    let comment2 = service.add_comment(viewer2_id, stream_id, SendLiveCommentRequest {
        content: "What game are you playing next?".to_string(),
    }).await.expect("Failed to send second comment");

    let comments = service.list_comments(stream_id, None, Some(50))
        .await
        .expect("Failed to list live comments");
    assert_eq!(comments.len(), 2);

    // Host pins a comment (SuperChat / Highlight announcement)
    service.pin_comment(host_id, stream_id, comment2.id)
        .await
        .expect("Host failed to pin comment");

    let stream_refreshed = service.get_stream(viewer1_id, stream_id)
        .await
        .expect("Failed to get refreshed stream");
    assert_eq!(stream_refreshed.stream.pinned_comment_id, Some(comment2.id));

    // Host unpins comment
    service.unpin_comment(host_id, stream_id)
        .await
        .expect("Host failed to unpin comment");

    let stream_unpinned = service.get_stream(viewer1_id, stream_id)
        .await
        .expect("Failed to get unpinned stream");
    assert_eq!(stream_unpinned.stream.pinned_comment_id, None);

    // Viewers send floating hearts / reactions burst
    let react_res = service.add_reaction(viewer1_id, stream_id, SendLiveReactionRequest {
        reaction: Some("heart".to_string()),
        count: Some(15),
    }).await.expect("Failed to send reaction burst");
    assert_eq!(react_res.count, 15);
    assert_eq!(react_res.total_likes, 15);

    // Viewers send virtual gifts (TikTok / Facebook Stars style)
    let gift1 = service.send_gift(viewer1_id, stream_id, SendLiveGiftRequest {
        gift_type: "rose".to_string(),
        count: Some(10),
    }).await.expect("Failed to send rose gift");
    assert_eq!(gift1.gift_type, "rose");
    assert_eq!(gift1.gift_count, 10);
    assert_eq!(gift1.coin_amount, 1);

    let gift2 = service.send_gift(viewer2_id, stream_id, SendLiveGiftRequest {
        gift_type: "rocket".to_string(),
        count: Some(1),
    }).await.expect("Failed to send rocket gift");
    assert_eq!(gift2.gift_type, "rocket");
    assert_eq!(gift2.coin_amount, 500);

    // Check Gift Leaderboard
    let leaderboard = service.get_leaderboard(stream_id)
        .await
        .expect("Failed to get leaderboard");
    assert_eq!(leaderboard.supporters.len(), 2);
    // Viewer 2 (Rocket: 500 coins) should be #1 top supporter
    assert_eq!(leaderboard.supporters[0].user.id, viewer2_id);
    assert_eq!(leaderboard.supporters[0].total_coins, 500);
    // Viewer 1 (10 Roses: 10 coins) should be #2
    assert_eq!(leaderboard.supporters[1].user.id, viewer1_id);
    assert_eq!(leaderboard.supporters[1].total_coins, 10);

    // Co-Hosting / Multi-Guest request and response (TikTok PK style)
    let cohost_req = service.request_cohost(viewer1_id, stream_id)
        .await
        .expect("Failed to request co-host");
    assert_eq!(cohost_req.status, "pending");

    let cohost_accepted = service.respond_cohost(host_id, stream_id, RespondLiveCohostRequest {
        user_id: viewer1_id,
        action: "accept".to_string(),
    }).await.expect("Failed to accept cohost");
    assert_eq!(cohost_accepted.status, "accepted");

    let active_cohosts = service.list_cohosts(stream_id)
        .await
        .expect("Failed to list active cohosts");
    assert_eq!(active_cohosts.len(), 1);
    assert_eq!(active_cohosts[0].user.id, viewer1_id);

    // Viewer leaves stream
    let remaining_count = service.leave_stream(viewer1_id, stream_id)
        .await
        .expect("Viewer failed to leave");
    assert_eq!(remaining_count, 1);

    // Host ends the live stream
    let ended_stream = service.end_stream(host_id, stream_id, EndLiveStreamRequest {
        recording_url: Some("https://lotus.local/recordings/stream1.mp4".to_string()),
    }).await.expect("Failed to end stream");

    assert_eq!(ended_stream.status, "ended");
    assert!(ended_stream.ended_at.is_some());
    assert_eq!(ended_stream.recording_url, Some("https://lotus.local/recordings/stream1.mp4".to_string()));
    assert_eq!(ended_stream.viewer_count, 0);
    assert_eq!(ended_stream.peak_viewers, 2);
    assert_eq!(ended_stream.like_count, 15);

    // Explore feed lists streams
    let explore_streams = service.list_streams(viewer1_id, LiveStreamQuery {
        status: Some("all".to_string()),
        cursor: None,
        limit: Some(10),
    }).await.expect("Failed to list explore streams");
    assert!(explore_streams.iter().any(|s| s.id == stream_id));

    // Cleanup
    cleanup_user(&pool, host_id).await;
    cleanup_user(&pool, viewer1_id).await;
    cleanup_user(&pool, viewer2_id).await;
}
