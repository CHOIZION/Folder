use tauri::{AppHandle, State};

use crate::{
    core::scanner::scan_library,
    db::{
        repository::{self, LibrarySnapshot, MetadataUpdate},
        DbState,
    },
    library_watcher::{LibraryWatcher, ScanCoordinator},
};

#[tauri::command]
pub fn scan_folder(
    path: String,
    state: State<'_, DbState>,
    coordinator: State<'_, ScanCoordinator>,
) -> Result<LibrarySnapshot, String> {
    let _scan_guard = coordinator
        .lock()
        .map_err(|_| "스캔 작업 잠금에 실패했습니다.".to_string())?;
    let scan_result = scan_library(&path)?;
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::save_scan(&mut connection, &scan_result)
}

#[tauri::command]
pub fn watch_library(
    path: String,
    app: AppHandle,
    database: State<'_, DbState>,
    watcher: State<'_, LibraryWatcher>,
    coordinator: State<'_, ScanCoordinator>,
) -> Result<(), String> {
    watcher.watch(
        path.into(),
        database.connection.clone(),
        coordinator.shared(),
        app,
    )
}

#[tauri::command]
pub fn load_library(path: String, state: State<'_, DbState>) -> Result<LibrarySnapshot, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::load_snapshot(&connection, &path)
}

#[tauri::command]
pub fn update_item_metadata(
    root_path: String,
    update: MetadataUpdate,
    state: State<'_, DbState>,
) -> Result<LibrarySnapshot, String> {
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::update_metadata(&mut connection, &root_path, update)
}

#[tauri::command]
pub fn record_item_open(
    root_path: String,
    item_path: String,
    state: State<'_, DbState>,
) -> Result<LibrarySnapshot, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::record_open(&connection, &root_path, &item_path)
}

#[tauri::command]
pub fn create_user_category(
    root_path: String,
    name: String,
    state: State<'_, DbState>,
) -> Result<LibrarySnapshot, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::create_user_category(&connection, &root_path, &name)
}

#[tauri::command]
pub fn delete_user_category(
    root_path: String,
    category_id: i64,
    state: State<'_, DbState>,
) -> Result<LibrarySnapshot, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::delete_user_category(&connection, &root_path, category_id)
}

#[tauri::command]
pub fn set_item_user_categories(
    root_path: String,
    item_path: String,
    category_ids: Vec<i64>,
    state: State<'_, DbState>,
) -> Result<LibrarySnapshot, String> {
    let mut connection = state
        .connection
        .lock()
        .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;

    repository::set_item_user_categories(&mut connection, &root_path, &item_path, category_ids)
}

#[tauri::command]
pub fn get_database_path(state: State<'_, DbState>) -> String {
    state.path.to_string_lossy().to_string()
}
