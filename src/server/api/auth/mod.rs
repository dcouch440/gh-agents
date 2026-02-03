//! Authentication endpoints

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::server::auth;
use crate::server::state::AppState;

// ============================================================================
// Request/Response Types
// ============================================================================

/// Request body for auth setup
#[derive(Deserialize, utoipa::ToSchema)]
pub struct SetupRequest {
    pub password: String,
}

/// Response for auth setup
#[derive(Serialize, utoipa::ToSchema)]
pub struct SetupResponse {
    pub message: String,
}

/// Request body for registration
#[derive(Deserialize, utoipa::ToSchema)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

/// Response for registration
#[derive(Serialize, utoipa::ToSchema)]
pub struct AuthTokenResponse {
    pub token: String,
    pub expires_in: u64,
    pub user: UserResponse,
}

/// User info in API responses
#[derive(Serialize, utoipa::ToSchema)]
pub struct UserResponse {
    pub id: String,
    pub email: String,
    pub github_login: Option<String>,
}

/// Request body for login
#[derive(Deserialize, utoipa::ToSchema)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Response for successful login
#[derive(Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub token: String,
    pub expires_in: u64,
}

/// Response for /api/auth/me
#[derive(Serialize, utoipa::ToSchema)]
pub struct MeResponse {
    pub id: String,
    pub email: String,
    pub github_login: Option<String>,
    pub authenticated: bool,
    pub token_expires: usize,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /api/auth/setup - First-run password configuration
///
/// This endpoint is only available when no password has been configured yet.
/// Once a password is set, this endpoint returns 409 Conflict.
#[utoipa::path(
    post,
    path = "/api/auth/setup",
    tag = "Auth",
    request_body = SetupRequest,
    responses(
        (status = 200, description = "Password configured", body = SetupResponse),
        (status = 400, description = "Password too short"),
        (status = 409, description = "Password already configured")
    )
)]
pub async fn auth_setup(State(state): State<AppState>, Json(request): Json<SetupRequest>) -> Result<Json<SetupResponse>, (StatusCode, String)> {
    // Check if already setup
    if state.repo.has_password().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))? {
        return Err((StatusCode::CONFLICT, "Password already configured".to_string()));
    }

    // Validate password strength
    if request.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 8 characters".to_string()));
    }

    // Hash and store
    let hash = auth::hash_password(&request.password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    state.repo.set_password(hash).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SetupResponse {
        message: "Password configured successfully".to_string(),
    }))
}

/// POST /api/auth/register - Register a new user
#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Auth",
    request_body = RegisterRequest,
    responses(
        (status = 201, description = "User registered", body = AuthTokenResponse),
        (status = 400, description = "Invalid email or password"),
        (status = 409, description = "Email already registered")
    )
)]
pub async fn auth_register(State(state): State<AppState>, Json(request): Json<RegisterRequest>) -> Result<(StatusCode, Json<AuthTokenResponse>), (StatusCode, String)> {
    // Validate
    if request.email.trim().is_empty() || !request.email.contains('@') {
        return Err((StatusCode::BAD_REQUEST, "Invalid email".into()));
    }
    if request.password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "Password must be at least 8 characters".into()));
    }

    let user_repo = state.user_repo.as_ref().ok_or((StatusCode::INTERNAL_SERVER_ERROR, "User service unavailable".into()))?;

    // Check if email already exists
    if user_repo
        .get_user_by_email(&request.email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .is_some()
    {
        return Err((StatusCode::CONFLICT, "Email already registered".into()));
    }

    let hash = auth::hash_password(&request.password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let user = user_repo.create_user(&request.email, &hash).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Seed built-in execution tools for the new user
    let _ = state.repo.seed_builtin_tools(user.id).await;

    let token = auth::create_token(&state.jwt_secret, 24, user.id, &user.email).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((
        StatusCode::CREATED,
        Json(AuthTokenResponse {
            token,
            expires_in: 86400,
            user: UserResponse {
                id: user.id.to_string(),
                email: user.email,
                github_login: user.github_login,
            },
        }),
    ))
}

/// POST /api/auth/login - Authenticate and get JWT token
///
/// Verifies the provided password and returns a JWT token valid for 24 hours.
#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Auth",
    request_body = LoginRequest,
    responses(
        (status = 200, description = "Login successful", body = LoginResponse),
        (status = 401, description = "Invalid credentials")
    )
)]
pub async fn auth_login(State(state): State<AppState>, Json(request): Json<LoginRequest>) -> Result<Json<LoginResponse>, StatusCode> {
    let user_repo = state.user_repo.as_ref().ok_or_else(|| {
        tracing::error!("user_repo is None");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user = user_repo
        .get_user_by_email(&request.email)
        .await
        .map_err(|e| {
            tracing::error!("Database error getting user: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let password_hash = user.password_hash.as_ref().ok_or(StatusCode::UNAUTHORIZED)?;
    if !auth::verify_password(&request.password, password_hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = auth::create_token(&state.jwt_secret, 24, user.id, &user.email).map_err(|e| {
        tracing::error!("JWT token creation error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(LoginResponse { token, expires_in: 86400 }))
}

/// GET /api/auth/me - Get current user info from token
///
/// Requires a valid JWT token in Authorization header.
#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "Auth",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user info", body = MeResponse)
    )
)]
pub async fn auth_me(State(state): State<AppState>, auth: auth::AuthUser) -> Result<Json<MeResponse>, StatusCode> {
    let user_repo = state.user_repo.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let user = user_repo
        .get_user_by_id(auth.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(Json(MeResponse {
        id: user.id.to_string(),
        email: user.email,
        github_login: user.github_login,
        authenticated: true,
        token_expires: auth.claims.exp,
    }))
}
mod tests;
