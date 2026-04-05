use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, IntoActiveModel,
    QueryFilter, Set,
};
use uuid::Uuid;

use crate::entities::{
    financial_records::{self, Model},
    record_type::RecordType,
};

pub async fn create_record(
    db: &DatabaseConnection,
    user_id: Uuid,
    amount: rust_decimal::Decimal,
    r#type: RecordType,
    category: String,
    notes: Option<String>,
    date: chrono::NaiveDate,
) -> Result<Model, DbErr> {
    let record_id = Uuid::new_v4();
    let now = Utc::now();

    let record = financial_records::ActiveModel {
        id: Set(record_id),
        user_id: Set(user_id),
        amount: Set(amount),
        r#type: Set(r#type),
        category: Set(category),
        notes: Set(notes),
        date: Set(date),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        deleted_at: Set(None),
    };

    financial_records::Entity::insert(record).exec(db).await?;

    financial_records::Entity::find_by_id(record_id)
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Record not found after insert".to_string()))
}

pub async fn soft_delete_record(
    db: &DatabaseConnection,
    user_id: Uuid,
    record_id: Uuid,
) -> Result<Model, DbErr> {
    let record = financial_records::Entity::find()
        .filter(financial_records::Column::Id.eq(record_id))
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Record not found".to_string()))?;

    let mut active_record: financial_records::ActiveModel = record.into_active_model();
    active_record.deleted_at = Set(Some(Utc::now().into()));

    active_record.update(db).await
}

pub async fn update_record(
    db: &DatabaseConnection,
    user_id: Uuid,
    record_id: Uuid,
    amount: Option<rust_decimal::Decimal>,
    r#type: Option<RecordType>,
    category: Option<String>,
    notes: Option<Option<String>>,
    date: Option<chrono::NaiveDate>,
) -> Result<Model, DbErr> {
    let record = financial_records::Entity::find()
        .filter(financial_records::Column::Id.eq(record_id))
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Record not found".to_string()))?;

    let mut active_record: financial_records::ActiveModel = record.into_active_model();

    if let Some(amount) = amount {
        active_record.amount = Set(amount);
    }
    if let Some(r#type) = r#type {
        active_record.r#type = Set(r#type);
    }
    if let Some(category) = category {
        active_record.category = Set(category);
    }
    if let Some(notes) = notes {
        active_record.notes = Set(notes);
    }
    if let Some(date) = date {
        active_record.date = Set(date);
    }
    active_record.updated_at = Set(Utc::now().into());

    active_record.update(db).await
}

pub async fn get_record(
    db: &DatabaseConnection,
    user_id: Uuid,
    record_id: Uuid,
) -> Result<Model, DbErr> {
    financial_records::Entity::find()
        .filter(financial_records::Column::Id.eq(record_id))
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .one(db)
        .await?
        .ok_or_else(|| DbErr::RecordNotFound("Record not found".to_string()))
}

pub async fn list_records(
    db: &DatabaseConnection,
    user_id: Uuid,
    record_type: Option<RecordType>,
    category: Option<String>,
    start_date: Option<chrono::NaiveDate>,
    end_date: Option<chrono::NaiveDate>,
) -> Result<Vec<Model>, DbErr> {
    let mut query = financial_records::Entity::find()
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null());

    if let Some(r#type) = record_type {
        query = query.filter(financial_records::Column::Type.eq(r#type));
    }
    if let Some(cat) = category {
        query = query.filter(financial_records::Column::Category.eq(cat));
    }
    if let Some(start) = start_date {
        query = query.filter(financial_records::Column::Date.gte(start));
    }
    if let Some(end) = end_date {
        query = query.filter(financial_records::Column::Date.lte(end));
    }

    query.all(db).await
}
