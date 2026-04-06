use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::{
    role::Role,
    status::Status,
    users::{self, Model},
};
use crate::error::AppError;

pub async fn list_users(db: &DatabaseConnection) -> Result<Vec<Model>, AppError> {
    users::Entity::find()
        .all(db)
        .await
        .map_err(AppError::from)
}

pub async fn create_user_by_admin(
    db: &DatabaseConnection,
    email: String,
    role: Role,
) -> Result<(Model, String), AppError> {
    let temp_password = generate_secure_password();

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(temp_password.as_bytes(), &salt)
        .map_err(|e| AppError::internal(format!("Failed to hash password: {e}")))?
        .to_string();

    let user_id = Uuid::new_v4();

    let user = users::ActiveModel {
        id: Set(user_id),
        email: Set(email),
        password_hash: Set(password_hash),
        role: Set(role),
        status: Set(Status::Active),
        ..Default::default()
    };

    users::Entity::insert(user).exec(db).await?;

    let model = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::internal("User not found after insert".to_string()))?;

    Ok((model, temp_password))
}

fn generate_secure_password() -> String {
    use rand::RngExt;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*";
    let mut rng = rand::rng();
    (0..16)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

pub async fn create_user(
    db: &DatabaseConnection,
    email: String,
    password_hash: String,
    role: Role,
) -> Result<Model, AppError> {
    let user_id = Uuid::new_v4();

    let user = users::ActiveModel {
        id: Set(user_id),
        email: Set(email),
        password_hash: Set(password_hash),
        role: Set(role),
        status: Set(Status::Active),
        ..Default::default()
    };

    users::Entity::insert(user).exec(db).await?;

    users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::internal("User not found after insert".to_string()))
}

pub async fn find_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<Model>, AppError> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
        .map_err(AppError::from)
}

pub async fn update_role(
    db: &DatabaseConnection,
    user_id: Uuid,
    new_role: Role,
) -> Result<Model, AppError> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("User not found".to_string()))?;

    let mut active_user: users::ActiveModel = user.into_active_model();
    active_user.role = Set(new_role);

    active_user.update(db).await.map_err(AppError::from)
}

pub async fn deactivate_user(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Model, AppError> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("User not found".to_string()))?;

    let mut active_user: users::ActiveModel = user.into_active_model();
    active_user.status = Set(Status::Inactive);

    active_user.update(db).await.map_err(AppError::from)
}

pub async fn update_user_status(
    db: &DatabaseConnection,
    user_id: Uuid,
    new_status: Status,
) -> Result<Model, AppError> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| AppError::not_found("User not found".to_string()))?;

    let mut active_user: users::ActiveModel = user.into_active_model();
    active_user.status = Set(new_status);

    active_user.update(db).await.map_err(AppError::from)
}
