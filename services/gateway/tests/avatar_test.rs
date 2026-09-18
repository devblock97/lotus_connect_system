use dto::{AvatarUploadResponse, UpdateAvatarRequest, UserResponse};
use uuid::Uuid;
use validator::Validate;

#[test]
fn test_user_response_avatar_url_serialization() {
    let user = UserResponse {
        id: Uuid::now_v7(),
        username: "testuser".to_string(),
        full_name: Some("Test User".to_string()),
        email: "test@example.com".to_string(),
        avatar_url: Some("http://localhost:8080/uploads/avatar-123.jpg".to_string()),
        friendship_status: None,
        friendship_sender_id: None,
    };

    let serialized = serde_json::to_string(&user).expect("Failed to serialize UserResponse");
    assert!(serialized.contains(r#""avatarUrl":"http://localhost:8080/uploads/avatar-123.jpg""#));

    let deserialized: UserResponse = serde_json::from_str(&serialized).expect("Failed to deserialize UserResponse");
    assert_eq!(deserialized.avatar_url, Some("http://localhost:8080/uploads/avatar-123.jpg".to_string()));
    assert_eq!(deserialized.username, "testuser");
}

#[test]
fn test_update_avatar_request_validation() {
    let valid_req = UpdateAvatarRequest {
        avatar_url: "http://localhost:8080/uploads/my-avatar.png".to_string(),
    };
    assert!(valid_req.validate().is_ok());

    let invalid_req = UpdateAvatarRequest {
        avatar_url: "".to_string(),
    };
    assert!(invalid_req.validate().is_err());
}

#[test]
fn test_avatar_upload_response_serialization() {
    let user = UserResponse {
        id: Uuid::now_v7(),
        username: "johndoe".to_string(),
        full_name: Some("John Doe".to_string()),
        email: "john@example.com".to_string(),
        avatar_url: Some("http://localhost:8080/uploads/avatar.webp".to_string()),
        friendship_status: None,
        friendship_sender_id: None,
    };

    let upload_res = AvatarUploadResponse {
        avatar_url: "http://localhost:8080/uploads/avatar.webp".to_string(),
        file_url: "http://localhost:8080/uploads/avatar.webp".to_string(),
        user,
    };

    let json_val = serde_json::to_value(&upload_res).expect("Failed to serialize AvatarUploadResponse");
    assert_eq!(json_val["avatarUrl"], "http://localhost:8080/uploads/avatar.webp");
    assert_eq!(json_val["fileUrl"], "http://localhost:8080/uploads/avatar.webp");
    assert_eq!(json_val["user"]["avatarUrl"], "http://localhost:8080/uploads/avatar.webp");
    assert_eq!(json_val["user"]["username"], "johndoe");
}

#[test]
fn test_avatar_image_type_validation() {
    let is_valid_avatar = |file_name: &str, content_type: &str| -> bool {
        let ext = std::path::Path::new(file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "jpg" | "jpeg" | "png" | "webp" | "gif" => true,
            _ => content_type.starts_with("image/"),
        }
    };

    assert!(is_valid_avatar("photo.jpg", "image/jpeg"));
    assert!(is_valid_avatar("photo.JPEG", ""));
    assert!(is_valid_avatar("avatar.png", "image/png"));
    assert!(is_valid_avatar("profile.webp", "image/webp"));
    assert!(is_valid_avatar("pic.gif", "image/gif"));
    assert!(is_valid_avatar("upload_blob", "image/jpeg"));

    assert!(!is_valid_avatar("document.pdf", "application/pdf"));
    assert!(!is_valid_avatar("malicious.exe", "application/x-msdownload"));
    assert!(!is_valid_avatar("archive.zip", "application/zip"));
}
