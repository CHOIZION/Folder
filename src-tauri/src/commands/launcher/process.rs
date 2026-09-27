use std::{
    path::Path,
    process::Command,
    sync::{Mutex, OnceLock},
};

static ACTIVE_PROCESS_ID: OnceLock<Mutex<Option<u32>>> = OnceLock::new();

pub(super) fn spawn(
    executable: &Path,
    working_directory: &Path,
    context: &str,
) -> Result<(), String> {
    let child = Command::new(executable)
        .current_dir(working_directory)
        .spawn()
        .map_err(|error| format!("{context}: {error}"))?;
    remember(child.id());
    Ok(())
}

pub(super) fn spawn_with_argument(
    executable: &Path,
    argument: &Path,
    working_directory: &Path,
    context: &str,
) -> Result<(), String> {
    let child = Command::new(executable)
        .arg(argument)
        .current_dir(working_directory)
        .spawn()
        .map_err(|error| format!("{context}: {error}"))?;
    remember(child.id());
    Ok(())
}

pub(super) fn open_with_default_app(path: &Path) -> Result<(), String> {
    let escaped = path.to_string_lossy().replace('\'', "''");
    let script = format!("$p=Start-Process -FilePath '{escaped}' -PassThru; $p.Id");
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .output()
        .map_err(|error| format!("기본 프로그램으로 열기에 실패했습니다: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    if let Ok(process_id) = String::from_utf8_lossy(&output.stdout).trim().parse() {
        remember(process_id);
    }
    Ok(())
}

pub(super) fn stop() -> Result<(), String> {
    let process_id = active_process_id()
        .lock()
        .map_err(|_| "실행 프로세스 잠금에 실패했습니다.".to_owned())?
        .take();
    let Some(process_id) = process_id else {
        return Ok(());
    };

    let status = Command::new("taskkill")
        .args(["/PID", &process_id.to_string(), "/T", "/F"])
        .status()
        .map_err(|error| format!("현재 작품을 종료하지 못했습니다: {error}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| "현재 작품이 이미 종료되었거나 종료할 수 없습니다.".to_owned())
}

fn active_process_id() -> &'static Mutex<Option<u32>> {
    ACTIVE_PROCESS_ID.get_or_init(|| Mutex::new(None))
}

fn remember(id: u32) {
    if let Ok(mut current) = active_process_id().lock() {
        *current = Some(id);
    }
}
