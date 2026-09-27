use std::sync::atomic::{AtomicBool, Ordering};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::{db::DbState, tray};

const MINIMIZE_TO_TRAY_KEY: &str = "minimize_to_tray";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub minimize_to_tray: bool,
}

pub struct PreferencesState {
    minimize_to_tray: AtomicBool,
    exit_requested: AtomicBool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseBehavior {
    Exit,
    HideToTray,
}

impl PreferencesState {
    pub fn load(connection: &Connection) -> Result<Self, String> {
        Ok(Self {
            minimize_to_tray: AtomicBool::new(load_bool(connection, MINIMIZE_TO_TRAY_KEY, false)?),
            exit_requested: AtomicBool::new(false),
        })
    }

    pub fn snapshot(&self) -> AppPreferences {
        AppPreferences {
            minimize_to_tray: self.minimize_to_tray.load(Ordering::Acquire),
        }
    }

    pub fn close_behavior(&self) -> CloseBehavior {
        if self.minimize_to_tray.load(Ordering::Acquire)
            && !self.exit_requested.load(Ordering::Acquire)
        {
            CloseBehavior::HideToTray
        } else {
            CloseBehavior::Exit
        }
    }

    pub fn request_exit(&self) {
        self.exit_requested.store(true, Ordering::Release);
    }
}

#[tauri::command]
pub fn get_app_preferences(state: State<'_, PreferencesState>) -> AppPreferences {
    state.snapshot()
}

#[tauri::command]
pub fn set_minimize_to_tray(
    enabled: bool,
    app: AppHandle,
    preferences: State<'_, PreferencesState>,
    database: State<'_, DbState>,
) -> Result<AppPreferences, String> {
    let previous = preferences.snapshot().minimize_to_tray;
    if previous == enabled {
        return Ok(preferences.snapshot());
    }

    {
        let connection = database
            .connection
            .lock()
            .map_err(|_| "데이터베이스 잠금에 실패했습니다.".to_string())?;
        save_bool(&connection, MINIMIZE_TO_TRAY_KEY, enabled)?;
    }
    preferences
        .minimize_to_tray
        .store(enabled, Ordering::Release);

    if let Err(error) = tray::sync_tray(&app, enabled) {
        preferences
            .minimize_to_tray
            .store(previous, Ordering::Release);
        if let Ok(connection) = database.connection.lock() {
            let _ = save_bool(&connection, MINIMIZE_TO_TRAY_KEY, previous);
        }
        return Err(error);
    }

    Ok(preferences.snapshot())
}

fn load_bool(connection: &Connection, key: &str, default: bool) -> Result<bool, String> {
    let value = connection
        .query_row(
            "SELECT value FROM app_settings WHERE key = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|error| format!("설정을 읽지 못했습니다: {error}"))?;
    Ok(value.map_or(default, |value| value == "1"))
}

fn save_bool(connection: &Connection, key: &str, value: bool) -> Result<(), String> {
    connection
        .execute(
            r#"
            INSERT INTO app_settings(key, value) VALUES (?1, ?2)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            "#,
            params![key, if value { "1" } else { "0" }],
        )
        .map_err(|error| format!("설정을 저장하지 못했습니다: {error}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::create_schema;

    #[test]
    fn tray_preference_defaults_off_and_round_trips() {
        let connection = Connection::open_in_memory().expect("open database");
        create_schema(&connection).expect("create schema");

        assert!(
            !PreferencesState::load(&connection)
                .expect("load defaults")
                .snapshot()
                .minimize_to_tray
        );
        save_bool(&connection, MINIMIZE_TO_TRAY_KEY, true).expect("save preference");
        assert!(
            PreferencesState::load(&connection)
                .expect("load saved preference")
                .snapshot()
                .minimize_to_tray
        );
        let state = PreferencesState::load(&connection).expect("load enabled state");
        assert_eq!(state.close_behavior(), CloseBehavior::HideToTray);
        state.request_exit();
        assert_eq!(state.close_behavior(), CloseBehavior::Exit);
    }
}
