use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Extension,
    Json,
};
use serde_json::json;

use crate::AppState;
use crate::middleware::auth::Claims;
use crate::services::dashboard_service;

pub async fn summary_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match dashboard_service::get_summary(&state.db, claims.user_id).await {
        Ok(summary) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "summary": summary,
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

pub async fn category_summary_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match dashboard_service::get_category_summary(&state.db, claims.user_id).await {
        Ok(categories) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "categories": categories,
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

pub async fn trends_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match dashboard_service::get_trends(&state.db, claims.user_id).await {
        Ok(trends) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "trends": trends,
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

pub async fn recent_records_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    match dashboard_service::get_recent_records(&state.db, claims.user_id).await {
        Ok(records) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "records": records,
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
