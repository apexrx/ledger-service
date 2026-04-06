use axum::{
    extract::{Path, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::AppState;
use crate::error::{AppError, AppJson};
use crate::entities::role::Role;
use crate::entities::status::Status;
use crate::entities::users::Model as UserModel;
use crate::services::user_service;

#[derive(Deserialize)]
pub struct UpdateRoleRequest {
    pub role: Role,
}

#[derive(Deserialize)]
pub struct UpdateStatusRequest {
    pub status: Status,
}

#[derive(Deserialize)]
pub struct CreateAdminUserRequest {
    pub email: String,
    pub role: Role,
}

#[derive(Serialize)]
pub struct CreateUserResponse {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub status: Status,
    pub created_at: chrono::NaiveDateTime,
    pub temporary_password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub status: Status,
    pub created_at: chrono::NaiveDateTime,
}

impl From<&UserModel> for UserResponse {
    fn from(user: &UserModel) -> Self {
        Self {
            id: user.id,
            email: user.email.clone(),
            role: user.role.clone(),
            status: user.status.clone(),
            created_at: user.created_at,
        }
    }
}

pub async fn list_users(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let users = user_service::list_users(&state.db).await?;
    let users: Vec<UserResponse> = users.iter().map(UserResponse::from).collect();

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "users": users,
        })),
    ))
}

pub async fn create_user(
    State(state): State<AppState>,
    AppJson(payload): AppJson<CreateAdminUserRequest>,
) -> Result<impl IntoResponse, AppError> {
    let (user, temp_password) = user_service::create_user_by_admin(&state.db, payload.email, payload.role).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        axum::Json(json!({
            "status": "success",
            "message": "User created successfully",
            "user": CreateUserResponse {
                id: user.id,
                email: user.email,
                role: user.role,
                status: user.status,
                created_at: user.created_at,
                temporary_password: temp_password,
            },
        })),
    ))
}

pub async fn update_role(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    AppJson(payload): AppJson<UpdateRoleRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = user_service::update_role(&state.db, user_id, payload.role).await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "message": "User role updated",
            "user": {
                "id": user.id,
                "email": user.email,
                "role": user.role,
                "status": user.status,
            }
        })),
    ))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let user = user_service::deactivate_user(&state.db, user_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "message": "User deactivated",
            "user": {
                "id": user.id,
                "email": user.email,
                "role": user.role,
                "status": user.status,
            }
        })),
    ))
}

pub async fn update_user_status(
    State(state): State<AppState>,
    Path(user_id): Path<Uuid>,
    AppJson(payload): AppJson<UpdateStatusRequest>,
) -> Result<impl IntoResponse, AppError> {
    let user = user_service::update_user_status(&state.db, user_id, payload.status).await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "message": "User status updated",
            "user": {
                "id": user.id,
                "email": user.email,
                "role": user.role,
                "status": user.status,
            }
        })),
    ))
}
