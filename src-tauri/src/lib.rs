pub mod commands;
pub mod db;
pub mod migrations;
pub mod models;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
            commands::accounts::list_all_balances_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
