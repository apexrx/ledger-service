use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::AppState;
use crate::entities::record_type::RecordType;
use crate::services::record_service;

#[derive(Deserialize)]
pub struct CreateRecordRequest {
    pub amount: Decimal,
    pub r#type: RecordType,
    pub category: String,
    pub notes: Option<String>,
    pub date: chrono::NaiveDate,
}

#[derive(Deserialize)]
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
    Json(payload): Json<CreateRecordRequest>,
) -> impl IntoResponse {
    // TODO: get user_id from Claims extension
    let dummy_user = Uuid::new_v4();

    match record_service::create_record(
        &state.db,
        dummy_user,
        payload.amount,
        payload.r#type,
        payload.category,
        payload.notes,
        payload.date,
    )
    .await
    {
        Ok(record) => (
            StatusCode::CREATED,
            Json(json!({
                "status": "success",
                "message": "Record created",
                "record": record,
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

pub async fn update_record(
    State(state): State<AppState>,
    Path(record_id): Path<Uuid>,
    Json(payload): Json<UpdateRecordRequest>,
) -> impl IntoResponse {
    match record_service::update_record(
        &state.db,
        record_id,
        payload.amount,
        payload.r#type,
        payload.category,
        payload.notes,
        payload.date,
    )
    .await
    {
        Ok(record) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Record updated",
                "record": record,
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

pub async fn delete_record(
    State(state): State<AppState>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    match record_service::soft_delete_record(&state.db, record_id).await {
        Ok(record) => (
            StatusCode::OK,
            Json(json!({
                "status": "success",
                "message": "Record deleted",
                "record": record,
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

pub async fn list_records(
    State(state): State<AppState>,
    Query(params): Query<ListRecordsQuery>,
) -> impl IntoResponse {
    // TODO: get user_id from Claims extension
    let dummy_user = Uuid::new_v4();

    match record_service::list_records(
        &state.db,
        dummy_user,
        params.r#type,
        params.category,
        params.start_date,
        params.end_date,
    )
    .await
    {
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
