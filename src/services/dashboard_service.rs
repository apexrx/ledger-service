use sea_orm::{sea_query::Expr, ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QueryOrder, QuerySelect};
use serde::Serialize;

use crate::entities::{financial_records, record_type::RecordType};
use crate::error::AppError;

#[derive(FromQueryResult)]
struct SummaryRow {
    r#type: RecordType,
    total: Option<rust_decimal::Decimal>,
}

#[derive(Serialize)]
pub struct DashboardSummary {
    pub total_income: rust_decimal::Decimal,
    pub total_expense: rust_decimal::Decimal,
}

pub async fn get_summary(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<DashboardSummary, AppError> {
    let rows: Vec<SummaryRow> = financial_records::Entity::find()
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .select_only()
        .column(financial_records::Column::Type)
        .column_as(
            Expr::col(financial_records::Column::Amount).sum(),
            "total",
        )
        .group_by(financial_records::Column::Type)
        .into_model::<SummaryRow>()
        .all(db)
        .await?;

    let mut summary = DashboardSummary {
        total_income: rust_decimal::Decimal::ZERO,
        total_expense: rust_decimal::Decimal::ZERO,
    };

    for row in rows {
        let amount = row.total.unwrap_or(rust_decimal::Decimal::ZERO);
        match row.r#type {
            RecordType::Income => summary.total_income += amount,
            RecordType::Expense => summary.total_expense += amount,
            RecordType::Transfer | RecordType::Adjustment => {}
        }
    }

    Ok(summary)
}

#[derive(FromQueryResult)]
struct CategorySummaryRaw {
    category: String,
    r#type: RecordType,
    total: Option<rust_decimal::Decimal>,
}

#[derive(Serialize)]
pub struct CategorySummaryRow {
    pub category: String,
    pub r#type: RecordType,
    pub total: rust_decimal::Decimal,
}

pub async fn get_category_summary(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<Vec<CategorySummaryRow>, AppError> {
    let raw_rows: Vec<CategorySummaryRaw> = financial_records::Entity::find()
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .select_only()
        .column(financial_records::Column::Category)
        .column(financial_records::Column::Type)
        .column_as(
            Expr::col(financial_records::Column::Amount).sum(),
            "total",
        )
        .group_by(financial_records::Column::Category)
        .group_by(financial_records::Column::Type)
        .into_model::<CategorySummaryRaw>()
        .all(db)
        .await?;

    Ok(raw_rows
        .into_iter()
        .map(|row| CategorySummaryRow {
            category: row.category,
            r#type: row.r#type,
            total: row.total.unwrap_or(rust_decimal::Decimal::ZERO),
        })
        .collect())
}

#[derive(FromQueryResult)]
struct TrendSummaryRaw {
    date: chrono::NaiveDate,
    r#type: RecordType,
    total: Option<rust_decimal::Decimal>,
}

#[derive(Serialize)]
pub struct TrendSummaryRow {
    pub date: chrono::NaiveDate,
    pub r#type: RecordType,
    pub total: rust_decimal::Decimal,
}

pub async fn get_trends(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<Vec<TrendSummaryRow>, AppError> {
    let raw_rows: Vec<TrendSummaryRaw> = financial_records::Entity::find()
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .select_only()
        .column(financial_records::Column::Date)
        .column(financial_records::Column::Type)
        .column_as(
            Expr::col(financial_records::Column::Amount).sum(),
            "total",
        )
        .group_by(financial_records::Column::Date)
        .group_by(financial_records::Column::Type)
        .into_model::<TrendSummaryRaw>()
        .all(db)
        .await?;

    Ok(raw_rows
        .into_iter()
        .map(|row| TrendSummaryRow {
            date: row.date,
            r#type: row.r#type,
            total: row.total.unwrap_or(rust_decimal::Decimal::ZERO),
        })
        .collect())
}

pub async fn get_recent_records(
    db: &DatabaseConnection,
    user_id: uuid::Uuid,
) -> Result<Vec<financial_records::Model>, AppError> {
    financial_records::Entity::find()
        .filter(financial_records::Column::UserId.eq(user_id))
        .filter(financial_records::Column::DeletedAt.is_null())
        .order_by_desc(financial_records::Column::Date)
        .limit(5)
        .all(db)
        .await
        .map_err(AppError::from)
}
