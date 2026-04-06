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
use ledger_service::entities::record_type::RecordType;
use ledger_service::entities::role::Role;
use ledger_service::handlers::record_handler::CreateRecordRequest;
use ledger_service::services::user_service;
use migration::{Migrator, MigratorTrait};
use sea_orm::Database;
use tower::ServiceExt;

fn app(db: sea_orm::DatabaseConnection) -> Router {
    let state = AppState { db };

    let dashboard_routes = Router::new()
        .route("/summary", get(ledger_service::handlers::dashboard_handler::summary_handler))
        .route("/categories", get(ledger_service::handlers::dashboard_handler::category_summary_handler))
        .route("/trends", get(ledger_service::handlers::dashboard_handler::trends_handler))
        .route("/recent", get(ledger_service::handlers::dashboard_handler::recent_records_handler))
        .route_layer(middleware::from_fn(
            ledger_service::middleware::auth::require_auth,
        ));

    let record_routes = Router::new()
        .route("/", get(ledger_service::handlers::record_handler::list_records))
        .route("/", post(ledger_service::handlers::record_handler::create_record))
        .route_layer(middleware::from_fn(
            ledger_service::middleware::auth::require_analyst_or_admin,
        ))
        .route_layer(middleware::from_fn(
            ledger_service::middleware::auth::require_auth,
        ));

    Router::new()
        .route(
            "/auth/login",
            post(ledger_service::handlers::auth::login_handler),
        )
        .nest("/dashboard", dashboard_routes)
        .nest("/records", record_routes)
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

#[tokio::test]
async fn test_dashboard_empty_dataset() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "empty@test.com", "hunter2").await;

    // Request summary with zero records
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/summary")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert_eq!(json["status"], "success");
    assert_eq!(json["summary"]["total_income"], "0");
    assert_eq!(json["summary"]["total_expense"], "0");

    // Categories should also return an empty list
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/categories")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json["categories"].as_array().unwrap().is_empty());

    // Trends should also return an empty list
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/trends")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json["trends"].as_array().unwrap().is_empty());

    // Recent records should return an empty list
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/recent")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert!(json["records"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn test_dashboard_summary_correct_totals() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "totals@test.com", "hunter2").await;

    // Create income records
    let create_income = |app: &Router, token: &str, amount: rust_decimal::Decimal, category: &str| {
        let payload = CreateRecordRequest {
            amount,
            r#type: RecordType::Income,
            category: category.to_string(),
            notes: None,
            date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        };
        let app = app.clone();
        let token = token.to_string();
        async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/records")
                        .header("Authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&payload).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::CREATED);
            json_body(resp).await
        }
    };

    let _income1 = create_income(&app.clone(), &token, rust_decimal::Decimal::new(5000, 2), "salary").await;
    let _income2 = create_income(&app.clone(), &token, rust_decimal::Decimal::new(3000, 2), "freelance").await;

    // Create expense records
    let create_expense = |app: &Router, token: &str, amount: rust_decimal::Decimal, category: &str| {
        let payload = CreateRecordRequest {
            amount,
            r#type: RecordType::Expense,
            category: category.to_string(),
            notes: None,
            date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        };
        let app = app.clone();
        let token = token.to_string();
        async move {
            let resp = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/records")
                        .header("Authorization", format!("Bearer {token}"))
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::to_string(&payload).unwrap()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(resp.status(), StatusCode::CREATED);
            json_body(resp).await
        }
    };

    let _expense1 = create_expense(&app.clone(), &token, rust_decimal::Decimal::new(1000, 2), "food").await;
    let _expense2 = create_expense(&app.clone(), &token, rust_decimal::Decimal::new(2000, 2), "rent").await;

    // Check summary: income should be 50+30=80, expense should be 10+20=30
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/summary")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    assert_eq!(json["summary"]["total_income"], "80");
    assert_eq!(json["summary"]["total_expense"], "30");
}

