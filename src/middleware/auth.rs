use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::entities::role::Role;

#[derive(Serialize, Deserialize, Clone)]
pub struct Claims {
    pub user_id: Uuid,
    pub role: Role,
    pub exp: usize,
}

fn extract_bearer_token(req: &Request) -> Result<&str, StatusCode> {
    let header_value = req
        .headers()
        .get(AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let header_str = header_value
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    header_str
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)
}

pub async fn require_auth(
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = extract_bearer_token(&req)?;

    let secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "super_secret_key".to_string());
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    let token_data = decode::<Claims>(token, &decoding_key, &Validation::default())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert(token_data.claims);

    Ok(next.run(req).await)
}

pub async fn require_admin(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if claims.role != Role::Admin {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(req).await)
}

pub async fn require_analyst_or_admin(
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let claims = req
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    match claims.role {
        Role::Analyst | Role::Admin => Ok(next.run(req).await),
        _ => Err(StatusCode::FORBIDDEN),
    }
}
