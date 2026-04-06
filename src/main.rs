use axum::{Router, extract::State, http::StatusCode, middleware, response::IntoResponse, Json, routing::{get, post, put, patch, delete}};
use dotenvy::dotenv;
use ledger_service::{AppState, handlers, middleware::auth, error::AppError};
use sea_orm::Database;
use serde_json::json;

fn user_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::user_handler::list_users).post(handlers::user_handler::create_user))
        .route("/{id}/role", put(handlers::user_handler::update_role))
        .route("/{id}/status", patch(handlers::user_handler::update_user_status))
        .route("/{id}", delete(handlers::user_handler::delete_user))
        .route_layer(middleware::from_fn(auth::require_admin))
        .route_layer(middleware::from_fn(auth::require_auth))
}

fn record_routes() -> Router<AppState> {
    let read_routes = Router::new()
        .route("/", get(handlers::record_handler::list_records));

    let write_routes = Router::new()
        .route("/", post(handlers::record_handler::create_record))
        .route("/{id}", put(handlers::record_handler::update_record).delete(handlers::record_handler::delete_record))
        .route_layer(middleware::from_fn(auth::require_analyst_or_admin));

    read_routes
        .merge(write_routes)
        .route_layer(middleware::from_fn(auth::require_auth))
}

fn dashboard_routes() -> Router<AppState> {
    Router::new()
        .route("/summary", get(handlers::dashboard_handler::summary_handler))
        .route("/categories", get(handlers::dashboard_handler::category_summary_handler))
        .route("/trends", get(handlers::dashboard_handler::trends_handler))
        .route("/recent", get(handlers::dashboard_handler::recent_records_handler))
        .route_layer(middleware::from_fn(auth::require_auth))
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be in .env");

    let db_connection = match Database::connect(&database_url).await {
        Ok(conn) => {
            println!("Successfully connected to the database");
            conn
        }
        Err(err) => {
            eprintln!("Failed to connected to the database: {}", err);
            std::process::exit(1);
        }
    };

    let state = AppState {
        db: db_connection,
    };

    let app = Router::new()
        .route("/health", get(health_handler))
        .route("/db-status", get(db_status_handler))
        .route("/auth/register", post(handlers::auth::register_handler))
        .route("/auth/login", post(handlers::auth::login_handler))
        .nest("/users", user_routes())
        .nest("/records", record_routes())
        .nest("/dashboard", dashboard_routes())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("Failed to bind to port 3000");

    println!("Server running on http://127.0.0.1:3000");

    axum::serve(listener, app).await.unwrap();
}

async fn health_handler() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn db_status_handler(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    state.db.ping().await?;
    
    Ok((
        StatusCode::OK,
        Json(json!({
            "status": "healthy",
            "database": "connected"
        })),
    ))
}
