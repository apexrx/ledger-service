use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::borrow::Cow;
use uuid::Uuid;
use validator::Validate;

use crate::AppState;
use crate::error::{AppError, AppJson};
use crate::entities::record_type::RecordType;
use crate::middleware::auth::Claims;
use crate::services::record_service;

fn positive_amount(value: &Decimal) -> Result<(), validator::ValidationError> {
    if *value > Decimal::ZERO {
        Ok(())
    } else {
        Err(validator::ValidationError::new("amount")
            .with_message(Cow::Borrowed("Amount must be greater than 0")))
    }
}

fn non_empty_string(value: &str) -> Result<(), validator::ValidationError> {
    if !value.is_empty() {
        Ok(())
    } else {
        Err(validator::ValidationError::new("category")
            .with_message(Cow::Borrowed("Category cannot be empty")))
    }
}

#[derive(Deserialize, Serialize, Validate)]
pub struct CreateRecordRequest {
    #[validate(custom(function = positive_amount))]
    pub amount: Decimal,
    pub r#type: RecordType,
    #[validate(custom(function = non_empty_string))]
    pub category: String,
    pub notes: Option<String>,
    pub date: chrono::NaiveDate,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateRecordRequest {
    pub amount: Option<Decimal>,
    pub r#type: Option<RecordType>,
    pub category: Option<String>,
    pub notes: Option<Option<String>>,
    pub date: Option<chrono::NaiveDate>,
}

#[derive(Deserialize)]
pub struct ListRecordsQuery {
    pub r#type: Option<RecordType>,
    pub category: Option<String>,
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
}

#[derive(Serialize)]
pub struct RecordResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub r#type: RecordType,
    pub category: String,
    pub notes: Option<String>,
    pub date: chrono::NaiveDate,
}

pub async fn create_record(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    AppJson(payload): AppJson<CreateRecordRequest>,
) -> Result<impl IntoResponse, AppError> {
    payload.validate()?;

    let record = record_service::create_record(
        &state.db,
        claims.user_id,
        payload.amount,
        payload.r#type,
        payload.category,
        payload.notes,
        payload.date,
    )
    .await?;

    Ok((
        axum::http::StatusCode::CREATED,
        axum::Json(json!({
            "status": "success",
            "message": "Record created",
            "record": record,
        })),
    ))
}

pub async fn get_record(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(record_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let record = record_service::get_record(&state.db, claims.user_id, record_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "record": record,
        })),
    ))
}

pub async fn update_record(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(record_id): Path<Uuid>,
    AppJson(payload): AppJson<UpdateRecordRequest>,
) -> Result<impl IntoResponse, AppError> {
    let record = record_service::update_record(
        &state.db,
        claims.user_id,
        record_id,
        payload.amount,
        payload.r#type,
        payload.category,
        payload.notes,
        payload.date,
    )
    .await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "message": "Record updated",
            "record": record,
        })),
    ))
}

pub async fn delete_record(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(record_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    let record = record_service::soft_delete_record(&state.db, claims.user_id, record_id).await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "message": "Record deleted",
            "record": record,
        })),
    ))
}

pub async fn list_records(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ListRecordsQuery>,
) -> Result<impl IntoResponse, AppError> {
    let records = record_service::list_records(
        &state.db,
        claims.user_id,
        params.r#type,
        params.category,
        params.start_date,
        params.end_date,
    )
    .await?;

    Ok((
        axum::http::StatusCode::OK,
        axum::Json(json!({
            "status": "success",
            "records": records,
        })),
    ))
}
