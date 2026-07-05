use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct FireProfile {
    pub date_of_birth: Option<String>,
    pub target_retirement_age: i32,
    pub annual_expenses_target: f64,
    pub lean_fire_annual_expenses: Option<f64>,
    pub fat_fire_annual_expenses: Option<f64>,
    pub annual_income: f64,
    pub expected_return_rate: f64,
    pub inflation_rate: f64,
    /// Safe withdrawal rate; 0.04 = the 4% rule (FIRE number = expenses × 25).
    #[serde(default = "default_withdrawal_rate")]
    pub withdrawal_rate: f64,
    pub hsa_coverage: String,
    #[serde(default)]
    pub onboarding_completed: bool,
}

fn default_withdrawal_rate() -> f64 {
    0.04
}

#[derive(Serialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct DeletionPreview {
    pub transactions: i64,
    pub paychecks: i64,
    pub balance_snapshots: i64,
    pub budget_months: i64,
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
    /// SimpleFIN account id this account is linked to, when bank sync is set up.
    pub simplefin_id: Option<String>,
    /// Count transfers INTO this account as spending (loan-style accounts where
    /// the payment is the expense's only footprint). Defaults on for mortgages.
    pub count_payments_as_expense: bool,
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
    /// "manual" for user-entered/txn-generated snapshots, "simplefin" for synced ones.
    pub source: String,
}

#[derive(Serialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct BalanceMonthSummary {
    pub month: String,        // "YYYY-MM"
    pub count: i64,
    pub latest_balance: f64,
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
    pub is_withdrawal: bool,
    pub import_source: String,
    pub generated_balance_id: Option<i32>,
    pub generated_balance_to_id: Option<i32>,
    pub paycheck_id: Option<i32>,
    /// Category supplied by the bank/bridge (SimpleFIN), separate from `category`.
    pub vendor_category: Option<String>,
    /// SimpleFIN transaction id, set only on synced transactions (dedup key).
    pub simplefin_id: Option<String>,
    /// Every distinct description field the bank sent, unedited — kept so the
    /// user can see the original text behind the import's cleanup (e.g. a
    /// stripped "LINK.COM*" processor wrapper). NULL on manual/CSV rows.
    pub raw_description: Option<String>,
    /// Rule-derived noise kind ('investment_activity' | 'fee' | 'interest').
    /// When set, the transaction is excluded from cash-flow analytics and
    /// hidden from the default transactions list; balance math still sees it.
    pub suppressed_as: Option<String>,
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
pub struct CategoryRule {
    pub id: i32,
    pub keyword: String,
    pub category: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct VendorRule {
    pub id: i32,
    pub keyword: String,
    pub vendor_name: String,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct SuppressRule {
    pub id: i32,
    pub keyword: String,
    /// 'investment_activity' | 'fee' | 'interest'
    pub kind: String,
    /// Scope the rule to one account; None applies to all accounts.
    pub account_id: Option<i32>,
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
pub struct AssetEvent {
    pub id: i32,
    pub account_id: Option<i32>,
    pub asset_label: Option<String>,
    pub date: String,
    pub description: String,
    pub kind: String,
    pub cost: f64,
    pub asset_value: Option<f64>,
    pub vendor: Option<String>,
    pub notes: Option<String>,
    pub life_expectancy: Option<String>,
    pub linked_transaction_id: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    pub has_attachment: bool,
}

#[derive(Serialize, Deserialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct AssetAttachment {
    pub id: i32,
    pub asset_event_id: i32,
    pub object_key: String,
    pub original_name: String,
    pub provider: String,
    pub byte_size: Option<i64>,
    pub created_at: String,
}

#[derive(Serialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct MigrationSummary {
    pub migrated: i64,
    pub failed: i64,
    pub failed_names: Vec<String>,
}

#[derive(Serialize, TS, Clone)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../src/lib/types/")]
pub struct StorageInfo {
    pub provider: String,
    pub bucket_name: Option<String>,
    pub r2_account_id: Option<String>,
    pub s3_region: Option<String>,
    pub local_path: String,
    pub has_credentials: bool,
    /// True when a cloud provider is configured (synced from another device) but no
    /// credentials exist in this device's keychain yet.
    pub needs_credentials: bool,
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
    pub income_account_id: Option<i32>,
    pub import_source: String,
    pub created_at: String,
    pub updated_at: String,
}
