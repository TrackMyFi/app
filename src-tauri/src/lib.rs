pub mod commands;
pub mod db;
pub mod migrations;
pub mod models;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let db = tauri::async_runtime::block_on(db::init(&handle))
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fire_profile::get_fire_profile,
            commands::fire_profile::upsert_fire_profile,
            commands::accounts::list_accounts_cmd,
            commands::accounts::create_account_cmd,
            commands::accounts::archive_account_cmd,
            commands::accounts::unarchive_account_cmd,
            commands::accounts::delete_account_cmd,
            commands::accounts::add_balance_cmd,
            commands::accounts::update_account_cmd,
            commands::accounts::update_balance_cmd,
            commands::accounts::delete_balance_cmd,
            commands::accounts::list_all_balances_cmd,
            commands::transactions::list_transactions_cmd,
            commands::transactions::get_transaction_cmd,
            commands::transactions::create_transaction_cmd,
            commands::transactions::update_transaction_cmd,
            commands::transactions::delete_transaction_cmd,
            commands::transactions::bulk_create_transactions_cmd,
            commands::import_mappings::list_import_mappings_cmd,
            commands::import_mappings::create_import_mapping_cmd,
            commands::import_mappings::delete_import_mapping_cmd,
            commands::paychecks::list_paychecks_cmd,
            commands::paychecks::get_paycheck_cmd,
            commands::paychecks::create_paycheck_cmd,
            commands::paychecks::update_paycheck_cmd,
            commands::paychecks::delete_paycheck_cmd,
            commands::contributions::list_contribution_txns_cmd,
            commands::contributions::list_contribution_years_cmd,
            commands::budget::list_budget_months_cmd,
            commands::budget::list_budget_txns_cmd,
            commands::budget::get_budget_month_target_cmd,
            commands::budget::set_budget_month_target_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
