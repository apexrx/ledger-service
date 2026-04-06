use axum::{
    extract::{FromRequest, Request, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;

#[derive(Debug)]
pub enum AppError {
    DatabaseError(String),
    ValidationError(String),
    ValidationErrorWithDetails { message: String, details: serde_json::Value },
    NotFound(String),
    Unauthorized(String),
    Forbidden(String),
    Conflict(String),
    InternalError(String),
}

impl AppError {
    pub fn database(msg: impl Into<String>) -> Self {
        AppError::DatabaseError(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        AppError::ValidationError(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        AppError::NotFound(msg.into())
    }

    pub fn unauthorized(msg: impl Into<String>) -> Self {
        AppError::Unauthorized(msg.into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        AppError::Forbidden(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        AppError::Conflict(msg.into())
    }

    pub fn internal(msg: impl Into<String>) -> Self {
        AppError::InternalError(msg.into())
    }

    pub fn with_details(self, details: serde_json::Value) -> Self {
        let message = match self {
            AppError::ValidationError(m) => m,
            AppError::DatabaseError(m)
            | AppError::NotFound(m)
            | AppError::Unauthorized(m)
            | AppError::Forbidden(m)
            | AppError::Conflict(m)
            | AppError::InternalError(m) => m,
            AppError::ValidationErrorWithDetails { message, .. } => message,
        };
        AppError::ValidationErrorWithDetails { message, details }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            AppError::ValidationError(msg) => write!(f, "Validation error: {msg}"),
            AppError::ValidationErrorWithDetails { message, .. } => write!(f, "Validation error: {message}"),
            AppError::NotFound(msg) => write!(f, "Not found: {msg}"),
            AppError::Unauthorized(msg) => write!(f, "Unauthorized: {msg}"),
            AppError::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
            AppError::Conflict(msg) => write!(f, "Conflict: {msg}"),
            AppError::InternalError(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

#[derive(Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message, details) = match &self {
            AppError::DatabaseError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
                Some(json!({ "detail": msg })),
            ),
            AppError::ValidationError(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "Validation failed",
                Some(json!({ "detail": msg })),
            ),
            AppError::ValidationErrorWithDetails { message, details } => (
                StatusCode::UNPROCESSABLE_ENTITY,
                message.as_str(),
                Some(details.clone()),
            ),
            AppError::NotFound(msg) => (
                StatusCode::NOT_FOUND,
                "Resource not found",
                Some(json!({ "detail": msg })),
            ),
            AppError::Unauthorized(msg) => (
                StatusCode::UNAUTHORIZED,
                "Authentication required",
                Some(json!({ "detail": msg })),
            ),
            AppError::Forbidden(msg) => (
                StatusCode::FORBIDDEN,
                "Access denied",
                Some(json!({ "detail": msg })),
            ),
            AppError::Conflict(msg) => (
                StatusCode::CONFLICT,
                "Resource conflict",
                Some(json!({ "detail": msg })),
            ),
            AppError::InternalError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error",
                Some(json!({ "detail": msg })),
            ),
        };

        let body = ErrorResponse {
            error: error_message.to_string(),
            details,
        };

        (status, Json(body)).into_response()
    }
}

impl From<DbErr> for AppError {
    fn from(err: DbErr) -> Self {
        match err {
            DbErr::RecordNotFound(msg) => AppError::not_found(msg),
            DbErr::Conn(err) => AppError::database(format!("Connection error: {err}")),
            DbErr::Exec(err) => AppError::database(format!("Execution error: {err}")),
            DbErr::Query(err) => AppError::database(format!("Query error: {err}")),
            DbErr::RecordNotInserted => AppError::internal("Record not inserted".to_string()),
            DbErr::UnpackInsertId => AppError::internal("Failed to get insert ID".to_string()),
            _ => AppError::internal(format!("Database error: {err}")),
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::validation(format!("Invalid JSON: {err}"))
    }
}

impl From<jsonwebtoken::errors::Error> for AppError {
    fn from(err: jsonwebtoken::errors::Error) -> Self {
        match err.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => {
                AppError::unauthorized("Token has expired")
            }
            _ => AppError::unauthorized(format!("Invalid token: {err}")),
        }
    }
}

impl From<validator::ValidationErrors> for AppError {
    fn from(err: validator::ValidationErrors) -> Self {
        let field_errors: serde_json::Value = err
            .field_errors()
            .iter()
            .map(|(field, errors)| {
                let messages: Vec<serde_json::Value> = errors
                    .iter()
                    .map(|e| {
                        e.message
                            .as_ref()
                            .map(|m| serde_json::Value::String(m.to_string()))
                            .unwrap_or_else(|| serde_json::Value::String("validation failed".to_string()))
                    })
                    .collect();
                (field.clone(), serde_json::Value::Array(messages))
            })
            .collect();

        AppError::ValidationErrorWithDetails {
            message: "Validation failed".to_string(),
            details: serde_json::json!({ "fields": field_errors }),
        }
    }
}

/// Intercept Axum's JSON deserialization failures (bad enum, missing required fields,
/// wrong types) and re-emit them with our standardized error shape at 422.
impl From<JsonRejection> for AppError {
    fn from(rejection: JsonRejection) -> Self {
        AppError::ValidationErrorWithDetails {
            message: "Validation failed".to_string(),
            details: json!({ "detail": rejection.body_text() }),
        }
    }
}

/// Custom JSON extractor that converts Axum's default rejection into our AppError shape.
/// Use this instead of `axum::Json` in handlers to get consistent error responses.
pub struct AppJson<T>(pub T);

impl<T, S> FromRequest<S> for AppJson<T>
where
    T: serde::de::DeserializeOwned,
    S: Send + Sync,
    Json<T>: FromRequest<S, Rejection = JsonRejection>,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(value) = Json::<T>::from_request(req, state).await?;
        Ok(AppJson(value))
    }
}
