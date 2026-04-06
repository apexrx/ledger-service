use sea_orm_migration::{prelude::*, schema::*};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {

        // Users table
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Users::Email)
                            .string()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(Users::PasswordHash)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Users::Role)
                            .string()
                            .not_null()
                            .default("user"),
                    )
                    .col(
                        ColumnDef::new(Users::Status)
                            .string()
                            .not_null()
                            .default("active"),
                    )
                    .col(
                        ColumnDef::new(Users::CreatedAt)
                            .timestamp()
                            .not_null()
                            .default(Expr::current_timestamp())
                    )
                    .to_owned()
            )
            .await?;

        // Financial records table
        manager
            .create_table(
                Table::create()
                    .table(FinancialRecords::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FinancialRecords::Id)
                            .uuid()
                            .primary_key()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::UserId)
                            .uuid()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::Amount)
                            .decimal_len(10, 2)
                            .not_null()
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::Type)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::Category)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::Notes)
                            .string()
                            .null(),
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::Date)
                            .date()
                            .not_null()
                            .default(Expr::current_timestamp())
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp())
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp())
                    )
                    .col(
                        ColumnDef::new(FinancialRecords::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_financial_records_user")
                            .from(FinancialRecords::Table, FinancialRecords::UserId)
                            .to(Users::Table, Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned()
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FinancialRecords::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
    Email,
    PasswordHash,
    Role,
    Status,
    CreatedAt,
}

#[derive(DeriveIden)]
enum FinancialRecords {
    Table,
    Id,
    UserId,
    Amount,
    Type,
    Category,
    Notes,
    Date,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
}
