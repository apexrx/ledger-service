use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// User account status
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize
)]
#[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
pub enum Status {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "suspended")]
    Suspended,
    #[sea_orm(string_value = "deleted")]
    Deleted,
}

impl Status {
    /// Check if user can log in
    pub fn can_login(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Check if account is soft-deleted
    pub fn is_deleted(&self) -> bool {
        matches!(self, Self::Deleted)
    }
}
