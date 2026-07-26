use std::sync::Arc;
use axum::{
    routing::{get, post},
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

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub auth_service: Arc<dyn service_auth::AuthService>,
    pub user_service: Arc<dyn service_users::UserService>,
}

pub async fn run_server(config: AppConfig, pool: PgPool) -> Result<()> {
    // 1. Initialize Repositories and Services (Dependency Injection)
    let user_repo = Arc::new(service_users::UserRepositoryImpl::new(pool.clone()));
    let user_service = Arc::new(service_users::UserServiceImpl::new(user_repo));

    let token_repo = Arc::new(service_auth::RefreshTokenRepositoryImpl::new(pool.clone()));
    let auth_service = Arc::new(service_auth::AuthServiceImpl::new(
        config.clone(),
        pool.clone(),
        user_service.clone(),
        token_repo,
    ));

    let state = AppState {
        config: config.clone(),
        auth_service,
        user_service,
    };

    // 2. Setup CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 3. Build Auth router
    let auth_routes = Router::new()
        .route("/register", post(handlers::register_handler))
        .route("/login", post(handlers::login_handler))
        .route("/refresh", post(handlers::refresh_handler))
        .route("/logout", post(handlers::logout_handler));

    // 4. Build Users router (protected by auth)
    let user_routes = Router::new()
        .route("/friends", post(handlers::add_friend_handler).get(handlers::list_friends_handler))
        .route("/friends/accept", post(handlers::accept_friend_handler))
        .layer(axum_middleware::from_fn(self::middleware::require_auth));

    // 5. Combine all routers under versioned api prefix
    let api_router = Router::new()
        .nest("/auth", auth_routes)
        .nest("/users", user_routes);

    // 6. Base App Router
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

    // 7. Bind and run
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
