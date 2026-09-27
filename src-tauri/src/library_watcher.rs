use std::{
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc::{self, RecvTimeoutError, TrySendError},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use rusqlite::Connection;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::{
    core::scanner::scan_library,
    db::repository::{self, LibrarySnapshot},
};

const CHANGE_DEBOUNCE: Duration = Duration::from_secs(2);

#[derive(Default)]
pub struct ScanCoordinator(Arc<Mutex<()>>);

impl ScanCoordinator {
    pub fn lock(&self) -> Result<MutexGuard<'_, ()>, std::sync::PoisonError<MutexGuard<'_, ()>>> {
        self.0.lock()
    }

    pub fn shared(&self) -> Arc<Mutex<()>> {
        self.0.clone()
    }
}

#[derive(Default)]
pub struct LibraryWatcher {
    session: Mutex<Option<WatchSession>>,
}

impl LibraryWatcher {
    pub fn watch(
        &self,
        root_path: PathBuf,
        connection: Arc<Mutex<Connection>>,
        scan_coordinator: Arc<Mutex<()>>,
        app: AppHandle,
    ) -> Result<(), String> {
        if !root_path.is_dir() {
            return Err(format!(
                "실시간 감시할 라이브러리 폴더가 없습니다: {}",
                root_path.display()
            ));
        }

        let mut session = self
            .session
            .lock()
            .map_err(|_| "라이브러리 감시 상태 잠금에 실패했습니다.".to_string())?;
        if session
            .as_ref()
            .is_some_and(|current| current.root_path == root_path)
        {
            return Ok(());
        }

        let next = WatchSession::start(root_path, connection, scan_coordinator, app)?;
        *session = Some(next);
        Ok(())
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct LibraryUpdated {
    root_path: String,
    total_items: usize,
}

struct WatchSession {
    root_path: PathBuf,
    shutdown: Arc<AtomicBool>,
    directory_handle: Arc<AtomicUsize>,
}

impl WatchSession {
    fn start(
        root_path: PathBuf,
        connection: Arc<Mutex<Connection>>,
        scan_coordinator: Arc<Mutex<()>>,
        app: AppHandle,
    ) -> Result<Self, String> {
        let directory_handle = platform::open_directory(&root_path)?;
        let handle = Arc::new(AtomicUsize::new(directory_handle));
        let shutdown = Arc::new(AtomicBool::new(false));
        let (change_sender, change_receiver) = mpsc::sync_channel(1);
        let session = Self {
            root_path: root_path.clone(),
            shutdown: shutdown.clone(),
            directory_handle: handle.clone(),
        };

        let watch_handle = handle.clone();
        let watch_shutdown = shutdown.clone();
        let watch_app = app.clone();
        thread::Builder::new()
            .name("heart-library-events".to_string())
            .spawn(move || {
                while !watch_shutdown.load(Ordering::Acquire) {
                    match platform::wait_for_change(watch_handle.load(Ordering::Acquire)) {
                        Ok(()) => match change_sender.try_send(()) {
                            Ok(()) | Err(TrySendError::Full(())) => {}
                            Err(TrySendError::Disconnected(())) => break,
                        },
                        Err(error) => {
                            if !watch_shutdown.load(Ordering::Acquire) {
                                let _ = watch_app.emit("library-watch-error", error);
                            }
                            break;
                        }
                    }
                }
            })
            .map_err(|error| format!("라이브러리 감시 스레드를 시작하지 못했습니다: {error}"))?;

        let scan_root = root_path.clone();
        let scan_shutdown = shutdown.clone();
        thread::Builder::new()
            .name("heart-library-refresh".to_string())
            .spawn(move || {
                while change_receiver.recv().is_ok() {
                    if scan_shutdown.load(Ordering::Acquire) {
                        break;
                    }
                    loop {
                        match change_receiver.recv_timeout(CHANGE_DEBOUNCE) {
                            Ok(()) => continue,
                            Err(RecvTimeoutError::Timeout) => break,
                            Err(RecvTimeoutError::Disconnected) => return,
                        }
                    }
                    if scan_shutdown.load(Ordering::Acquire) {
                        break;
                    }
                    if let Err(error) =
                        refresh_library(&scan_root, &connection, &scan_coordinator, &app)
                    {
                        let _ = app.emit("library-watch-error", error);
                    }
                }
            })
            .map_err(|error| format!("자동 스캔 스레드를 시작하지 못했습니다: {error}"))?;

        Ok(session)
    }
}

impl Drop for WatchSession {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let handle = self.directory_handle.swap(0, Ordering::AcqRel);
        if handle != 0 {
            platform::close_directory(handle);
        }
    }
}

fn refresh_library(
    root_path: &Path,
    connection: &Arc<Mutex<Connection>>,
    scan_coordinator: &Arc<Mutex<()>>,
    app: &AppHandle,
) -> Result<(), String> {
    let snapshot = scan_and_save(root_path, connection, scan_coordinator)?;
    let path = root_path.to_string_lossy().into_owned();
    app.emit(
        "library-updated",
        LibraryUpdated {
            root_path: path,
            total_items: snapshot.scan.total_items,
        },
    )
    .map_err(|error| format!("라이브러리 갱신 알림을 보내지 못했습니다: {error}"))?;
    Ok(())
}

fn scan_and_save(
    root_path: &Path,
    connection: &Arc<Mutex<Connection>>,
    scan_coordinator: &Arc<Mutex<()>>,
) -> Result<LibrarySnapshot, String> {
    let _scan_guard = scan_coordinator
        .lock()
        .map_err(|_| "자동 스캔 작업 잠금에 실패했습니다.".to_string())?;
    let path = root_path.to_string_lossy().into_owned();
    let scan = scan_library(&path)?;
    let mut connection = connection
        .lock()
        .map_err(|_| "자동 스캔 DB 잠금에 실패했습니다.".to_string())?;
    repository::save_scan(&mut connection, &scan)
}

#[cfg(target_os = "windows")]
mod platform {
    use std::{ffi::c_void, os::windows::ffi::OsStrExt, path::Path};

    type Handle = *mut c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn CreateFileW(
            file_name: *const u16,
            desired_access: u32,
            share_mode: u32,
            security_attributes: *mut c_void,
            creation_disposition: u32,
            flags_and_attributes: u32,
            template_file: Handle,
        ) -> Handle;
        fn ReadDirectoryChangesW(
            directory: Handle,
            buffer: *mut c_void,
            buffer_length: u32,
            watch_subtree: i32,
            notify_filter: u32,
            bytes_returned: *mut u32,
            overlapped: *mut c_void,
            completion_routine: *mut c_void,
        ) -> i32;
        fn CloseHandle(object: Handle) -> i32;
        fn GetLastError() -> u32;
    }

    const FILE_LIST_DIRECTORY: u32 = 0x0001;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const FILE_SHARE_DELETE: u32 = 0x0000_0004;
    const OPEN_EXISTING: u32 = 3;
    const FILE_FLAG_BACKUP_SEMANTICS: u32 = 0x0200_0000;
    const FILE_NOTIFY_CHANGE_FILE_NAME: u32 = 0x0000_0001;
    const FILE_NOTIFY_CHANGE_DIR_NAME: u32 = 0x0000_0002;
    const FILE_NOTIFY_CHANGE_SIZE: u32 = 0x0000_0008;
    const FILE_NOTIFY_CHANGE_LAST_WRITE: u32 = 0x0000_0010;
    const FILE_NOTIFY_CHANGE_CREATION: u32 = 0x0000_0040;
    const INVALID_HANDLE_VALUE: Handle = -1_isize as Handle;

    pub fn open_directory(path: &Path) -> Result<usize, String> {
        let wide_path = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        let handle = unsafe {
            CreateFileW(
                wide_path.as_ptr(),
                FILE_LIST_DIRECTORY,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(format!(
                "라이브러리 실시간 감시를 시작하지 못했습니다: {} (Windows 오류 {})",
                path.display(),
                unsafe { GetLastError() }
            ));
        }
        Ok(handle as usize)
    }

    pub fn wait_for_change(handle: usize) -> Result<(), String> {
        if handle == 0 {
            return Err("라이브러리 감시가 종료되었습니다.".to_string());
        }
        let mut buffer = [0_u8; 64 * 1024];
        let mut bytes_returned = 0_u32;
        let result = unsafe {
            ReadDirectoryChangesW(
                handle as Handle,
                buffer.as_mut_ptr().cast(),
                buffer.len() as u32,
                1,
                FILE_NOTIFY_CHANGE_FILE_NAME
                    | FILE_NOTIFY_CHANGE_DIR_NAME
                    | FILE_NOTIFY_CHANGE_SIZE
                    | FILE_NOTIFY_CHANGE_LAST_WRITE
                    | FILE_NOTIFY_CHANGE_CREATION,
                &mut bytes_returned,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if result == 0 {
            Err(format!(
                "라이브러리 변경 감지가 중단되었습니다. (Windows 오류 {})",
                unsafe { GetLastError() }
            ))
        } else {
            Ok(())
        }
    }

    pub fn close_directory(handle: usize) {
        unsafe {
            CloseHandle(handle as Handle);
        }
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::{platform, scan_and_save};
    use crate::db::schema::create_schema;
    use rusqlite::Connection;
    use std::{
        fs,
        sync::{mpsc, Arc, Mutex},
        thread,
        time::Duration,
    };

    fn unique_test_directory(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "heart-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock")
                .as_nanos()
        ))
    }

    #[test]
    fn native_watcher_reports_a_file_change() {
        let directory = unique_test_directory("watch-test");
        fs::create_dir(&directory).expect("create test directory");
        let handle = platform::open_directory(&directory).expect("watch test directory");
        let (sender, receiver) = mpsc::channel();
        let wait = thread::spawn(move || {
            sender
                .send(platform::wait_for_change(handle))
                .expect("send watch result");
            platform::close_directory(handle);
        });

        thread::sleep(Duration::from_millis(50));
        let changed_file = directory.join("new-item.txt");
        fs::write(&changed_file, b"changed").expect("create changed file");
        receiver
            .recv_timeout(Duration::from_secs(3))
            .expect("receive filesystem event")
            .expect("valid filesystem event");
        wait.join().expect("join watcher");

        fs::remove_file(changed_file).expect("remove changed file");
        fs::remove_dir(directory).expect("remove test directory");
    }

    #[test]
    fn detected_change_pipeline_scans_and_persists_the_library() {
        let root = unique_test_directory("refresh-test");
        let category = root.join("영상");
        let item = category.join("Sample");
        fs::create_dir_all(&item).expect("create sample library");
        let video = item.join("sample.mp4");
        fs::write(&video, b"sample").expect("create sample video");

        let connection = Arc::new(Mutex::new(
            Connection::open_in_memory().expect("open database"),
        ));
        create_schema(&connection.lock().expect("lock database")).expect("create schema");
        let snapshot = scan_and_save(&root, &connection, &Arc::new(Mutex::new(())))
            .expect("scan and save changed library");

        assert_eq!(snapshot.scan.total_items, 1);
        assert_eq!(snapshot.scan.categories[0].items[0].title, "Sample");

        fs::remove_file(video).expect("remove sample video");
        fs::remove_dir(item).expect("remove sample item");
        fs::remove_dir(category).expect("remove sample category");
        fs::remove_dir(root).expect("remove sample root");
    }
}

#[cfg(not(target_os = "windows"))]
mod platform {
    use std::path::Path;

    pub fn open_directory(path: &Path) -> Result<usize, String> {
        Err(format!(
            "라이브러리 실시간 감시는 현재 Windows에서만 지원합니다: {}",
            path.display()
        ))
    }

    pub fn wait_for_change(_: usize) -> Result<(), String> {
        Err("라이브러리 실시간 감시는 현재 Windows에서만 지원합니다.".to_string())
    }

    pub fn close_directory(_: usize) {}
}
