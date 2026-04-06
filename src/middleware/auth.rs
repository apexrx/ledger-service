use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::role::Role;
use crate::error::AppError;

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user_id: Uuid,
    pub role: Role,
    pub exp: usize,
}

fn extract_bearer_token(req: &Request) -> Result<&str, AppError> {
    let header_value = req
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .ok_or_else(|| AppError::unauthorized("Missing authorization header"))?;

    let header_str = header_value
        .to_str()
        .map_err(|_| AppError::unauthorized("Invalid authorization header format"))?;

    header_str
        .strip_prefix("Bearer ")
        .ok_or_else(|| AppError::unauthorized("Invalid authorization header, expected Bearer token"))
}

pub async fn require_auth(
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(&req)?;

    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_key".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    let token_data = decode::<Claims>(token, &decoding_key, &Validation::default())
        .map_err(AppError::from)?;

    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}

pub async fn require_admin(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::unauthorized("Authentication required"))?;

    if claims.role != Role::Admin {
        return Err(AppError::forbidden("Admin access required"));
    }

    Ok(next.run(req).await)
}

pub async fn require_analyst_or_admin(
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or_else(|| AppError::unauthorized("Authentication required"))?;

    match claims.role {
        Role::Analyst | Role::Admin => Ok(next.run(req).await),
        _ => Err(AppError::forbidden("Analyst or Admin access required")),
    }
}
