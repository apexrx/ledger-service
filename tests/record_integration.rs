use ledger_service::entities::{role::Role, record_type::RecordType};
use ledger_service::services::{record_service, user_service};
use migration::Migrator;
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;

#[tokio::test]
async fn test_soft_delete_excludes_from_list() {
    let db = Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to test DB");

    Migrator::up(&db, None).await.expect("Migrations failed");

    let user = user_service::create_user(
        &db,
        "test@example.com".to_string(),
        "fake_hash".to_string(),
        Role::Admin,
    )
    .await
    .expect("Failed to create user");

    let record = record_service::create_record(
        &db,
        user.id,
        rust_decimal::Decimal::new(100_00, 2),
        RecordType::Income,
        "salary".to_string(),
        Some("Monthly salary".to_string()),
        chrono::NaiveDate::from_ymd_opt(2026, 4, 1).unwrap(),
    )
    .await
    .expect("Failed to create record");

    let records_before = record_service::list_records(&db, user.id, None, None, None, None)
        .await
        .expect("Failed to list records");
    assert_eq!(records_before.len(), 1, "Should have one record before soft delete");

    let deleted_record = record_service::soft_delete_record(&db, record.id)
        .await
        .expect("Failed to soft delete record");
    assert!(
        deleted_record.deleted_at.is_some(),
        "deleted_at should be set after soft delete"
    );

    let records_after = record_service::list_records(&db, user.id, None, None, None, None)
        .await
        .expect("Failed to list records");

    assert_eq!(
        records_after.len(),
        0,
        "Soft-deleted records should be excluded from list"
    );
}
