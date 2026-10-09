use std::sync::Arc;
use config_crate::AppConfig;
use dto::{
    CreatePostRequest, CreateCommentRequest, UpdateCommentRequest, CommentQuery
};
use service_feed::{FeedRepositoryImpl, FeedServiceImpl, FeedService};
use uuid::Uuid;

async fn setup_test_context() -> (database::PgPool, Arc<FeedServiceImpl>) {
    let config = AppConfig::load().expect("Failed to load config");
    let pool = database::init_db(&config).await.expect("Failed to connect to db");
    let feed_repo = Arc::new(FeedRepositoryImpl::new(pool.clone()));
    let feed_service = Arc::new(FeedServiceImpl::new(feed_repo));
    (pool, feed_service)
}

async fn create_test_user(pool: &database::PgPool) -> Uuid {
    let user_id = Uuid::now_v7();
    let username = format!("user_{}", user_id.simple());
    let email = format!("user_{}@example.com", user_id.simple());
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
async fn test_comment_lifecycle_facebook_tiktok_instagram_style() {
    let (pool, feed_service) = setup_test_context().await;

    let author_id = create_test_user(&pool).await;
    let commenter_id = create_test_user(&pool).await;

    // Author creates a post
    let post = feed_service.create_post(author_id, CreatePostRequest {
        content: Some("Excited to launch our new comments feature! 🚀".to_string()),
        media_items: None,
        visibility: Some("public".to_string()),
    }).await.expect("Failed to create post");

    assert_eq!(post.comment_count, 0);

    // Commenter adds a top-level comment with an attached sticker/media URL
    let comment_req = CreateCommentRequest {
        content: "This looks fantastic! Great job! 🎉".to_string(),
        parent_comment_id: None,
        media_url: Some("https://lotus.local/stickers/celebrate.gif".to_string()),
    };
    let c1 = feed_service.add_comment(commenter_id, post.id, comment_req).await.expect("Failed to add comment");

    assert_eq!(c1.post_id, post.id);
    assert_eq!(c1.author.id, commenter_id);
    assert_eq!(c1.content, "This looks fantastic! Great job! 🎉");
    assert_eq!(c1.media_url, Some("https://lotus.local/stickers/celebrate.gif".to_string()));
    assert_eq!(c1.like_count, 0);
    assert_eq!(c1.reply_count, 0);
    assert_eq!(c1.is_pinned, false);
    assert_eq!(c1.user_has_liked, false);

    // Verify post's comment count updated to 1
    let post_updated = feed_service.get_post(author_id, post.id).await.expect("Failed to get post");
    assert_eq!(post_updated.comment_count, 1);

    // Post author replies to commenter's comment
    let reply_req1 = CreateCommentRequest {
        content: "Thank you so much! Appreciate the feedback ❤️".to_string(),
        parent_comment_id: Some(c1.id),
        media_url: None,
    };
    let r1 = feed_service.add_comment(author_id, post.id, reply_req1).await.expect("Failed to add reply");
    assert_eq!(r1.parent_comment_id, Some(c1.id));
    assert_eq!(r1.author.id, author_id);

    // Verify parent comment reply_count incremented to 1
    let c1_refreshed = feed_service.get_comment(commenter_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_refreshed.reply_count, 1);

    // Verify post total comment count incremented to 2
    let post_updated2 = feed_service.get_post(author_id, post.id).await.expect("Failed to get post");
    assert_eq!(post_updated2.comment_count, 2);

    // Commenter replies to author's reply (nested reply)
    // In Instagram/TikTok style 2-tier flattening, nested replies attach to the root comment thread
    let reply_req2 = CreateCommentRequest {
        content: "Will you support voice notes in comments next?".to_string(),
        parent_comment_id: Some(r1.id),
        media_url: None,
    };
    let r2 = feed_service.add_comment(commenter_id, post.id, reply_req2).await.expect("Failed to add nested reply");
    assert_eq!(r2.parent_comment_id, Some(c1.id)); // Flattened to root parent

    let c1_refreshed2 = feed_service.get_comment(commenter_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_refreshed2.reply_count, 2);

    let post_updated3 = feed_service.get_post(author_id, post.id).await.expect("Failed to get post");
    assert_eq!(post_updated3.comment_count, 3);

    // Test Comment Replies Query
    let replies = feed_service.get_comment_replies(author_id, Some(post.id), c1.id, CommentQuery {
        cursor: None,
        limit: Some(10),
        sort: None,
    }).await.expect("Failed to get comment replies");
    assert_eq!(replies.len(), 2);
    assert_eq!(replies[0].id, r1.id);
    assert_eq!(replies[1].id, r2.id);

    // Test Comment Reactions (Like, Love, etc.)
    // Author reacts "❤️" to commenter's comment
    let rx_res = feed_service.add_comment_reaction(author_id, Some(post.id), c1.id, "❤️").await.expect("Failed to react to comment");
    assert_eq!(rx_res.reaction, "❤️");
    assert_eq!(rx_res.user.id, author_id);

    // Check author's view of comment c1 (should show user_has_liked = true and user_reaction = "❤️")
    let c1_view_author = feed_service.get_comment(author_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_view_author.like_count, 1);
    assert_eq!(c1_view_author.user_has_liked, true);
    assert_eq!(c1_view_author.user_reaction, Some("❤️".to_string()));

    // Check commenter's view of comment c1 (like_count = 1, but user_has_liked = false)
    let c1_view_commenter = feed_service.get_comment(commenter_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_view_commenter.like_count, 1);
    assert_eq!(c1_view_commenter.user_has_liked, false);
    assert_eq!(c1_view_commenter.user_reaction, None);

    // List reactions on comment
    let reactions_list = feed_service.get_comment_reactions(commenter_id, Some(post.id), c1.id).await.expect("Failed to list reactions");
    assert_eq!(reactions_list.len(), 1);
    assert_eq!(reactions_list[0].reaction, "❤️");

    // Author changes reaction to "🔥"
    let _ = feed_service.add_comment_reaction(author_id, Some(post.id), c1.id, "🔥").await.expect("Failed to update reaction");
    let c1_view_author2 = feed_service.get_comment(author_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_view_author2.like_count, 1);
    assert_eq!(c1_view_author2.user_reaction, Some("🔥".to_string()));

    // Author removes reaction
    feed_service.remove_comment_reaction(author_id, Some(post.id), c1.id).await.expect("Failed to remove reaction");
    let c1_view_author3 = feed_service.get_comment(author_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_view_author3.like_count, 0);
    assert_eq!(c1_view_author3.user_has_liked, false);

    // Test Comment Editing
    let edit_req = UpdateCommentRequest {
        content: "This looks fantastic! Edited to add more emojis! 🎉🔥👏".to_string(),
        media_url: Some("https://lotus.local/stickers/updated.gif".to_string()),
    };
    let c1_edited = feed_service.update_comment(commenter_id, Some(post.id), c1.id, edit_req).await.expect("Failed to update comment");
    assert_eq!(c1_edited.content, "This looks fantastic! Edited to add more emojis! 🎉🔥👏");
    assert_eq!(c1_edited.media_url, Some("https://lotus.local/stickers/updated.gif".to_string()));

    // Ensure non-author cannot edit comment
    let unauthorized_edit = feed_service.update_comment(author_id, Some(post.id), c1.id, UpdateCommentRequest {
        content: "Hacked".to_string(),
        media_url: None,
    }).await;
    assert!(unauthorized_edit.is_err());

    // Test Pinning (Instagram / TikTok feature)
    // Post author pins commenter's comment
    feed_service.pin_comment(author_id, Some(post.id), c1.id).await.expect("Failed to pin comment");
    let c1_pinned = feed_service.get_comment(commenter_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_pinned.is_pinned, true);
    assert!(c1_pinned.pinned_at.is_some());

    // Non-author cannot pin comment
    let unauthorized_pin = feed_service.pin_comment(commenter_id, Some(post.id), c1.id).await;
    assert!(unauthorized_pin.is_err());

    // Top-level comments list query with sorting
    let comments_list = feed_service.get_post_comments(author_id, post.id, CommentQuery {
        cursor: None,
        limit: Some(10),
        sort: Some("popular".to_string()),
    }).await.expect("Failed to list top-level comments");
    assert_eq!(comments_list.len(), 1);
    assert_eq!(comments_list[0].id, c1.id);
    assert_eq!(comments_list[0].is_pinned, true);

    // Post author unpins comment
    feed_service.unpin_comment(author_id, Some(post.id), c1.id).await.expect("Failed to unpin comment");
    let c1_unpinned = feed_service.get_comment(commenter_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_unpinned.is_pinned, false);

    // Test Deletion & Cascade Counts
    // Delete one reply
    feed_service.delete_comment(commenter_id, Some(post.id), r2.id).await.expect("Failed to delete reply");
    let c1_after_r2_del = feed_service.get_comment(commenter_id, c1.id).await.expect("Failed to get comment");
    assert_eq!(c1_after_r2_del.reply_count, 1);
    let post_after_r2_del = feed_service.get_post(author_id, post.id).await.expect("Failed to get post");
    assert_eq!(post_after_r2_del.comment_count, 2);

    // Author (moderator) deletes the parent comment c1 (which still has 1 reply r1)
    feed_service.delete_comment(author_id, Some(post.id), c1.id).await.expect("Post author should be able to delete comment on own post");
    let post_after_c1_del = feed_service.get_post(author_id, post.id).await.expect("Failed to get post");
    assert_eq!(post_after_c1_del.comment_count, 0); // Both c1 and r1 cascade deleted!

    // Cleanup
    let _ = feed_service.delete_post(author_id, post.id).await;
    cleanup_user(&pool, author_id).await;
    cleanup_user(&pool, commenter_id).await;
}

#[tokio::test]
async fn test_comment_pagination_and_sorting_modes() {
    let (pool, feed_service) = setup_test_context().await;

    let author_id = create_test_user(&pool).await;
    let user1 = create_test_user(&pool).await;
    let user2 = create_test_user(&pool).await;

    let post = feed_service.create_post(author_id, CreatePostRequest {
        content: Some("Testing sorting and pagination! 📊".to_string()),
        media_items: None,
        visibility: Some("public".to_string()),
    }).await.expect("Failed to create post");

    // Create 3 comments
    let c1 = feed_service.add_comment(user1, post.id, CreateCommentRequest {
        content: "Comment 1 (First)".to_string(),
        parent_comment_id: None,
        media_url: None,
    }).await.unwrap();

    let c2 = feed_service.add_comment(user2, post.id, CreateCommentRequest {
        content: "Comment 2 (Middle)".to_string(),
        parent_comment_id: None,
        media_url: None,
    }).await.unwrap();

    let c3 = feed_service.add_comment(user1, post.id, CreateCommentRequest {
        content: "Comment 3 (Latest)".to_string(),
        parent_comment_id: None,
        media_url: None,
    }).await.unwrap();

    // Like comment 2 so it has highest likes
    feed_service.add_comment_reaction(user1, Some(post.id), c2.id, "like").await.unwrap();
    feed_service.add_comment_reaction(author_id, Some(post.id), c2.id, "love").await.unwrap();

    // Test "popular" sorting -> c2 (2 likes) should be first
    let popular_comments = feed_service.get_post_comments(author_id, post.id, CommentQuery {
        cursor: None,
        limit: Some(10),
        sort: Some("popular".to_string()),
    }).await.unwrap();
    assert_eq!(popular_comments[0].id, c2.id);
    assert_eq!(popular_comments[0].like_count, 2);

    // Test "newest" sorting -> c3 should be first
    let newest_comments = feed_service.get_post_comments(author_id, post.id, CommentQuery {
        cursor: None,
        limit: Some(10),
        sort: Some("newest".to_string()),
    }).await.unwrap();
    assert_eq!(newest_comments[0].id, c3.id);

    // Test "oldest" sorting -> c1 should be first
    let oldest_comments = feed_service.get_post_comments(author_id, post.id, CommentQuery {
        cursor: None,
        limit: Some(10),
        sort: Some("oldest".to_string()),
    }).await.unwrap();
    assert_eq!(oldest_comments[0].id, c1.id);

    // Pin comment 1: pinned comments always appear top regardless of sort mode
    feed_service.pin_comment(author_id, Some(post.id), c1.id).await.unwrap();
    let popular_with_pinned = feed_service.get_post_comments(author_id, post.id, CommentQuery {
        cursor: None,
        limit: Some(10),
        sort: Some("popular".to_string()),
    }).await.unwrap();
    assert_eq!(popular_with_pinned[0].id, c1.id);
    assert_eq!(popular_with_pinned[0].is_pinned, true);
    assert_eq!(popular_with_pinned[1].id, c2.id); // c2 comes second because of likes

    // Test cursor pagination (limit: 2, then next page)
    let page1 = feed_service.get_post_comments(author_id, post.id, CommentQuery {
        cursor: None,
        limit: Some(2),
        sort: Some("newest".to_string()),
    }).await.unwrap();
    assert_eq!(page1.len(), 2);

    // Clean up
    let _ = feed_service.delete_post(author_id, post.id).await;
    cleanup_user(&pool, author_id).await;
    cleanup_user(&pool, user1).await;
    cleanup_user(&pool, user2).await;
}

