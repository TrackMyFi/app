pub mod commands;
pub mod db;
pub mod migrations;
pub mod models;
pub mod sync;

use tauri::{Manager, RunEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init());

    // Self-update support (desktop only). `process` is needed to relaunch after install.
    #[cfg(desktop)]
    {
        builder = builder
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_process::init());
    }

    builder
        .setup(|app| {
            let handle = app.handle().clone();
            let db = tauri::async_runtime::block_on(db::init(&handle))
                .map_err(|e| Box::<dyn std::error::Error>::from(e))?;
            app.manage(db);

            // Seed sync status from the DB mode and manage shared sync state.
            let initial = if app.state::<db::Db>().is_synced() {
                sync::SyncStatus::synced_just_now()
            } else {
                sync::SyncStatus::local()
            };
            app.manage(sync::SyncShared {
                status: std::sync::Mutex::new(initial),
                lock: tokio::sync::Mutex::new(()),
            });
            app.manage(sync::RefreshGate::new());

            // Background sync (only meaningful in synced mode; the calls no-op otherwise).
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // Immediate startup catch-up: pull the cloud + migrate + refresh
                // the UI, off the critical path so the window already shows
                // last-synced local data instead of waiting on the network.
                if let Err(e) = sync::initial_catch_up(&handle).await {
                    eprintln!("warning: initial sync catch-up failed: {e}");
                }
                // Backstop pull for long-open sessions.
                let mut tick =
                    tokio::time::interval(std::time::Duration::from_secs(sync::SYNC_INTERVAL_SECS));
                tick.tick().await; // consume the immediate first tick
                loop {
                    tick.tick().await;
                    let _ = sync::do_sync(&handle).await;
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::fire_profile::get_fire_profile,
            commands::fire_profile::upsert_fire_profile,
            commands::fire_profile::mark_onboarding_complete,
            commands::data_management::preview_data_deletion,
            commands::data_management::delete_data,
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
            commands::accounts::list_latest_balances_cmd,
            commands::accounts::list_balance_month_summaries_cmd,
            commands::accounts::list_balances_for_month_cmd,
            commands::transactions::list_transactions_cmd,
            commands::transactions::get_transaction_cmd,
            commands::transactions::create_transaction_cmd,
            commands::transactions::update_transaction_cmd,
            commands::transactions::delete_transaction_cmd,
            commands::transactions::bulk_create_transactions_cmd,
            commands::transactions::bulk_create_transactions_with_snapshots_cmd,
            commands::transactions::rebuild_account_balances_cmd,
            commands::import_mappings::list_import_mappings_cmd,
            commands::import_mappings::create_import_mapping_cmd,
            commands::import_mappings::update_import_mapping_cmd,
            commands::import_mappings::delete_import_mapping_cmd,
            commands::category_rules::list_category_rules_cmd,
            commands::category_rules::create_category_rule_cmd,
            commands::category_rules::delete_category_rule_cmd,
            commands::paychecks::list_paychecks_cmd,
            commands::paychecks::get_paycheck_cmd,
            commands::paychecks::create_paycheck_cmd,
            commands::paychecks::update_paycheck_cmd,
            commands::paychecks::delete_paycheck_cmd,
            commands::asset_events::list_asset_events_cmd,
            commands::asset_events::get_asset_event_cmd,
            commands::asset_events::create_asset_event_cmd,
            commands::asset_events::update_asset_event_cmd,
            commands::asset_events::delete_asset_event_cmd,
            commands::contributions::list_contribution_txns_cmd,
            commands::contributions::list_contribution_years_cmd,
            commands::budget::list_budget_months_cmd,
            commands::budget::list_budget_txns_cmd,
            commands::budget::get_budget_month_target_cmd,
            commands::budget::set_budget_month_target_cmd,
            commands::budget::get_budget_paycheck_summary_cmd,
            sync::get_sync_status,
            sync::sync_now,
            sync::save_sync_config,
            sync::clear_sync_config,
            sync::restart_app,
            sync::frontend_ready,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|handle, event| {
            if let RunEvent::ExitRequested { .. } = event {
                // Best-effort final push so another device sees this session's edits.
                // Bounded so a slow/unreachable Turso can't block app quit.
                let _ = tauri::async_runtime::block_on(async {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(10),
                        sync::do_sync(handle),
                    )
                    .await
                });
            }
        });
}
