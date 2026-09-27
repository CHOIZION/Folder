use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use rusqlite::Connection;
use tauri::{AppHandle, Manager};

pub mod repository;
pub mod schema;

pub struct DbState {
    pub connection: Arc<Mutex<Connection>>,
    pub path: PathBuf,
}

pub fn initialize(app: &AppHandle) -> Result<DbState, String> {
    let app_data_dir = match std::env::var_os("HEART_DATA_DIR") {
        Some(path) => PathBuf::from(path),
        None => app
            .path()
            .app_data_dir()
            .map_err(|error| format!("앱 데이터 경로를 찾지 못했습니다: {error}"))?,
    };

    std::fs::create_dir_all(&app_data_dir)
        .map_err(|error| format!("앱 데이터 폴더를 만들지 못했습니다: {error}"))?;

    let database_path = app_data_dir.join("heart.db");
    let connection = Connection::open(&database_path)
        .map_err(|error| format!("데이터베이스를 열지 못했습니다: {error}"))?;

    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             PRAGMA journal_mode = WAL;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;
             PRAGMA busy_timeout = 5000;",
        )
        .map_err(|error| format!("SQLite 설정에 실패했습니다: {error}"))?;

    schema::create_schema(&connection)?;

    Ok(DbState {
        connection: Arc::new(Mutex::new(connection)),
        path: database_path,
    })
}
