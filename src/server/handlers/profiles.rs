//! Profile API handlers.

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;
use std::sync::Arc;

use crate::printer::DeviceProfile;

use super::super::state::AppState;

/// GET /api/profiles - List built-in profiles.
pub async fn list() -> Json<Vec<DeviceProfile>> {
    Json(DeviceProfile::built_in())
}

/// GET /api/profiles/active - Get the current active profile.
pub async fn active(State(state): State<Arc<AppState>>) -> Json<DeviceProfile> {
    let profile = state.active_profile.read().await;
    Json(profile.clone())
}

/// Request body for setting the active profile.
///
/// Accepts either a full DeviceProfile JSON (with `type` field) or
/// a simple `{"name": "..."}` to select by name/parse string.
/// Profile is tried first so that objects with `type` aren't
/// incorrectly matched as ByName.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum SetProfileRequest {
    /// Set directly as a DeviceProfile value.
    Profile(DeviceProfile),
    /// Set by name (e.g., "tsp650ii", "canvas:1200x1800", or display name).
    ByName { name: String },
}

/// PUT /api/profiles/active - Set the active profile.
pub async fn set_active(
    State(state): State<Arc<AppState>>,
    Json(req): Json<SetProfileRequest>,
) -> Result<Json<DeviceProfile>, (StatusCode, String)> {
    let new_profile = match req {
        SetProfileRequest::ByName { name } => {
            DeviceProfile::parse(&name).map_err(|e| (StatusCode::BAD_REQUEST, e))?
        }
        SetProfileRequest::Profile(p) => p,
    };

    let mut profile = state.active_profile.write().await;
    *profile = new_profile.clone();
    Ok(Json(new_profile))
}