#[tokio::test]
async fn test_dashboard_categories_grouping() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "categories@test.com", "hunter2").await;

    // Create records in different categories
    let payloads = [
        (RecordType::Income, "salary", rust_decimal::Decimal::new(5000, 2)),
        (RecordType::Income, "salary", rust_decimal::Decimal::new(5000, 2)),
        (RecordType::Expense, "food", rust_decimal::Decimal::new(1000, 2)),
        (RecordType::Expense, "food", rust_decimal::Decimal::new(500, 2)),
        (RecordType::Expense, "rent", rust_decimal::Decimal::new(2000, 2)),
    ];

    for (r#type, category, amount) in &payloads {
        let payload = CreateRecordRequest {
            amount: *amount,
            r#type: *r#type,
            category: category.to_string(),
            notes: None,
            date: chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(),
        };
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/records")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/categories")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    let categories = json["categories"].as_array().unwrap();

    // Should have 3 groups: salary(income), food(expense), rent(expense)
    assert_eq!(categories.len(), 3);

    // Find and verify each group
    let salary: Vec<_> = categories.iter().filter(|c| c["category"] == "salary" && c["type"] == "Income").collect();
    assert_eq!(salary.len(), 1);
    assert_eq!(salary[0]["total"], "100"); // 50+50

    let food: Vec<_> = categories.iter().filter(|c| c["category"] == "food" && c["type"] == "Expense").collect();
    assert_eq!(food.len(), 1);
    assert_eq!(food[0]["total"], "15"); // 10+5

    let rent: Vec<_> = categories.iter().filter(|c| c["category"] == "rent" && c["type"] == "Expense").collect();
    assert_eq!(rent.len(), 1);
    assert_eq!(rent[0]["total"], "20");
}

#[tokio::test]
async fn test_dashboard_trends_grouping() {
    let db = setup_db().await;
    let app = app(db.clone());

    let token = create_analyst_and_login(&app, &db, "trends@test.com", "hunter2").await;

    // Create income on two different dates
    let payloads = [
        (RecordType::Income, chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(), rust_decimal::Decimal::new(5000, 2)),
        (RecordType::Income, chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(), rust_decimal::Decimal::new(3000, 2)),
        (RecordType::Expense, chrono::NaiveDate::from_ymd_opt(2025, 3, 1).unwrap(), rust_decimal::Decimal::new(1000, 2)),
        (RecordType::Income, chrono::NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(), rust_decimal::Decimal::new(2000, 2)),
        (RecordType::Expense, chrono::NaiveDate::from_ymd_opt(2025, 3, 15).unwrap(), rust_decimal::Decimal::new(500, 2)),
    ];

    for (r#type, date, amount) in &payloads {
        let payload = CreateRecordRequest {
            amount: *amount,
            r#type: *r#type,
            category: "test".to_string(),
            notes: None,
            date: *date,
        };
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/records")
                    .header("Authorization", format!("Bearer {token}"))
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&payload).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::CREATED);
    }

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/dashboard/trends")
                .header("Authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);
    let json = json_body(resp).await;
    let trends = json["trends"].as_array().unwrap();

    // Should have 4 groups: Income 2025-03-01, Expense 2025-03-01, Income 2025-03-15, Expense 2025-03-15
    assert_eq!(trends.len(), 4);

    let income_mar1: Vec<_> = trends.iter().filter(|t| t["date"] == "2025-03-01" && t["type"] == "Income").collect();
    assert_eq!(income_mar1.len(), 1);
    assert_eq!(income_mar1[0]["total"], "80"); // 50+30

    let expense_mar1: Vec<_> = trends.iter().filter(|t| t["date"] == "2025-03-01" && t["type"] == "Expense").collect();
    assert_eq!(expense_mar1.len(), 1);
    assert_eq!(expense_mar1[0]["total"], "10");

    let income_mar15: Vec<_> = trends.iter().filter(|t| t["date"] == "2025-03-15" && t["type"] == "Income").collect();
    assert_eq!(income_mar15.len(), 1);
    assert_eq!(income_mar15[0]["total"], "20");
}
