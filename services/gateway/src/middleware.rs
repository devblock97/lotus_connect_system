use axum::{
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};
use errors::AppError;

/// Middleware to inject secure HTTP headers (Helmet-like security)
pub async fn security_headers(req: Request, next: Next) -> Response {
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    
    // Security headers
    if let Ok(v) = "DENY".parse() {
        headers.insert("X-Frame-Options", v);
    }
    if let Ok(v) = "nosniff".parse() {
        headers.insert("X-Content-Type-Options", v);
    }
    if let Ok(v) = "1; mode=block".parse() {
        headers.insert("X-XSS-Protection", v);
    }
    if let Ok(v) = "default-src 'self'".parse() {
        headers.insert("Content-Security-Policy", v);
    }
    if let Ok(v) = "max-age=63072000; includeSubDomains; preload".parse() {
        headers.insert("Strict-Transport-Security", v);
    }
    
    response
}

/// Middleware to enforce JWT Authentication on endpoints
pub async fn require_auth(
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let auth_header = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AppError::Authentication("Missing authorization header".to_string()))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(AppError::Authentication("Authorization header must be a Bearer token".to_string()));
    }

    let token = &auth_header[7..];

    // Extract AppConfig to verify the token
    let config = req
        .extensions()
        .get::<config_crate::AppConfig>()
        .ok_or_else(|| AppError::Internal("AppConfig extension not found".to_string()))?;

    let claims = auth::verify_token(token, &config.jwt_secret)?;
    
    // Inject validated claims into request extensions for subsequent handlers
    req.extensions_mut().insert(claims);

    Ok(next.run(req).await)
}
