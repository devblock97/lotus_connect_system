use std::sync::Arc;
use std::time::Duration;
use config_crate::AppConfig;
use service_notification::{MockNotificationProvider, NotificationServiceImpl, NotificationService};
use uuid::Uuid;

async fn setup_test_context() -> (database::PgPool, Arc<NotificationServiceImpl>) {
    let config = AppConfig::load().expect("Failed to load config");
    let pool = database::init_db(&config).await.expect("Failed to connect to db");
    let provider = Arc::new(MockNotificationProvider);
    let service = Arc::new(NotificationServiceImpl::new(pool.clone(), provider));
    (pool, service)
}

async fn create_test_user(pool: &database::PgPool) -> Uuid {
    let user_id = Uuid::now_v7();
    let username = format!("notif_user_{}", user_id.simple());
    let email = format!("notif_{}@example.com", user_id.simple());
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
    let _ = sqlx::query("DELETE FROM notifications WHERE user_id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(pool)
        .await;
}

#[tokio::test]
async fn test_notification_cursor_based_pagination() {
    let (pool, service) = setup_test_context().await;
    let user_id = create_test_user(&pool).await;

    // Create 5 notifications with slight delays to ensure monotonic ordering
    for i in 1..=5 {
        let title = format!("Notification #{}", i);
        let body = format!("Body of notification #{}", i);
        service.send_notification(user_id, &title, &body, None)
            .await
            .expect("Failed to send notification");
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Fetch Page 1 (limit: 2, no cursor)
    let page1 = service.get_notifications(user_id, None, Some(2))
        .await
        .expect("Failed to fetch page 1");
    assert_eq!(page1.len(), 2, "Page 1 should have exactly 2 items");
    assert_eq!(page1[0].title, "Notification #5");
    assert_eq!(page1[1].title, "Notification #4");

    // Fetch Page 2 (cursor: last item of page 1, limit: 2)
    let cursor_page1 = Some(page1[1].id);
    let page2 = service.get_notifications(user_id, cursor_page1, Some(2))
        .await
        .expect("Failed to fetch page 2");
    assert_eq!(page2.len(), 2, "Page 2 should have exactly 2 items");
    assert_eq!(page2[0].title, "Notification #3");
    assert_eq!(page2[1].title, "Notification #2");

    // Fetch Page 3 (cursor: last item of page 2, limit: 2)
    let cursor_page2 = Some(page2[1].id);
    let page3 = service.get_notifications(user_id, cursor_page2, Some(2))
        .await
        .expect("Failed to fetch page 3");
    assert_eq!(page3.len(), 1, "Page 3 should have remaining 1 item");
    assert_eq!(page3[0].title, "Notification #1");

    // Fetch Page 4 (cursor: last item of page 3 -> end of list)
    let cursor_page3 = Some(page3[0].id);
    let page4 = service.get_notifications(user_id, cursor_page3, Some(2))
        .await
        .expect("Failed to fetch page 4");
    assert_eq!(page4.len(), 0, "Page 4 should be empty");

    // Test default page limit when limit is None (should clamp to default 20)
    let all = service.get_notifications(user_id, None, None)
        .await
        .expect("Failed to fetch all notifications");
    assert_eq!(all.len(), 5, "Should return all 5 notifications");

    // Clean up
    cleanup_user(&pool, user_id).await;
}
