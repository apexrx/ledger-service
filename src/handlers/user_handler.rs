use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use uuid::Uuid;

use crate::AppState;
use crate::entities::role::Role;
use crate::services::user_service;

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

pub async fn update_role(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateRoleRequest>,
) -> impl IntoResponse {
    match user_service::update_role(&state.db, user_id, payload.role).await {
        Ok(user) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "User role updated",
                "user": {
                    "id": user.id,
                    "email": user.email,
                    "role": user.role,
                    "status": user.status,
                }
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": e.to_string(),
            })),
        ),
    }
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> impl IntoResponse {
    match user_service::deactivate_user(&state.db, user_id).await {
        Ok(user) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "User deactivated",
                "user": {
                    "id": user.id,
                    "email": user.email,
                    "role": user.role,
                    "status": user.status,
                }
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "error",
                "message": e.to_string(),
            })),
        ),
    }
}
