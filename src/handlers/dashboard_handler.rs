use axum::{
    extract::State,
    response::IntoResponse,
    Extension,
    Json,
};
use serde_json::json;

use crate::AppState;
use crate::error::AppError;
use crate::middleware::auth::Claims;
use crate::services::dashboard_service;

pub async fn summary_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let summary = dashboard_service::get_summary(&state.db, claims.user_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        Json(json!({
            "status": "success",
            "summary": summary,
        })),
    ))
}

pub async fn category_summary_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let categories = dashboard_service::get_category_summary(&state.db, claims.user_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        Json(json!({
            "status": "success",
            "categories": categories,
        })),
    ))
}

pub async fn trends_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let trends = dashboard_service::get_trends(&state.db, claims.user_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        Json(json!({
            "status": "success",
            "trends": trends,
        })),
    ))
}

pub async fn recent_records_handler(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<impl IntoResponse, AppError> {
    let records = dashboard_service::get_recent_records(&state.db, claims.user_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        Json(json!({
            "status": "success",
            "records": records,
        })),
    ))
}
