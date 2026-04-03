use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel, QueryFilter, Set};
use uuid::Uuid;

use crate::entities::{
    role::Role,
    status::Status,
    users::{self, Model},
};

pub async fn create_user(
    db: &DatabaseConnection,
    email: String,
    password_hash: String,
    role: Role,
) -> Result<Model, DbErr> {
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
        .ok_or_else(|| DbErr::RecordNotFound("User not found after insert".to_string()))
}

pub async fn find_by_email(
    db: &DatabaseConnection,
    email: &str,
) -> Result<Option<Model>, DbErr> {
    users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await
}

pub async fn update_role(
    db: &DatabaseConnection,
    user_id: Uuid,
    new_role: Role,
) -> Result<Model, DbErr> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("User not found".to_string()))?;

    let mut active_user: users::ActiveModel = user.into_active_model();
    active_user.role = Set(new_role);

    active_user.update(db).await
}

pub async fn deactivate_user(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> Result<Model, DbErr> {
    let user = users::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("User not found".to_string()))?;

    let mut active_user: users::ActiveModel = user.into_active_model();
    active_user.status = Set(Status::Inactive);

    active_user.update(db).await
}
