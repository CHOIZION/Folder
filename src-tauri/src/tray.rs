use tauri::{
    menu::MenuBuilder,
    tray::{TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::preferences::PreferencesState;

const TRAY_ID: &str = "heart-main-tray";
const SHOW_ID: &str = "heart-show";
const QUIT_ID: &str = "heart-quit";

pub fn sync_tray(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if enabled {
        ensure_tray(app)
    } else {
        app.remove_tray_by_id(TRAY_ID);
        Ok(())
    }
}

pub fn handle_menu_event(app: &AppHandle, event_id: &str) {
    match event_id {
        SHOW_ID => show_main_window(app),
        QUIT_ID => quit(app),
        _ => {}
    }
}

pub fn handle_icon_event(app: &AppHandle, event: TrayIconEvent) {
    if event.id() == TRAY_ID && matches!(event, TrayIconEvent::DoubleClick { .. }) {
        show_main_window(app);
    }
}

fn ensure_tray(app: &AppHandle) -> Result<(), String> {
    if app.tray_by_id(TRAY_ID).is_some() {
        return Ok(());
    }

    let menu = MenuBuilder::new(app)
        .text(SHOW_ID, "HEART 열기")
        .separator()
        .text(QUIT_ID, "완전히 종료")
        .build()
        .map_err(|error| format!("트레이 메뉴를 만들지 못했습니다: {error}"))?;
    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .menu(&menu)
        .tooltip("HEART")
        .show_menu_on_left_click(true);
    if let Some(icon) = app.default_window_icon().cloned() {
        builder = builder.icon(icon);
    }
    builder
        .build(app)
        .map_err(|error| format!("트레이 아이콘을 만들지 못했습니다: {error}"))?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
}

fn quit(app: &AppHandle) {
    if let Some(preferences) = app.try_state::<PreferencesState>() {
        preferences.request_exit();
    }
    app.exit(0);
}
