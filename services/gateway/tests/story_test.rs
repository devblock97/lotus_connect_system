use std::sync::Arc;
use config_crate::AppConfig;
use dto::CreateStoryRequest;
use service_story::{StoryRepositoryImpl, StoryServiceImpl, StoryService};
use uuid::Uuid;

async fn setup_test_context() -> (database::PgPool, Arc<StoryServiceImpl>) {
    let config = AppConfig::load().expect("Failed to load config");
    let pool = database::init_db(&config).await.expect("Failed to connect to db");
    let story_repo = Arc::new(StoryRepositoryImpl::new(pool.clone()));
    let story_service = Arc::new(StoryServiceImpl::new(story_repo));
    (pool, story_service)
}

async fn create_test_user(pool: &database::PgPool) -> Uuid {
    let user_id = Uuid::now_v7();
    let username = format!("story_user_{}", user_id.simple());
    let email = format!("story_{}@example.com", user_id.simple());
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
async fn test_create_text_only_story_with_null_media_url_in_database() {
    let (pool, service) = setup_test_context().await;
    let author_id = create_test_user(&pool).await;

    // Create text-only story with media_url explicitly None
    let text_req = CreateStoryRequest {
        media_type: Some("text".to_string()),
        media_url: None,
        thumbnail_url: None,
        caption: Some("Loving the autumn vibe! 🍂✨".to_string()),
        duration: Some(5.0),
        visibility: Some("public".to_string()),
        background_color: Some("#8B5CF6".to_string()),
        metadata: Some(serde_json::json!({
            "fontSize": 24,
            "fontFamily": "Inter"
        })),
    };

    let story = service.create_story(author_id, text_req).await
        .expect("Failed to create text-only story with null media_url");

    assert_eq!(story.media_type, "text");
    assert_eq!(story.media_url, None);
    assert_eq!(story.caption, Some("Loving the autumn vibe! 🍂✨".to_string()));
    assert_eq!(story.background_color, Some("#8B5CF6".to_string()));

    // Query raw database row to verify media_url is stored as NULL
    let row: (Option<String>, String, Option<String>) = sqlx::query_as(
        "SELECT media_url, media_type, caption FROM stories WHERE id = $1"
    )
    .bind(story.id)
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch story from DB");

    assert_eq!(row.0, None);
    assert_eq!(row.1, "text");
    assert_eq!(row.2, Some("Loving the autumn vibe! 🍂✨".to_string()));

    // Retrieve story through service
    let fetched = service.get_story(author_id, story.id).await
        .expect("Failed to fetch story");
    assert_eq!(fetched.id, story.id);
    assert_eq!(fetched.media_url, None);

    // Test JSON serialization: mediaUrl should serialize as null
    let json_val = serde_json::to_value(&fetched).expect("Failed to serialize story response");
    assert!(json_val["mediaUrl"].is_null());

    // Create image story with media_url provided
    let image_req = CreateStoryRequest {
        media_type: Some("image".to_string()),
        media_url: Some("https://lotus.local/uploads/photo.jpg".to_string()),
        thumbnail_url: None,
        caption: Some("Beach day! 🏖️".to_string()),
        duration: Some(5.0),
        visibility: Some("public".to_string()),
        background_color: None,
        metadata: None,
    };
    let image_story = service.create_story(author_id, image_req).await
        .expect("Failed to create image story");
    assert_eq!(image_story.media_url, Some("https://lotus.local/uploads/photo.jpg".to_string()));

    // Cleanup
    cleanup_user(&pool, author_id).await;
}
