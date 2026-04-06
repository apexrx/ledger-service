use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
    middleware,
    routing::{get, post, put},
};
use argon2::{
    password_hash::{rand_core::OsRng, SaltString},
    Argon2, PasswordHasher,
};
use http_body_util::BodyExt;
use ledger_service::handlers::auth::LoginRequest;
use ledger_service::handlers::record_handler::{
    CreateRecordRequest, UpdateRecordRequest,
};
use ledger_service::AppState;
use ledger_service::entities::record_type::RecordType;
use ledger_service::entities::role::Role;
use ledger_service::services::user_service;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use tower::ServiceExt;

fn app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState { db };

    let record_read_routes = Router::new()
        .route("/", get(ledger_service::handlers::record_handler::list_records))
        .route("/{id}", get(ledger_service::handlers::record_handler::get_record));

    let record_write_routes = Router::new()
        .route("/", post(ledger_service::handlers::record_handler::create_record))
        .route(
            "/{id}",
            put(ledger_service::handlers::record_handler::update_record)
                .delete(ledger_service::handlers::record_handler::delete_record),
        )
        .route_layer(middleware::from_fn(
            ledger_service::middleware::auth::require_analyst_or_admin,
        ));

    Router::new()
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

async fn create_record(app: &Router, token: &str, payload: &CreateRecordRequest) -> serde_json::Value {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/records")
                .header("Authorization", format!("Bearer {token}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::CREATED);
    json_body(resp).await
}

async fn list_records(app: &Router, token: &str, query: &str) -> serde_json::Value {
    let uri = if query.is_empty() {
        "/records".to_string()
    } else {
        format!("/records?{query}")
    };

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(&uri)
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    json_body(resp).await
}

#[tokio::test]
async fn test_filtering_by_category() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "filter_cat@test.com", "hunter2").await;

    let salary_record = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(5000, 2),
        r#type: RecordType::Income,
        category: "salary".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    }).await;

    let _freelance_record = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(3000, 2),
        r#type: RecordType::Income,
        category: "freelance".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(),
    }).await;

    let filtered = list_records(&app, &token, "category=salary").await;
    let records = filtered["records"].as_array().unwrap();

    assert_eq!(records.len(), 1, "Should return exactly one record");
    assert_eq!(records[0]["category"], "salary");
    assert_eq!(records[0]["id"], salary_record["record"]["id"]);
}

#[tokio::test]
async fn test_filtering_by_type() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "filter_type@test.com", "hunter2").await;

    let _income = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(5000, 2),
        r#type: RecordType::Income,
        category: "work".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    }).await;

    let expense = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(2000, 2),
        r#type: RecordType::Expense,
        category: "food".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 5).unwrap(),
    }).await;

    // Filter by type=Expense
    let filtered = list_records(&app, &token, "type=Expense").await;
    let records = filtered["records"].as_array().unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["type"], "Expense");
    assert_eq!(records[0]["id"], expense["record"]["id"]);
}

#[tokio::test]
async fn test_filtering_by_date_range() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "filter_date@test.com", "hunter2").await;

    let _march = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(1000, 2),
        r#type: RecordType::Income,
        category: "gig".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    }).await;

    let april = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(2000, 2),
        r#type: RecordType::Income,
        category: "gig".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    }).await;

    let _may = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(3000, 2),
        r#type: RecordType::Income,
        category: "gig".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 5, 1).unwrap(),
    }).await;

    let filtered = list_records(&app, &token, "start_date=2025-04-01&end_date=2025-04-30").await;
    let records = filtered["records"].as_array().unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"], april["record"]["id"]);
}

#[tokio::test]
async fn test_combined_filters() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "filter_combined@test.com", "hunter2").await;

    let _march = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(1000, 2),
        r#type: RecordType::Income,
        category: "salary".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    }).await;

    let april_salary = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(2000, 2),
        r#type: RecordType::Income,
        category: "salary".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 4, 1).unwrap(),
    }).await;

    let _april_freelance = create_record(&app, &token, &CreateRecordRequest {
        amount: rust_decimal::Decimal::new(500, 2),
        r#type: RecordType::Income,
        category: "freelance".to_string(),
        notes: None,
        date: chrono::NaiveDate::from_ymd_opt(2025, 4, 15).unwrap(),
    }).await;

    let filtered = list_records(&app, &token, "category=salary&start_date=2025-04-01").await;
    let records = filtered["records"].as_array().unwrap();

    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["id"], april_salary["record"]["id"]);
}

#[tokio::test]
async fn test_record_lifecycle() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "lifecycle@test.com", "hunter2").await;
    let auth_header = format!("Bearer {token}");

    let create_payload = CreateRecordRequest {
        amount: rust_decimal::Decimal::new(5000, 2), // 50.00
        r#type: RecordType::Income,
        category: "salary".to_string(),
        notes: Some("Monthly salary".to_string()),
        date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
    };

    let create_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/records")
                .header("Authorization", &auth_header)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&create_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let create_json = json_body(create_resp).await;
    assert_eq!(create_json["status"], "success");
    assert_eq!(create_json["message"], "Record created");
    let record_id = create_json["record"]["id"].as_str().unwrap().to_string();
    assert_eq!(
        create_json["record"]["amount"].as_str().unwrap(),
        "50"
    );
    assert_eq!(create_json["record"]["category"], "salary");

    let get_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/records/{record_id}"))
                .header("Authorization", &auth_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_resp.status(), StatusCode::OK);
    let get_json = json_body(get_resp).await;
    assert_eq!(get_json["status"], "success");
    assert_eq!(get_json["record"]["id"], record_id);
    assert_eq!(get_json["record"]["category"], "salary");
    assert_eq!(get_json["record"]["amount"].as_str().unwrap(), "50");

    let update_payload = UpdateRecordRequest {
        amount: Some(rust_decimal::Decimal::new(7500, 2)), // 75.00
        r#type: None,
        category: Some("freelance".to_string()),
        notes: Some(Some("One-off project".to_string())),
        date: None,
    };

    let update_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PUT")
                .uri(format!("/records/{record_id}"))
                .header("Authorization", &auth_header)
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&update_payload).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(update_resp.status(), StatusCode::OK);
    let update_json = json_body(update_resp).await;
    assert_eq!(update_json["status"], "success");
    assert_eq!(update_json["message"], "Record updated");
    assert_eq!(update_json["record"]["amount"].as_str().unwrap(), "75");
    assert_eq!(update_json["record"]["category"], "freelance");

    let delete_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/records/{record_id}"))
                .header("Authorization", &auth_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(delete_resp.status(), StatusCode::OK);
    let delete_json = json_body(delete_resp).await;
    assert_eq!(delete_json["status"], "success");
    assert_eq!(delete_json["message"], "Record deleted");

    let get_deleted_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/records/{record_id}"))
                .header("Authorization", &auth_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(get_deleted_resp.status(), StatusCode::NOT_FOUND);

    let list_resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/records")
                .header("Authorization", &auth_header)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(list_resp.status(), StatusCode::OK);
    let list_json = json_body(list_resp).await;
    assert_eq!(list_json["status"], "success");
    assert!(
        list_json["records"].as_array().unwrap().is_empty(),
        "Deleted record should not appear in the list"
    );
}
