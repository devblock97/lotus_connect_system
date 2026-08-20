use std::sync::Arc;
use axum::{
    routing::{get, post, put},
    middleware as axum_middleware,
    Router,
    Extension,
};
use tower_http::{
    cors::{Any, CorsLayer},
    request_id::{MakeRequestUuid, SetRequestIdLayer},
    propagate_header::PropagateHeaderLayer,
};
use std::net::SocketAddr;
use tokio::net::TcpListener;

use config_crate::AppConfig;
use database::PgPool;
use errors::Result;

pub mod handlers;
pub mod middleware;
pub mod ws;

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub auth_service: Arc<dyn service_auth::AuthService>,
    pub user_service: Arc<dyn service_users::UserService>,
    pub presence_service: Arc<dyn service_presence::PresenceService>,
    pub chat_service: Arc<dyn service_chat::ChatService>,
    pub call_service: Arc<dyn service_call::CallService>,
    pub storage_provider: Arc<dyn service_upload::StorageProvider>,
    pub notification_service: Arc<dyn service_notification::NotificationService>,
    pub ws_manager: ws::manager::WsManager,
}

pub async fn run_server(config: AppConfig, pool: PgPool) -> Result<()> {
    // 1. Initialize Redis client and connection manager
    let redis_client = redis::Client::open(config.redis_url.clone())
        .map_err(|err| errors::AppError::Internal(format!("Failed to open Redis: {}", err)))?;
    let redis_conn = redis_client.get_connection_manager().await
        .map_err(|err| errors::AppError::Internal(format!("Failed to connect to Redis: {}", err)))?;

    // 2. Initialize Repositories and Services (Dependency Injection)
    let user_repo = Arc::new(service_users::UserRepositoryImpl::new(pool.clone()));
    let user_service = Arc::new(service_users::UserServiceImpl::new(user_repo));

    let token_repo = Arc::new(service_auth::RefreshTokenRepositoryImpl::new(pool.clone()));
    let auth_service = Arc::new(service_auth::AuthServiceImpl::new(
        config.clone(),
        pool.clone(),
        user_service.clone(),
        token_repo,
    ));

    let presence_service = Arc::new(service_presence::PresenceServiceImpl::new(pool.clone(), redis_conn));
    
    let chat_repo = Arc::new(service_chat::ChatRepositoryImpl::new(pool.clone()));
    let chat_service = Arc::new(service_chat::ChatServiceImpl::new(chat_repo));

    let call_repo = Arc::new(service_call::CallRepositoryImpl::new(pool.clone()));
    let call_service = Arc::new(service_call::CallServiceImpl::new(call_repo));

    let storage_provider = service_upload::create_storage_provider(&config);

    let notification_provider: Arc<dyn service_notification::NotificationProvider> =
        if let Some(key) = service_notification::ServiceAccountKey::from_env() {
            tracing::info!(
                "FCM Service Account initialized for project '{}'. Using FcmV1NotificationProvider.",
                key.project_id
            );
            Arc::new(service_notification::FcmV1NotificationProvider::new(key))
        } else {
            tracing::info!("FCM Service Account not configured. Using MockNotificationProvider.");
            Arc::new(service_notification::MockNotificationProvider)
        };

    let notification_service = Arc::new(service_notification::NotificationServiceImpl::new(
        pool.clone(),
        notification_provider,
    ));


    let ws_manager = ws::manager::WsManager::new();

    let state = AppState {
        config: config.clone(),
        auth_service,
        user_service,
        presence_service,
        chat_service,
        call_service,
        storage_provider,
        notification_service,
        ws_manager,
    };

    // 3. Setup CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 4. Build Auth router
    let auth_routes = Router::new()
        .route("/register", post(handlers::register_handler))
        .route("/login", post(handlers::login_handler))
        .route("/refresh", post(handlers::refresh_handler))
        .route("/logout", post(handlers::logout_handler));

    // 5. Build Users router (protected by auth)
    let user_routes = Router::new()
        .route("/friends", post(handlers::add_friend_handler).get(handlers::list_friends_handler))
        .route("/friends/requests", get(handlers::list_friend_requests_handler))
        .route("/friends/accept", post(handlers::accept_friend_handler))
        .route("/friends/reject", post(handlers::reject_friend_handler))
        .route("/search", get(handlers::search_users_handler))
        .route("/devices", post(handlers::register_device_handler))
        .route("/devices/unregister", post(handlers::unregister_device_handler))
        .route("/device-token", post(handlers::register_device_handler))

        .route("/notifications", get(handlers::list_notifications_handler))
        .route("/notifications/read", post(handlers::mark_notifications_read_handler))
        .layer(axum_middleware::from_fn(self::middleware::require_auth));

    // 6. Build Chats router (protected by auth)
    let chat_routes = Router::new()
        .route("/", get(handlers::list_conversations_handler))
        .route("/private", post(handlers::create_private_chat_handler))
        .route("/group", post(handlers::create_group_chat_handler))
        .route("/:conversation_id/messages", get(handlers::get_messages_handler).post(handlers::send_message_handler))
        .route("/messages/:message_id", put(handlers::edit_message_handler).delete(handlers::delete_message_handler))
        .layer(axum_middleware::from_fn(self::middleware::require_auth));

    // 7. Build Calls router (protected by auth)
    let call_routes = Router::new()
        .route("/history", get(handlers::get_calls_history_handler))
        .layer(axum_middleware::from_fn(self::middleware::require_auth));

    // 8. Build Uploads router (protected by auth)
    let upload_routes = Router::new()
        .route("/", post(handlers::upload_file_handler))
        .layer(axum_middleware::from_fn(self::middleware::require_auth));

    // 9. Combine all routers under versioned api prefix
    let api_router = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes)
        .nest("/chats", chat_routes)
        .nest("/calls", call_routes)
        .nest("/uploads", upload_routes)
        .route("/ws", get(ws::ws_handler));

    // 10. Base App Router
    let app = Router::new()
        .nest("/api/v1", api_router)
        .route("/health", get(health_handler))
        .layer(cors)
        .layer(axum_middleware::from_fn(self::middleware::security_headers))
        // Logging middleware must run after RequestId injection
        .layer(axum_middleware::from_fn(request_logger_middleware))
        .layer(PropagateHeaderLayer::new(axum::http::HeaderName::from_static("x-request-id")))
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(Extension(config.clone()))
        .with_state(state);

    // 11. Bind and run
    let addr = SocketAddr::new(
        config.host.parse().unwrap_or([0, 0, 0, 0].into()),
        config.port,
    );
    
    tracing::info!("Gateway server listening on {}", addr);
    let listener = TcpListener::bind(&addr)
        .await
        .map_err(|err| errors::AppError::Internal(format!("Failed to bind port {}: {}", config.port, err)))?;

    axum::serve(listener, app)
        .await
        .map_err(|err| errors::AppError::Internal(format!("Server execution failed: {}", err)))?;

    Ok(())
}

async fn health_handler() -> &'static str {
    "OK"
}

/// Custom logging middleware that extracts Request ID, User ID (if authenticated), duration, and status code
async fn request_logger_middleware(
    req: axum::extract::Request,
    next: axum_middleware::Next,
) -> axum::response::Response {
    let start = std::time::Instant::now();
    let path = req.uri().path().to_string();
    let method = req.method().to_string();

    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Check if JWT claims were extracted and added to extensions
    let user_id = req
        .extensions()
        .get::<auth::Claims>()
        .map(|c| c.sub.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    let response = next.run(req).await;
    let latency = start.elapsed();
    let status = response.status().as_u16();

    tracing::info!(
        request_id = %request_id,
        user_id = %user_id,
        method = %method,
        path = %path,
        status = status,
        latency_ms = latency.as_millis(),
        "Request completed"
    );

    response
}
