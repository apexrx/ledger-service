use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use axum::{extract::State, response::IntoResponse};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::Validate;

use crate::AppState;
use crate::error::{AppError, AppJson};
use crate::middleware::auth::Claims;
use crate::entities::role::Role;
use crate::services::user_service;

#[derive(Deserialize, Serialize, Validate)]
pub struct RegisterRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password cannot be empty"))]
    pub password: String,
}

#[derive(Deserialize, Serialize, Validate)]
pub struct LoginRequest {
    #[validate(email(message = "Invalid email format"))]
    pub email: String,
    #[validate(length(min = 1, message = "Password cannot be empty"))]
    pub password: String,
}

pub async fn register_handler(
    State(state): State<AppState>,
    AppJson(payload): AppJson<RegisterRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate()?;

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|e| AppError::internal(format!("Failed to hash password: {e}")))?
        .to_string();

    let user = user_service::create_user(&state.db, payload.email, password_hash, Role::Viewer).await?;

    Ok((
        axum::http::StatusCode::CREATED,
        axum::Json(json!({
            "status": "success",
            "message": "User registered successfully",
            "user": {
                "id": user.id,
                "email": user.email,
                "role": user.role,
            }
        })),
    ))
}

pub async fn login_handler(
    State(state): State<AppState>,
    AppJson(payload): AppJson<LoginRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate()?;

    let user = user_service::find_by_email(&state.db, &payload.email)
        .await?
        .ok_or_else(|| AppError::unauthorized("Invalid email or password"))?;

    if !user.status.can_login() {
        return Err(AppError::unauthorized("Account is not active"));
    }

    let parsed_hash = PasswordHash::new(&user.password_hash)
        .map_err(|e| AppError::internal(format!("Failed to parse stored password hash: {e}")))?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::unauthorized("Invalid email or password"))?;

    let exp = jsonwebtoken::get_current_timestamp() as usize + (24 * 60 * 60);
    let claims = Claims {
        user_id: user.id,
        role: user.role.clone(),
        exp,
    };

    let secret =
        std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::internal(format!("Failed to generate token: {e}")))?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "message": "Login successful",
            "token": token,
        })),
    ))
}
