pub mod commands;
pub mod db;
pub mod migrations;
pub mod models;
pub mod simplefin;
pub mod storage;
pub mod sync;

use tauri::{LogicalSize, Manager, RunEvent};

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
            .plugin(tauri_plugin_process::init())
            // Remember the window's size & position across launches. We only
            // persist SIZE | POSITION (not maximized/fullscreen) so reopening
            // restores exactly what the user last left, never a surprise
            // fullscreen state.
            .plugin(
                tauri_plugin_window_state::Builder::default()
                    .with_state_flags(
                        tauri_plugin_window_state::StateFlags::SIZE
                            | tauri_plugin_window_state::StateFlags::POSITION,
                    )
                    .build(),
            );
    }

    builder
        .setup(|app| {
            // First-launch window sizing. The window-state plugin restores the
            // user's last size & position on every subsequent launch, but on a
            // fresh install (no saved state yet) we pick sensible defaults:
            // fill the screen on laptops/small displays, but cap the size on
            // large external displays (e.g. a 27" 5K) so the app doesn't sprawl.
            #[cfg(desktop)]
            {
                const MAX_W: f64 = 1760.0;
                const MAX_H: f64 = 1120.0;
                let has_saved_state = app
                    .path()
                    .app_config_dir()
                    .map(|dir| dir.join(".window-state.json").exists())
                    .unwrap_or(false);
                if !has_saved_state {
                    if let Some(win) = app.get_webview_window("main") {
                        if let Ok(Some(monitor)) = win.current_monitor() {
                            let scale = monitor.scale_factor();
                            let mon = monitor.size().to_logical::<f64>(scale);
                            if mon.width <= MAX_W && mon.height <= MAX_H {
                                // Laptop / small display: fill the available space.
                                let _ = win.maximize();
                            } else {
                                // Large external display: open at a comfortable,
                                // capped size, centered.
                                let _ = win.set_size(LogicalSize::new(MAX_W, MAX_H));
                                let _ = win.center();
                            }
                        }
                    }
                }
            }

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
            app.manage(sync::SyncShared { status: std::sync::Mutex::new(initial) });
            // One gate serializing everything that syncs against the DB file:
            // Turso pulls/pushes and SimpleFIN imports queue instead of overlapping.
            app.manage(sync::DbGate::new());
            app.manage(sync::RefreshGate::new());
            app.manage(simplefin::SimpleFinShared::new());

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

            // SimpleFIN daily bank sync. `maybe_sync` no-ops unless connected
            // AND due (24h after the last success, 6h backoff after failures),
            // so ticking every 30 minutes stays well within SimpleFIN's
            // ~once-a-day polling guidance while catching up promptly after
            // the app has been closed for days.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut tick = tokio::time::interval(std::time::Duration::from_secs(
                    simplefin::SCHEDULER_TICK_SECS,
                ));
                loop {
                    tick.tick().await; // first tick fires immediately (startup catch-up)
                    simplefin::maybe_sync(&handle).await;
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
            commands::transactions::period_stats_cmd,
            commands::transactions::get_transaction_cmd,
            commands::transactions::create_transaction_cmd,
            commands::transactions::update_transaction_cmd,
            commands::transactions::delete_transaction_cmd,
            commands::transactions::delete_transaction_keep_snapshot_cmd,
            commands::transactions::bulk_create_transactions_cmd,
            commands::transactions::bulk_create_transactions_with_snapshots_cmd,
            commands::transactions::rebuild_account_balances_cmd,
            commands::import_mappings::list_import_mappings_cmd,
            commands::import_mappings::create_import_mapping_cmd,
            commands::import_mappings::update_import_mapping_cmd,
            commands::import_mappings::delete_import_mapping_cmd,
            commands::category_rules::list_category_rules_cmd,
            commands::category_rules::create_category_rule_cmd,
            commands::category_rules::update_category_rule_cmd,
            commands::category_rules::delete_category_rule_cmd,
            commands::vendor_rules::list_vendor_rules_cmd,
            commands::vendor_rules::create_vendor_rule_cmd,
            commands::vendor_rules::update_vendor_rule_cmd,
            commands::vendor_rules::delete_vendor_rule_cmd,
            commands::suppress_rules::list_suppress_rules_cmd,
            commands::suppress_rules::create_suppress_rule_cmd,
            commands::suppress_rules::update_suppress_rule_cmd,
            commands::suppress_rules::delete_suppress_rule_cmd,
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
            commands::storage::get_storage_config_cmd,
            commands::storage::save_storage_config_cmd,
            commands::storage::clear_storage_config_cmd,
            commands::storage::count_migratable_attachments_cmd,
            commands::storage::migrate_and_save_storage_config_cmd,
            commands::storage::list_attachments_cmd,
            commands::storage::upload_attachment_cmd,
            commands::storage::delete_attachment_cmd,
            commands::storage::open_attachment_cmd,
            commands::contributions::list_contribution_txns_cmd,
            commands::contributions::list_contribution_years_cmd,
            commands::budget::list_budget_months_cmd,
            commands::budget::list_budget_txns_cmd,
            commands::budget::get_budget_month_target_cmd,
            commands::budget::set_budget_month_target_cmd,
            commands::budget::get_budget_paycheck_summary_cmd,
            simplefin::simplefin_get_status,
            simplefin::simplefin_connect,
            simplefin::simplefin_link_account,
            simplefin::simplefin_sync_now,
            simplefin::simplefin_duplicate_candidates,
            simplefin::simplefin_disconnect,
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
