use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct FireProfile {
    pub current_age: i32,
    pub target_retirement_age: i32,
    pub annual_expenses_target: f64,
    pub lean_fire_annual_expenses: Option<f64>,
    pub fat_fire_annual_expenses: Option<f64>,
    pub annual_income: f64,
    pub expected_return_rate: f64,
    pub inflation_rate: f64,
    pub hsa_coverage: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Account {
    pub id: i32,
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
    pub id: i32,
    pub account_id: i32,
    pub balance: f64,
    pub recorded_at: String,
    pub linked_transaction_id: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Transaction {
    pub id: i32,
    pub account_id: i32,
    pub transfer_account_id: Option<i32>,
    pub amount: f64,
    pub description: String,
    pub date: String,
    pub r#type: String,
    pub category: String,
    pub is_contribution: bool,
    pub import_source: String,
    pub generated_balance_id: Option<i32>,
    pub generated_balance_to_id: Option<i32>,
    pub paycheck_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct ImportMapping {
    pub id: i32,
    pub name: String,
    pub config: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct PaycheckDeduction {
    pub label: String,
    pub amount: f64,
    pub pre_tax: bool,
    pub contribution_account_type: Option<String>,
    pub account_id: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct EmployerMatchItem {
    pub label: String,
    pub amount: f64,
    pub account_id: Option<i32>,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct Paycheck {
    pub id: i32,
    pub pay_date: String,
    pub employer: String,
    pub pay_period: String,
    pub gross_amount: f64,
    pub net_amount: f64,
    pub federal_tax: f64,
    pub state_tax: f64,
    pub local_tax: f64,
    pub social_security_tax: f64,
    pub medicare_tax: f64,
    pub deductions: Vec<PaycheckDeduction>,
    pub employer_match: Vec<EmployerMatchItem>,
    pub import_source: String,
    pub created_at: String,
    pub updated_at: String,
}
