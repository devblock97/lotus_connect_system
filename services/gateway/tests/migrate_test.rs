use std::sync::Arc;
use config_crate::AppConfig;
use dto::CreatePostRequest;
use service_feed::{FeedRepositoryImpl, FeedServiceImpl, FeedService};
use uuid::Uuid;

#[tokio::test]
async fn test_run_db_migrations() {
    let config = AppConfig::load().expect("Failed to load config");
    let pool = database::init_db(&config).await;
    match pool {
        Ok(_) => println!("Migrations successfully applied!"),
        Err(e) => panic!("Migration failed: {:?}", e),
    }
}

#[tokio::test]
async fn test_create_post_with_user_payload() {
    let config = AppConfig::load().expect("Failed to load config");
    let pool = database::init_db(&config).await.expect("Failed to connect to db");

    let feed_repo = Arc::new(FeedRepositoryImpl::new(pool.clone()));
    let feed_service = Arc::new(FeedServiceImpl::new(feed_repo));

    let author_id: Uuid = "01a03314-af86-72f3-b270-d5642b05fbdc".parse().unwrap();

    let json_str = r#"{
        "content": "Exploring the serene beauty of the countryside! 🌿✨ #travel #nature",
        "mediaItems": [],
        "visibility": "public"
    }"#;

    let req: CreatePostRequest = serde_json::from_str(json_str).unwrap();

    let res = feed_service.create_post(author_id, req).await;
    println!("Create post result: {:?}", res);
    assert!(res.is_ok(), "Failed to create post: {:?}", res.err());

    let post = res.unwrap();
    assert_eq!(post.author.id, author_id);
    assert_eq!(post.content, "Exploring the serene beauty of the countryside! 🌿✨ #travel #nature");
    assert_eq!(post.visibility, "public");
    assert_eq!(post.like_count, 0);
    assert_eq!(post.comment_count, 0);
    assert_eq!(post.media_items.len(), 0);

    // Clean up test post
    let _ = feed_service.delete_post(author_id, post.id).await;
}


