use axum::{Router, extract::State, http::StatusCode, response::IntoResponse, Json, routing::get};
use dotenvy::dotenv;
use sea_orm::Database;
use serde_json::json;

#[derive(Clone)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
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

async fn db_status_handler(State(state): State<AppState>) -> impl IntoResponse {
    match state.db.ping().await {
        Ok(_) => (
            StatusCode::OK,
            Json(json!({
                "status": "healthy",
                "database": "connected"
            })),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({
                "status": "unhealthy",
                "database": "disconnected",
                "error": e.to_string()
            })),
        ),
    }
}
