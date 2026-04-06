use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{get, post},
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use http_body_util::BodyExt;
use ledger_service::handlers::auth::LoginRequest;
use ledger_service::AppState;
use ledger_service::entities::role::Role;
use ledger_service::services::user_service;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use tower::ServiceExt;

fn app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState { db };

    let record_read_routes = Router::new()
        .route("/", get(ledger_service::handlers::record_handler::list_records));

    let record_write_routes = Router::new()
        .route("/", post(ledger_service::handlers::record_handler::create_record))
        .route_layer(middleware::from_fn(
            ledger_service::middleware::auth::require_analyst_or_admin,
        ));

    Router::new()
        .route(
            "/auth/register",
            post(ledger_service::handlers::auth::register_handler),
        )
        .route(
            "/auth/login",
            post(ledger_service::handlers::auth::login_handler),
        )
        .nest(
            "/records",
            record_read_routes
                .merge(record_write_routes)
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .with_state(state)
}

async fn setup_db() -> sea_orm::DatabaseConnection {
    let db = Database::connect("sqlite::memory:").await.unwrap();
    Migrator::up(&db, None).await.unwrap();
    db
}

async fn create_analyst_and_login(
    app: &Router,
    db: &sea_orm::DatabaseConnection,
    email: &str,
    password: &str,
) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string();

    user_service::create_user(db, email.to_string(), password_hash, Role::Analyst)
        .await
        .unwrap();

    let login_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::to_string(&LoginRequest {
                        email: email.to_string(),
                        password: password.to_string(),
                    })
                    .unwrap(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_resp.status(), StatusCode::OK);

    let body = login_resp.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    json["token"].as_str().unwrap().to_string()
}

async fn json_body(resp: axum::response::Response) -> serde_json::Value {
    let body = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&body).unwrap()
}

async fn post_raw_record(app: &Router, token: &str, body: &str) -> axum::response::Response {
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/records")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn test_reject_negative_amount() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "neg_amount@test.com", "hunter2").await;

    let resp = post_raw_record(
        &app,
        &token,
        r#"{"amount": -50.00, "type": "Income", "category": "test", "date": "2025-01-01"}"#,
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = json_body(resp).await;
    let details = json.get("details").unwrap_or_else(|| panic!("No details in response: {json}"));
    let fields = details.get("fields")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("No details.fields in response: {json}"));
    let amount_errors = fields.get("amount")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("No fields.amount in response: {json}"));
    assert!(
        amount_errors.iter().any(|e| e.as_str().unwrap().contains("greater than 0")),
        "Expected amount error about being greater than 0, got: {amount_errors:?}"
    );
}

#[tokio::test]
async fn test_reject_empty_category() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "empty_cat@test.com", "hunter2").await;

    let resp = post_raw_record(
        &app,
        &token,
        r#"{"amount": 100.00, "type": "Income", "category": "", "date": "2025-01-01"}"#,
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = json_body(resp).await;
    let details = json.get("details").unwrap_or_else(|| panic!("No details in response: {json}"));
    let fields = details.get("fields")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("No details.fields in response: {json}"));
    let category_errors = fields.get("category")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| panic!("No fields.category in response: {json}"));
    assert!(
        category_errors.iter().any(|e| e.as_str().unwrap().contains("empty")),
        "Expected category error about being empty, got: {category_errors:?}"
    );
}

#[tokio::test]
async fn test_reject_both_negative_amount_and_empty_category() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "both_invalid@test.com", "hunter2").await;

    let resp = post_raw_record(
        &app,
        &token,
        r#"{"amount": -50.00, "type": "Income", "category": "", "date": "2025-01-01"}"#,
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = json_body(resp).await;
    let details = json.get("details").unwrap_or_else(|| panic!("No details in response: {json}"));
    let fields = details.get("fields")
        .and_then(|v| v.as_object())
        .unwrap_or_else(|| panic!("No details.fields in response: {json}"));

    assert!(fields.contains_key("amount"), "Should have amount field errors, got: {fields:?}");
    assert!(fields.contains_key("category"), "Should have category field errors, got: {fields:?}");

    let amount_errors = fields.get("amount").unwrap().as_array().unwrap();
    let category_errors = fields.get("category").unwrap().as_array().unwrap();

    assert!(
        amount_errors.iter().any(|e| e.as_str().unwrap().contains("greater than 0")),
        "Expected amount error about being greater than 0, got: {amount_errors:?}"
    );
    assert!(
        category_errors.iter().any(|e| e.as_str().unwrap().contains("empty")),
        "Expected category error about being empty, got: {category_errors:?}"
    );
}

#[tokio::test]
async fn test_reject_invalid_enum_type() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "invalid_type@test.com", "hunter2").await;

    let resp = post_raw_record(
        &app,
        &token,
        r#"{"amount": 100.00, "type": "refund", "category": "test", "date": "2025-01-01"}"#,
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_reject_missing_required_field() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "missing_field@test.com", "hunter2").await;

    let resp = post_raw_record(
        &app,
        &token,
        r#"{"amount": 100.00, "type": "income", "date": "2025-01-01"}"#,
    )
    .await;

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn test_reject_invalid_email_format() {
    let db = setup_db().await;
    let app = app(db.clone());

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"email": "not-an-email", "password": "hunter2"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let json = json_body(resp).await;
    let fields = json["details"]["fields"].as_object().unwrap();
    let email_errors = fields["email"].as_array().unwrap();
    assert!(
        email_errors.iter().any(|e| e.as_str().unwrap().contains("email")),
        "Expected email format error, got: {email_errors:?}"
    );
}
