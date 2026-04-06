use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{get, post, put, delete},
};
use http_body_util::BodyExt;
use jsonwebtoken::{encode, EncodingKey, Header};
use ledger_service::handlers::auth::{LoginRequest, RegisterRequest};
use ledger_service::middleware::auth::Claims;
use ledger_service::AppState;
use ledger_service::entities::role::Role;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use tower::ServiceExt;
use uuid::Uuid;

fn app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState { db };

    Router::new()
        .route("/auth/register", post(ledger_service::handlers::auth::register_handler))
        .route("/auth/login", post(ledger_service::handlers::auth::login_handler))
        .route(
            "/test-protected",
            get(|| async { "Success!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .route(
            "/admin-only",
            get(|| async { "Admin!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_admin,
                ))
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .route(
            "/users/{id}/role",
            put(|| async { "Role updated!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_admin,
                ))
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .route(
            "/users/{id}",
            delete(|| async { "User deactivated!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_admin,
                ))
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .route(
            "/records",
            get(|| async { "Records list!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .route(
            "/records",
            post(|| async { "Record created!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_analyst_or_admin,
                ))
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_auth,
                )),
        )
        .route(
            "/records/{id}",
            put(|| async { "Record updated!" })
                .route_layer(middleware::from_fn(
                    ledger_service::middleware::auth::require_analyst_or_admin,
                ))
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

#[tokio::test]
async fn test_login_wrong_password() {
    let db = setup_db().await;
    let app = app(db);

    // Register a user
    let register_payload = RegisterRequest {
        email: "test@example.com".to_string(),
        password: "correct_password".to_string(),
    };

    let register_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&register_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(register_response.status(), StatusCode::CREATED);

    // Attempt login with wrong password
    let login_payload = LoginRequest {
        email: "test@example.com".to_string(),
        password: "wrong_password".to_string(),
    };

    let login_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::UNAUTHORIZED);

    let body = login_response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"], "Authentication required");
    assert_eq!(json["details"]["detail"], "Invalid email or password");
}

#[tokio::test]
async fn test_login_correct_password() {
    let db = setup_db().await;
    let app = app(db);

    // Register a user
    let register_payload = RegisterRequest {
        email: "test2@example.com".to_string(),
        password: "correct_password".to_string(),
    };

    let register_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&register_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(register_response.status(), StatusCode::CREATED);

    // Login with correct password
    let login_payload = LoginRequest {
        email: "test2@example.com".to_string(),
        password: "correct_password".to_string(),
    };

    let login_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);

    let body = login_response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "success");
    assert!(json["token"].as_str().is_some(), "Response should contain a token");
}

#[tokio::test]
async fn test_missing_token() {
    let db = setup_db().await;
    let app = app(db);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test-protected")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_expired_token() {
    let db = setup_db().await;
    let app = app(db);

    let expired_exp = jsonwebtoken::get_current_timestamp() as usize - 3600;
    let claims = Claims {
        user_id: Uuid::new_v4(),
        role: Role::Viewer,
        exp: expired_exp,
    };

    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap();

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/test-protected")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

fn make_token(role: Role, exp_offset_secs: i64) -> String {
    let exp = (jsonwebtoken::get_current_timestamp() as i64 + exp_offset_secs) as usize;
    let claims = Claims {
        user_id: Uuid::new_v4(),
        role,
        exp,
    };
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "super_secret_key".to_string());
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .unwrap()
}

#[tokio::test]
async fn test_admin_only_viewer_forbidden() {
    let db = setup_db().await;
    let app = app(db);

    let token = make_token(Role::Viewer, 3600);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin-only")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_admin_only_admin_allowed() {
    let db = setup_db().await;
    let app = app(db);

    let token = make_token(Role::Admin, 3600);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/admin-only")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_analyst_cannot_manage_users() {
    let db = setup_db().await;
    let app = app(db);

    let token = make_token(Role::Analyst, 3600);

    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/users/some-user-id/role")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"role":"admin"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_viewer_cannot_modify() {
    let db = setup_db().await;
    let app = app(db);

    let token = make_token(Role::Viewer, 3600);

    // Viewer tries to create a financial record
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/records")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"amount":100,"type":"income","category":"test","date":"2025-01-01"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);

    // Viewer tries to update a financial record
    let response = app
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri("/records/some-record-id")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"amount":200}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_viewer_can_read_records() {
    let db = setup_db().await;
    let app = app(db);

    let token = make_token(Role::Viewer, 3600);

    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/records")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_analyst_can_modify_records() {
    let db = setup_db().await;
    let app = app(db);

    let token = make_token(Role::Analyst, 3600);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/records")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(r#"{"amount":100,"type":"income","category":"test","date":"2025-01-01"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

/// Proves the full end-to-end RBAC flow: a freshly registered user defaults to
/// Viewer, logs in for a real JWT, and gets FORBIDDEN when trying to create a
/// financial record through the actual handler (not a stub).
#[tokio::test]
async fn test_viewer_cannot_create_record() {
    use ledger_service::handlers::record_handler;
    use ledger_service::middleware::auth as auth_middleware;

    let db = setup_db().await;

    // Build the real record write route with the actual create_record handler
    let state = AppState { db: db.clone() };
    let record_app = Router::new()
        .route("/records", post(record_handler::create_record))
        .route_layer(middleware::from_fn(auth_middleware::require_analyst_or_admin))
        .route_layer(middleware::from_fn(auth_middleware::require_auth))
        .with_state(state);

    // Step 1: Register a new user (defaults to Viewer)
    let register_payload = RegisterRequest {
        email: "newbie@example.com".to_string(),
        password: "strongpass".to_string(),
    };

    let register_response = app(db.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/register")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&register_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(register_response.status(), StatusCode::CREATED);

    // Step 2: Login to get a real JWT token
    let login_payload = LoginRequest {
        email: "newbie@example.com".to_string(),
        password: "strongpass".to_string(),
    };

    let login_response = app(db)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&login_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(login_response.status(), StatusCode::OK);

    let body = login_response.into_body().collect().await.unwrap().to_bytes();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let token = json["token"].as_str().expect("Login should return a token").to_string();

    // Step 3: Use the token to try creating a record
    let record_payload = serde_json::json!({
        "amount": "500.00",
        "type": "income",
        "category": "Freelance",
        "date": "2026-04-06"
    });

    let response = record_app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/records")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&record_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Step 4: Assert FORBIDDEN -- Viewer role cannot create records
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Viewer role should be forbidden from creating records"
    );

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let error_json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(error_json["error"], "Access denied");
}
