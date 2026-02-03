//! Server configuration endpoints

use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::server::state::AppState;
use crate::types::{AgentPoolConfig, AutonomyLevel, GitStrategy, SandboxMode, VerbosityLevel};

/// Configuration response
#[derive(Serialize, utoipa::ToSchema)]
pub struct ConfigResponse {
    pub verbosity: String,
    pub pool: AgentPoolConfig,
    pub autonomy: String,
    pub git_strategy: String,
    pub sandbox_mode: String,
}

/// Request body for updating pool sizes
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdatePoolRequest {
    pub max_agents: Option<u8>,
}

/// Request body for updating configuration
#[derive(Deserialize, utoipa::ToSchema)]
pub struct UpdateConfigRequest {
    pub verbosity: Option<String>,
    pub pool: Option<UpdatePoolRequest>,
    pub autonomy: Option<String>,
    pub git_strategy: Option<String>,
    pub sandbox_mode: Option<String>,
}

/// Get current configuration
#[utoipa::path(
    get,
    path = "/api/config",
    tag = "Config",
    responses(
        (status = 200, description = "Current configuration", body = ConfigResponse)
    )
)]
pub async fn get_config(State(state): State<AppState>) -> Json<ConfigResponse> {
    let config = state.config.read().await;

    Json(ConfigResponse {
        verbosity: format!("{:?}", config.verbosity).to_lowercase(),
        pool: config.pool.clone(),
        autonomy: format!("{:?}", config.autonomy).to_lowercase(),
        git_strategy: format!("{:?}", config.git_strategy).to_lowercase(),
        sandbox_mode: format!("{:?}", config.sandbox_mode).to_lowercase(),
    })
}

/// Update configuration (partial update)
#[utoipa::path(
    patch,
    path = "/api/config",
    tag = "Config",
    request_body = UpdateConfigRequest,
    responses(
        (status = 200, description = "Updated configuration", body = ConfigResponse),
        (status = 400, description = "Invalid value")
    )
)]
pub async fn update_config(State(state): State<AppState>, Json(request): Json<UpdateConfigRequest>) -> Result<Json<ConfigResponse>, StatusCode> {
    let mut config = state.config.write().await;

    // Verbosity
    if let Some(ref v) = request.verbosity {
        match v.to_lowercase().as_str() {
            "quiet" => config.verbosity = VerbosityLevel::Quiet,
            "normal" => config.verbosity = VerbosityLevel::Normal,
            "verbose" => config.verbosity = VerbosityLevel::Verbose,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Pool
    if let Some(ref pool) = request.pool {
        if let Some(v) = pool.max_agents {
            config.pool.max_agents = v;
        }
    }

    // Autonomy
    if let Some(ref a) = request.autonomy {
        match a.to_lowercase().as_str() {
            "full_auto" => config.autonomy = AutonomyLevel::FullAuto,
            "approval_gates" => config.autonomy = AutonomyLevel::ApprovalGates,
            "supervised" => config.autonomy = AutonomyLevel::Supervised,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Git strategy
    if let Some(ref g) = request.git_strategy {
        match g.to_lowercase().as_str() {
            "branch_per_slice" => config.git_strategy = GitStrategy::BranchPerSlice,
            "branch_per_ticket" => config.git_strategy = GitStrategy::BranchPerTicket,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    // Sandbox mode
    if let Some(ref s) = request.sandbox_mode {
        match s.to_lowercase().as_str() {
            "docker" => config.sandbox_mode = SandboxMode::Docker,
            "local_restricted" => config.sandbox_mode = SandboxMode::LocalRestricted,
            "none" => config.sandbox_mode = SandboxMode::None,
            _ => return Err(StatusCode::BAD_REQUEST),
        }
    }

    let resp = ConfigResponse {
        verbosity: format!("{:?}", config.verbosity).to_lowercase(),
        pool: config.pool.clone(),
        autonomy: format!("{:?}", config.autonomy).to_lowercase(),
        git_strategy: format!("{:?}", config.git_strategy).to_lowercase(),
        sandbox_mode: format!("{:?}", config.sandbox_mode).to_lowercase(),
    };

    Ok(Json(resp))
}
#[cfg(test)]
mod tests;
