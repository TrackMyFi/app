use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct FireProfile {
    pub current_age: i64,
    pub target_retirement_age: i64,
    pub annual_expenses_target: f64,
    pub lean_fire_annual_expenses: Option<f64>,
    pub fat_fire_annual_expenses: Option<f64>,
    pub annual_income: f64,
    pub expected_return_rate: f64,
    pub inflation_rate: f64,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Account {
    pub id: i64,
    pub name: String,
    pub r#type: String,
    pub institution: Option<String>,
    pub is_active: bool,
    pub include_in_fire_calculations: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct AccountBalance {
    pub id: i64,
    pub account_id: i64,
    pub balance: f64,
    pub recorded_at: String,
}
