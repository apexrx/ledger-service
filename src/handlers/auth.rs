use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, SaltString},
    Argon2, PasswordVerifier,
};
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::super::AppState;

use ledger_service::entities::role::Role;
use ledger_service::services::user_service;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct Claims {
    pub user_id: Uuid,
    pub role: Role,
    pub exp: usize,
}

pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .expect("Failed to hash password")
        .to_string();

    match user_service::create_user(&state.db, payload.email, password_hash, Role::Viewer).await {
        Ok(user) => (
            StatusCode::CREATED,
            Json(json!({
                "status": "success",
                "message": "User registered successfully",
                "user": {
                    "id": user.id,
                    "email": user.email,
                    "role": user.role,
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

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let user = match user_service::find_by_email(&state.db, &payload.email).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "status": "error",
                    "message": "Invalid email or password",
                })),
            );
        }
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": e.to_string(),
                })),
            );
        }
    };

    let parsed_hash = match PasswordHash::new(&user.password_hash) {
        Ok(hash) => hash,
        Err(_) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "status": "error",
                    "message": "Failed to parse stored password hash",
                })),
            );
        }
    };

    match Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash) {
        Ok(()) => {
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
            .expect("Failed to generate token");

            (
                StatusCode::OK,
                Json(json!({
                    "status": "success",
                    "message": "Login successful",
                    "token": token,
                })),
            )
        }
        Err(_) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "status": "error",
                "message": "Invalid email or password",
            })),
        ),
    }
}
