use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Financial record transaction type
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
pub enum RecordType {
    #[sea_orm(string_value = "income")]
    Income,
    #[sea_orm(string_value = "expense")]
    Expense,
    #[sea_orm(string_value = "transfer")]
    Transfer,
    #[sea_orm(string_value = "adjustment")]
    Adjustment,
}

impl RecordType {
    /// Check if this type affects net balance positively
    pub fn is_positive(&self) -> bool {
        matches!(self, Self::Income)
    }

    /// Check if this type affects net balance negatively
    pub fn is_negative(&self) -> bool {
        matches!(self, Self::Expense)
    }
}
