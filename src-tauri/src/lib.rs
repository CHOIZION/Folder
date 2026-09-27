mod commands;
mod core;
mod db;
mod desktop;
mod h264;
mod library_watcher;
mod preferences;
mod remote;
mod tray;

use commands::launcher::{launch_item, launch_media_file, stop_launched_item};
use commands::scan_folder::{
    create_user_category, delete_user_category, get_database_path, load_library, record_item_open,
    scan_folder, set_item_user_categories, update_item_metadata, watch_library,
};
use library_watcher::{LibraryWatcher, ScanCoordinator};
use preferences::{get_app_preferences, set_minimize_to_tray, CloseBehavior, PreferencesState};
use remote::{get_remote_host_status, reset_remote_pairing};
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .on_menu_event(|app, event| tray::handle_menu_event(app, event.id().as_ref()))
        .on_tray_icon_event(tray::handle_icon_event)
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let preferences = window.state::<PreferencesState>();
                match preferences.close_behavior() {
                    CloseBehavior::HideToTray => {
                        api.prevent_close();
                        let _ = window.hide();
                    }
                    CloseBehavior::Exit => {
                        preferences.request_exit();
                        window.app_handle().exit(0);
                    }
                }
            }
        })
        .setup(|app| {
            if let Some(main_window) = app.get_webview_window("main") {
                main_window.show().map_err(std::io::Error::other)?;
                let _ = main_window.set_focus();
            }

            let state = db::initialize(app.handle()).map_err(std::io::Error::other)?;
            let preferences = {
                let connection = state
                    .connection
                    .lock()
                    .map_err(|_| std::io::Error::other("데이터베이스 잠금에 실패했습니다."))?;
                PreferencesState::load(&connection).map_err(std::io::Error::other)?
            };
            let minimize_to_tray = preferences.snapshot().minimize_to_tray;
            let database_path = state.path.clone();
            app.manage(state);
            app.manage(preferences);
            app.manage(ScanCoordinator::default());
            app.manage(LibraryWatcher::default());
            tray::sync_tray(app.handle(), minimize_to_tray).map_err(std::io::Error::other)?;

            let remote_state = match remote::start(database_path) {
                Ok(state) => state,
                Err(error) => {
                    eprintln!("HEART Remote: {error}");
                    remote::unavailable(error)
                }
            };
            app.manage(remote_state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_folder,
            watch_library,
            load_library,
            update_item_metadata,
            record_item_open,
            create_user_category,
            delete_user_category,
            set_item_user_categories,
            get_database_path,
            get_app_preferences,
            set_minimize_to_tray,
            launch_item,
            launch_media_file,
            stop_launched_item,
            get_remote_host_status,
            reset_remote_pairing,
        ])
        .run(tauri::generate_context!())
        .expect("HEART 실행 중 오류가 발생했습니다.");
}
