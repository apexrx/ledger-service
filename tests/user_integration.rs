use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use sea_orm_migration::MigratorTrait;
use migration::Migrator;
use uuid::Uuid;
use ledger_service::entities::users;
use ledger_service::entities::Role;

#[tokio::test]
async fn test_insert_user_with_admin_role() {
    let db = sea_orm::Database::connect("sqlite::memory:")
        .await
        .expect("Failed to connect to test DB");

    Migrator::up(&db, None).await.expect("Migrations failed");

    let user_id = Uuid::new_v4();

    let user = users::ActiveModel {
        id: Set(user_id),
        email: Set("admin@example.com".to_string()),
        password_hash: Set("fake_hash_for_test".to_string()),
        role: Set(Role::Admin),
        ..Default::default()
    };

    let _insert_result = users::Entity::insert(user)
        .exec(&db)
        .await
        .expect("Failed to insert user");

    let fetched_user = users::Entity::find_by_id(user_id)
        .one(&db)
        .await
        .expect("Query failed")
        .expect("User should exist after insert");

    assert_eq!(fetched_user.id, user_id, "UUID should match");
    assert_eq!(fetched_user.role, Role::Admin, "Role enum mapping failed");
    assert_eq!(fetched_user.email, "admin@example.com");
    assert_eq!(fetched_user.password_hash, "fake_hash_for_test");
}
