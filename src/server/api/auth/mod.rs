//! Authentication endpoints

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};

use super::AppError;
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
    pub is_admin: bool,
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
pub async fn auth_setup(
    State(state): State<AppState>,
    Json(request): Json<SetupRequest>,
) -> Result<Json<SetupResponse>, AppError> {
    // Check if already setup
    if state.repos().auth_config.has_password().await? {
        return Err(AppError::Conflict("Password already configured".into()));
    }

    // Validate password strength
    if request.password.len() < 8 {
        return Err(AppError::bad_request(
            "Password must be at least 8 characters",
        ));
    }

    // Hash and store
    let hash =
        auth::hash_password(&request.password).map_err(|e| AppError::Internal(e.to_string()))?;

    state.repos().auth_config.set_password(hash).await?;

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
pub async fn auth_register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthTokenResponse>), AppError> {
    // Validate
    if request.email.trim().is_empty() || !request.email.contains('@') {
        return Err(AppError::bad_request("Invalid email"));
    }
    if request.password.len() < 8 {
        return Err(AppError::bad_request(
            "Password must be at least 8 characters",
        ));
    }

    let user_repo = &state.repos().users;

    // Check if email already exists
    if user_repo.get_user_by_email(&request.email).await?.is_some() {
        return Err(AppError::Conflict("Email already registered".into()));
    }

    let hash =
        auth::hash_password(&request.password).map_err(|e| AppError::Internal(e.to_string()))?;

    let user = user_repo.create_user(&request.email, &hash).await?;

    // Seed built-in execution tools (system-wide)
    let _ = state.repos().tools.seed_builtin_tools().await;

    let token = auth::create_token(state.jwt_secret(), 24, user.id, &user.email, false)
        .map_err(|e| AppError::Internal(e.to_string()))?;

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
pub async fn auth_login(
    State(state): State<AppState>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user_repo = &state.repos().users;

    let user = user_repo
        .get_user_by_email(&request.email)
        .await?
        .ok_or(AppError::Unauthorized("Invalid credentials".into()))?;

    let password_hash = user
        .password_hash
        .as_ref()
        .ok_or(AppError::Unauthorized("Invalid credentials".into()))?;
    if !auth::verify_password(&request.password, password_hash) {
        return Err(AppError::Unauthorized("Invalid credentials".into()));
    }

    let token = auth::create_token(state.jwt_secret(), 24, user.id, &user.email, user.is_admin)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        expires_in: 86400,
    }))
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
pub async fn auth_me(
    State(state): State<AppState>,
    auth: auth::AuthUser,
) -> Result<Json<MeResponse>, AppError> {
    let user_repo = &state.repos().users;

    let user = user_repo
        .get_user_by_id(auth.user_id)
        .await?
        .ok_or(AppError::Unauthorized("Invalid credentials".into()))?;

    Ok(Json(MeResponse {
        id: user.id.to_string(),
        email: user.email,
        github_login: user.github_login,
        is_admin: user.is_admin,
        authenticated: true,
        token_expires: auth.claims.exp,
    }))
}
#[cfg(test)]
mod tests;
