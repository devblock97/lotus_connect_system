use axum::{
    extract::{State, Extension},
    Json,
};
use validator::Validate;
use uuid::Uuid;

use errors::{AppError, Result};
use auth::Claims;
use dto::{
    RegisterRequest, LoginRequest, RefreshTokenRequest, 
    AuthResponse, UserResponse, TokenResponse, GenericResponse
};
use crate::AppState;

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<UserResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let user = state.auth_service.register(payload).await?;
    Ok(Json(user))
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let res = state.auth_service.login(payload).await?;
    Ok(Json(res))
}

pub async fn refresh_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let res = state.auth_service.refresh_token(&payload.refresh_token).await?;
    Ok(Json(res))
}

pub async fn logout_handler(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<GenericResponse>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    state.auth_service.logout(&payload.refresh_token).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Logged out successfully".to_string(),
    }))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AddFriendRequest {
    #[validate(length(min = 3, message = "Username must be at least 3 characters"))]
    pub username: String,
}

pub async fn add_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AddFriendRequest>,
) -> Result<Json<models::Friendship>> {
    payload.validate().map_err(|err| AppError::Validation(err.to_string()))?;
    let friendship = state.user_service.send_friend_request(claims.sub, &payload.username).await?;
    Ok(Json(friendship))
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
pub struct AcceptFriendRequest {
    pub friend_id: Uuid,
}

pub async fn accept_friend_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<AcceptFriendRequest>,
) -> Result<Json<GenericResponse>> {
    state.user_service.accept_friend_request(claims.sub, payload.friend_id).await?;
    Ok(Json(GenericResponse {
        success: true,
        message: "Friend request accepted successfully".to_string(),
    }))
}

pub async fn list_friends_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<models::User>>> {
    let friends = state.user_service.get_friends_list(claims.sub).await?;
    Ok(Json(friends))
}
