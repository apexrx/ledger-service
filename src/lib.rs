pub mod entities;
pub mod error;
pub mod handlers;
pub mod middleware;
pub mod services;

#[derive(Clone)]
pub struct AppState {
    pub db: sea_orm::DatabaseConnection,
}
