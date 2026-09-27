use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream, UdpSocket},
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex, RwLock},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use tauri::State;

use crate::{
    commands::launcher::{launch_item, stop_launched_item},
    core::scanner::{CategoryNode, ScannedItem},
    db::repository::{self, LibrarySnapshot},
    desktop,
};

mod streaming;

const DEFAULT_PORT: u16 = 37_218;
const LAST_PORT: u16 = 37_228;
const DISCOVERY_PORT: u16 = 37_219;
const DISCOVERY_REQUEST: &[u8] = b"HEART_DISCOVER_V1";
const MAX_REQUEST_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub struct RemoteHostState {
    inner: Arc<Mutex<RemoteHostInner>>,
    database_path: Option<PathBuf>,
    capture_guard: Arc<Mutex<()>>,
    thumbnail_paths: Arc<RwLock<HashMap<String, Option<String>>>>,
    tailscale_address: Option<String>,
}

struct RemoteHostInner {
    address: String,
    port: u16,
    pairing_code: String,
    auth_token: Option<String>,
    paired_device: Option<String>,
    last_seen: Option<Instant>,
    last_seen_at: Option<u64>,
    failed_pair_attempts: u8,
    pairing_locked_until: Option<Instant>,
    error_message: Option<String>,
    h264_encoder: Option<String>,
    h264_hardware_accelerated: Option<bool>,
    h264_error: Option<String>,
    h264_width: Option<u32>,
    h264_height: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteHostStatus {
    pub address: String,
    pub port: u16,
    pub connection_url: String,
    pub tailscale_connection_url: Option<String>,
    pub pairing_code: String,
    pub paired_device: Option<String>,
    pub connected: bool,
    pub last_seen_at: Option<u64>,
    pub error_message: Option<String>,
    pub h264_encoder: Option<String>,
    pub h264_hardware_accelerated: Option<bool>,
    pub h264_error: Option<String>,
    pub h264_width: Option<u32>,
    pub h264_height: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequest {
    code: String,
    device_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicStatusResponse {
    service: &'static str,
    version: &'static str,
    host_name: String,
    paired: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PairResponse {
    ok: bool,
    service: &'static str,
    host_name: String,
    device_name: String,
    token: String,
    message: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HeartbeatResponse {
    ok: bool,
    service: &'static str,
    host_name: String,
    server_time: u64,
    message: &'static str,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchRequest {
    item_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputRequest {
    action: String,
    x: Option<f64>,
    y: Option<f64>,
    button: Option<String>,
    delta: Option<i32>,
    key: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActionResponse {
    ok: bool,
    message: String,
    launched_path: Option<String>,
}

struct HttpRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

pub fn start(database_path: PathBuf) -> Result<RemoteHostState, String> {
    if std::env::var("HEART_ENABLE_REMOTE").as_deref() != Ok("1") {
        return Ok(unavailable(
            "원격 연결이 꺼져 있습니다. HEART_ENABLE_REMOTE=1로 설정한 뒤 앱을 다시 실행하세요."
                .to_string(),
        ));
    }
    start_inner(Some(database_path))
}

fn start_inner(database_path: Option<PathBuf>) -> Result<RemoteHostState, String> {
    let (listener, port) = bind_listener()?;
    start_bound_host(database_path, listener, port, local_ipv4_address(), true)
}

fn start_bound_host(
    database_path: Option<PathBuf>,
    listener: TcpListener,
    port: u16,
    address: String,
    enable_discovery: bool,
) -> Result<RemoteHostState, String> {
    let state = RemoteHostState {
        inner: Arc::new(Mutex::new(RemoteHostInner {
            address,
            port,
            pairing_code: new_pairing_code()?,
            auth_token: None,
            paired_device: None,
            last_seen: None,
            last_seen_at: None,
            failed_pair_attempts: 0,
            pairing_locked_until: None,
            error_message: None,
            h264_encoder: None,
            h264_hardware_accelerated: None,
            h264_error: None,
            h264_width: None,
            h264_height: None,
        })),
        database_path,
        capture_guard: Arc::new(Mutex::new(())),
        thumbnail_paths: Arc::new(RwLock::new(HashMap::new())),
        tailscale_address: tailscale_ipv4_address(),
    };

    let http_state = state.clone();
    thread::Builder::new()
        .name("heart-remote-http".to_string())
        .spawn(move || run_http_server(listener, http_state))
        .map_err(|error| format!("원격 연결 서버를 시작하지 못했습니다: {error}"))?;

    if enable_discovery {
        let discovery_state = state.clone();
        if let Err(error) = thread::Builder::new()
            .name("heart-remote-discovery".to_string())
            .spawn(move || run_discovery_server(discovery_state))
        {
            if let Ok(mut inner) = state.inner.lock() {
                inner.error_message =
                    Some(format!("자동 검색 서버를 시작하지 못했습니다: {error}"));
            }
        }
    }

    Ok(state)
}

pub fn unavailable(message: String) -> RemoteHostState {
    RemoteHostState {
        inner: Arc::new(Mutex::new(RemoteHostInner {
            address: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
            pairing_code: "------".to_string(),
            auth_token: None,
            paired_device: None,
            last_seen: None,
            last_seen_at: None,
            failed_pair_attempts: 0,
            pairing_locked_until: None,
            error_message: Some(message),
            h264_encoder: None,
            h264_hardware_accelerated: None,
            h264_error: None,
            h264_width: None,
            h264_height: None,
        })),
        database_path: None,
        capture_guard: Arc::new(Mutex::new(())),
        thumbnail_paths: Arc::new(RwLock::new(HashMap::new())),
        tailscale_address: None,
    }
}

#[tauri::command]
pub fn get_remote_host_status(state: State<'_, RemoteHostState>) -> RemoteHostStatus {
    state.status()
}

#[tauri::command]
pub fn reset_remote_pairing(state: State<'_, RemoteHostState>) -> Result<RemoteHostStatus, String> {
    if let Ok(mut inner) = state.inner.lock() {
        inner.pairing_code = new_pairing_code()?;
        inner.auth_token = None;
        inner.paired_device = None;
        inner.last_seen = None;
        inner.last_seen_at = None;
        inner.failed_pair_attempts = 0;
        inner.pairing_locked_until = None;
    }
    Ok(state.status())
}

impl RemoteHostState {
    fn status(&self) -> RemoteHostStatus {
        let Ok(inner) = self.inner.lock() else {
            return RemoteHostStatus {
                address: "127.0.0.1".to_string(),
                port: DEFAULT_PORT,
                connection_url: format!("http://127.0.0.1:{DEFAULT_PORT}"),
                tailscale_connection_url: None,
                pairing_code: "------".to_string(),
                paired_device: None,
                connected: false,
                last_seen_at: None,
                error_message: Some("원격 연결 상태를 읽지 못했습니다.".to_string()),
                h264_encoder: None,
                h264_hardware_accelerated: None,
                h264_error: None,
                h264_width: None,
                h264_height: None,
            };
        };

        let connected = inner
            .last_seen
            .is_some_and(|last_seen| last_seen.elapsed() < Duration::from_secs(12));
        RemoteHostStatus {
            address: inner.address.clone(),
            port: inner.port,
            connection_url: format!("http://{}:{}", inner.address, inner.port),
            tailscale_connection_url: self
                .tailscale_address
                .as_ref()
                .map(|address| format!("http://{address}:{}", inner.port)),
            pairing_code: inner.pairing_code.clone(),
            paired_device: inner.paired_device.clone(),
            connected,
            last_seen_at: inner.last_seen_at,
            error_message: inner.error_message.clone(),
            h264_encoder: inner.h264_encoder.clone(),
            h264_hardware_accelerated: inner.h264_hardware_accelerated,
            h264_error: inner.h264_error.clone(),
            h264_width: inner.h264_width,
            h264_height: inner.h264_height,
        }
    }
}

fn bind_listener() -> Result<(TcpListener, u16), String> {
    let mut errors = Vec::new();
    for port in DEFAULT_PORT..=LAST_PORT {
        match TcpListener::bind(("0.0.0.0", port)) {
            Ok(listener) => return Ok((listener, port)),
            Err(error) => errors.push(format!("{port}: {error}")),
        }
    }
    Err(format!(
        "원격 연결 포트({DEFAULT_PORT}-{LAST_PORT})를 열지 못했습니다: {}",
        errors.join(", ")
    ))
}

fn run_http_server(listener: TcpListener, state: RemoteHostState) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let request_state = state.clone();
                let _ = thread::Builder::new()
                    .name("heart-remote-request".to_string())
                    .spawn(move || handle_connection(stream, request_state));
            }
            Err(error) => {
                if let Ok(mut inner) = state.inner.lock() {
                    inner.error_message = Some(format!("원격 연결 요청 수신 오류: {error}"));
                }
            }
        }
    }
}

fn run_discovery_server(state: RemoteHostState) {
    let socket = match UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT)) {
        Ok(socket) => socket,
        Err(error) => {
            if let Ok(mut inner) = state.inner.lock() {
                inner.error_message = Some(format!(
                    "자동 검색 포트 {DISCOVERY_PORT}를 열지 못했습니다: {error}"
                ));
            }
            return;
        }
    };

    let mut buffer = [0_u8; 256];
    loop {
        let Ok((length, peer)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        if &buffer[..length] != DISCOVERY_REQUEST {
            continue;
        }
        let port = state
            .inner
            .lock()
            .map(|inner| inner.port)
            .unwrap_or(DEFAULT_PORT);
        let response = format!("HEART_HOST_V1|{port}|{}", host_name());
        let _ = socket.send_to(response.as_bytes(), peer);
    }
}

fn handle_connection(mut stream: TcpStream, state: RemoteHostState) {
    let _ = stream.set_nodelay(true);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(4)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(4)));

    let request = match read_request(&mut stream) {
        Ok(request) => request,
        Err(message) => {
            write_json_error(&mut stream, 400, "Bad Request", &message);
            return;
        }
    };

    if request.method == "OPTIONS" {
        write_response(&mut stream, 204, "No Content", "application/json", &[]);
        return;
    }

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/remote") | ("GET", "/remote/") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/html; charset=utf-8",
                include_str!("../remote-ui/index.html").as_bytes(),
            );
        }
        ("GET", "/remote/search.js") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/javascript; charset=utf-8",
                include_str!("../remote-ui/search.js").as_bytes(),
            );
        }
        ("GET", "/remote/app.js") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/javascript; charset=utf-8",
                include_str!("../remote-ui/app.js").as_bytes(),
            );
        }
        ("GET", "/remote/api.js") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/javascript; charset=utf-8",
                include_str!("../remote-ui/api.js").as_bytes(),
            );
        }
        ("GET", "/remote/cover-loader.js") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/javascript; charset=utf-8",
                include_str!("../remote-ui/cover-loader.js").as_bytes(),
            );
        }
        ("GET", "/remote/dom.js") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/javascript; charset=utf-8",
                include_str!("../remote-ui/dom.js").as_bytes(),
            );
        }
        ("GET", "/remote/styles.css") => {
            write_response(
                &mut stream,
                200,
                "OK",
                "text/css; charset=utf-8",
                include_str!("../remote-ui/styles.css").as_bytes(),
            );
        }
        ("GET", "/api/status") => {
            let paired = state
                .inner
                .lock()
                .map(|inner| inner.auth_token.is_some())
                .unwrap_or(false);
            let response = PublicStatusResponse {
                service: "HEART Remote Host",
                version: env!("CARGO_PKG_VERSION"),
                host_name: host_name(),
                paired,
            };
            write_serialized(&mut stream, 200, "OK", &response);
        }
        ("POST", "/api/pair") => pair_device(&mut stream, &state, &request.body),
        ("GET", "/api/heartbeat") => {
            if !authorize(&request, &state) {
                write_json_error(&mut stream, 401, "Unauthorized", "페어링이 필요합니다.");
                return;
            }
            mark_seen(&state);
            let response = HeartbeatResponse {
                ok: true,
                service: "HEART Remote Host",
                host_name: host_name(),
                server_time: unix_seconds(),
                message: "HEART와 연결되었습니다.",
            };
            write_serialized(&mut stream, 200, "OK", &response);
        }
        ("GET", "/api/library") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            match load_latest_library(&state) {
                Ok(snapshot) => write_serialized(&mut stream, 200, "OK", &snapshot),
                Err(message) => write_json_error(&mut stream, 404, "Not Found", &message),
            }
        }
        ("GET", "/api/thumbnail") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            serve_thumbnail(&mut stream, &request, &state);
        }
        ("GET", "/api/screen/stream") => {
            // An <img> request cannot set an Authorization header. Limit the
            // query-token exception to this media endpoint only.
            if !authorize_screen_stream(&request, &state) {
                write_json_error(&mut stream, 401, "Unauthorized", "페어링이 필요합니다.");
                return;
            }
            streaming::serve_mjpeg(&mut stream, &state);
        }
        ("GET", "/api/screen/h264") => {
            if !authorize_screen_stream(&request, &state) {
                write_json_error(&mut stream, 401, "Unauthorized", "페어링이 필요합니다.");
                return;
            }
            mark_seen(&state);
            let tailscale = request
                .query
                .get("network")
                .is_some_and(|network| network == "tailscale");
            streaming::serve_h264(&mut stream, &state, tailscale);
        }
        ("GET", "/api/screen") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            let _capture = match state.capture_guard.lock() {
                Ok(guard) => guard,
                Err(_) => {
                    write_json_error(
                        &mut stream,
                        500,
                        "Internal Server Error",
                        "화면 캡처 잠금 오류입니다.",
                    );
                    return;
                }
            };
            match desktop::capture_jpeg(streaming::MJPEG_WIDTH, streaming::MJPEG_QUALITY) {
                Ok(jpeg) => write_response(&mut stream, 200, "OK", "image/jpeg", &jpeg),
                Err(message) => {
                    write_json_error(&mut stream, 500, "Internal Server Error", &message)
                }
            }
        }
        ("POST", "/api/launch") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            launch_remote_item(&mut stream, &request.body, &state);
        }
        ("POST", "/api/stop") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            match stop_launched_item() {
                Ok(()) => write_serialized(
                    &mut stream,
                    200,
                    "OK",
                    &ActionResponse {
                        ok: true,
                        message: "실행 중인 작품을 종료했습니다.".to_string(),
                        launched_path: None,
                    },
                ),
                Err(message) => write_json_error(&mut stream, 409, "Conflict", &message),
            }
        }
        ("POST", "/api/input") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            handle_remote_input(&mut stream, &request.body);
        }
        ("POST", "/api/show-heart") => {
            if !authorize_or_reject(&mut stream, &request, &state) {
                return;
            }
            match show_heart_window(&state) {
                Ok(()) => write_serialized(
                    &mut stream,
                    200,
                    "OK",
                    &ActionResponse {
                        ok: true,
                        message: "노트북 HEART를 앞으로 가져왔습니다.".to_string(),
                        launched_path: None,
                    },
                ),
                Err(message) => write_json_error(&mut stream, 503, "Service Unavailable", &message),
            }
        }
        _ => write_json_error(&mut stream, 404, "Not Found", "지원하지 않는 요청입니다."),
    }
}

fn show_heart_window(_: &RemoteHostState) -> Result<(), String> {
    desktop::show_heart_window()
}

fn pair_device(stream: &mut TcpStream, state: &RemoteHostState, body: &[u8]) {
    let request: PairRequest = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(_) => {
            write_json_error(
                stream,
                400,
                "Bad Request",
                "페어링 요청 형식이 올바르지 않습니다.",
            );
            return;
        }
    };

    let device_name = request
        .device_name
        .unwrap_or_else(|| "Android 기기".to_string())
        .trim()
        .chars()
        .take(48)
        .collect::<String>();

    let result = state
        .inner
        .lock()
        .map_err(|_| "원격 연결 상태 잠금 오류".to_string())
        .and_then(|mut inner| {
            if inner
                .pairing_locked_until
                .is_some_and(|locked_until| Instant::now() < locked_until)
            {
                return Err("잘못된 코드가 반복되었습니다. 30초 후 다시 시도해 주세요.".to_string());
            }
            inner.pairing_locked_until = None;

            if request.code.trim() != inner.pairing_code {
                inner.failed_pair_attempts = inner.failed_pair_attempts.saturating_add(1);
                if inner.failed_pair_attempts >= 5 {
                    inner.failed_pair_attempts = 0;
                    inner.pairing_locked_until = Some(Instant::now() + Duration::from_secs(30));
                    return Err(
                        "잘못된 코드가 반복되었습니다. 30초 동안 페어링을 잠급니다.".to_string()
                    );
                }
                return Err("페어링 코드가 올바르지 않습니다.".to_string());
            }

            let token = new_auth_token()?;
            let next_pairing_code = new_pairing_code()?;
            inner.auth_token = Some(token.clone());
            inner.paired_device = Some(device_name.clone());
            inner.last_seen = Some(Instant::now());
            inner.last_seen_at = Some(unix_seconds());
            inner.failed_pair_attempts = 0;
            inner.pairing_locked_until = None;
            inner.pairing_code = next_pairing_code;
            Ok(token)
        });

    match result {
        Ok(token) => {
            let response = PairResponse {
                ok: true,
                service: "HEART Remote Host",
                host_name: host_name(),
                device_name,
                token,
                message: "HEART 페어링이 완료되었습니다.",
            };
            write_serialized(stream, 200, "OK", &response);
        }
        Err(message) => write_json_error(stream, 403, "Forbidden", &message),
    }
}

fn authorize(request: &HttpRequest, state: &RemoteHostState) -> bool {
    let Some(header) = request.headers.get("authorization") else {
        return false;
    };
    let Some(token) = header.strip_prefix("Bearer ") else {
        return false;
    };
    state
        .inner
        .lock()
        .ok()
        .and_then(|inner| inner.auth_token.clone())
        .is_some_and(|expected| expected == token)
}

fn authorize_screen_stream(request: &HttpRequest, state: &RemoteHostState) -> bool {
    let Some(token) = request.query.get("token") else {
        return false;
    };
    let authorized = state
        .inner
        .lock()
        .ok()
        .and_then(|inner| inner.auth_token.clone())
        .is_some_and(|expected| expected == *token);
    if authorized {
        mark_seen(state);
    }
    authorized
}

fn authorize_or_reject(
    stream: &mut TcpStream,
    request: &HttpRequest,
    state: &RemoteHostState,
) -> bool {
    if !authorize(request, state) {
        write_json_error(stream, 401, "Unauthorized", "페어링이 필요합니다.");
        return false;
    }
    mark_seen(state);
    true
}

fn load_latest_library(state: &RemoteHostState) -> Result<LibrarySnapshot, String> {
    let connection = open_remote_database(state)?;
    let root_path: Option<String> = connection
        .query_row(
            "SELECT path FROM library_roots ORDER BY datetime(last_scanned_at) DESC LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("최근 라이브러리를 찾지 못했습니다: {error}"))?;
    let root_path = root_path
        .ok_or_else(|| "노트북 HEART에서 먼저 작품 폴더를 한 번 스캔해 주세요.".to_string())?;
    let snapshot = repository::load_snapshot(&connection, &root_path)?;
    refresh_thumbnail_cache(state, &snapshot);
    Ok(snapshot)
}

fn open_remote_database(state: &RemoteHostState) -> Result<Connection, String> {
    let database_path = state
        .database_path
        .as_ref()
        .ok_or_else(|| "HEART 데이터베이스가 준비되지 않았습니다.".to_string())?;
    Connection::open(database_path)
        .map_err(|error| format!("HEART 데이터베이스를 열지 못했습니다: {error}"))
}

fn find_item<'a>(snapshot: &'a LibrarySnapshot, item_path: &str) -> Option<&'a ScannedItem> {
    snapshot
        .scan
        .unclassified_items
        .iter()
        .find(|item| item.path == item_path)
        .or_else(|| find_item_in_categories(&snapshot.scan.categories, item_path))
}

fn find_item_in_categories<'a>(
    categories: &'a [CategoryNode],
    item_path: &str,
) -> Option<&'a ScannedItem> {
    for category in categories {
        if let Some(item) = category.items.iter().find(|item| item.path == item_path) {
            return Some(item);
        }
        if let Some(item) = find_item_in_categories(&category.children, item_path) {
            return Some(item);
        }
    }
    None
}

fn serve_thumbnail(stream: &mut TcpStream, request: &HttpRequest, state: &RemoteHostState) {
    let Some(item_path) = request.query.get("item") else {
        write_json_error(stream, 400, "Bad Request", "작품 경로가 없습니다.");
        return;
    };
    let thumbnail_path = match resolve_thumbnail_path(state, item_path) {
        Ok(Some(path)) => path,
        Ok(None) => {
            write_json_error(stream, 404, "Not Found", "등록된 표지가 없습니다.");
            return;
        }
        Err(message) => {
            write_json_error(stream, 500, "Internal Server Error", &message);
            return;
        }
    };
    let path = Path::new(&thumbnail_path);
    if !path.is_file() {
        write_json_error(stream, 404, "Not Found", "표지 파일이 존재하지 않습니다.");
        return;
    }
    if path
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(u64::MAX)
        > 24 * 1024 * 1024
    {
        write_json_error(stream, 413, "Content Too Large", "표지 파일이 너무 큽니다.");
        return;
    }
    match fs::read(path) {
        Ok(bytes) => write_response(stream, 200, "OK", image_content_type(path), &bytes),
        Err(error) => write_json_error(
            stream,
            500,
            "Internal Server Error",
            &format!("표지를 읽지 못했습니다: {error}"),
        ),
    }
}

fn resolve_thumbnail_path(
    state: &RemoteHostState,
    item_path: &str,
) -> Result<Option<String>, String> {
    if let Ok(paths) = state.thumbnail_paths.read() {
        if let Some(path) = paths.get(item_path) {
            return Ok(path.clone());
        }
    }

    let connection = open_remote_database(state)?;
    let thumbnail_path = repository::load_thumbnail_path(&connection, item_path)?;
    if let Ok(mut paths) = state.thumbnail_paths.write() {
        paths.insert(item_path.to_string(), thumbnail_path.clone());
    }
    Ok(thumbnail_path)
}

fn refresh_thumbnail_cache(state: &RemoteHostState, snapshot: &LibrarySnapshot) {
    let mut paths = HashMap::with_capacity(snapshot.scan.total_items);
    collect_thumbnail_paths(&snapshot.scan.categories, &mut paths);
    for item in &snapshot.scan.unclassified_items {
        insert_thumbnail_path(item, &mut paths);
    }
    if let Ok(mut cached) = state.thumbnail_paths.write() {
        *cached = paths;
    }
}

fn collect_thumbnail_paths(
    categories: &[CategoryNode],
    paths: &mut HashMap<String, Option<String>>,
) {
    for category in categories {
        for item in &category.items {
            insert_thumbnail_path(item, paths);
        }
        collect_thumbnail_paths(&category.children, paths);
    }
}

fn insert_thumbnail_path(item: &ScannedItem, paths: &mut HashMap<String, Option<String>>) {
    let thumbnail_path = (!item.missing)
        .then(|| {
            item.custom_thumbnail_path
                .as_ref()
                .or(item.thumbnail_path.as_ref())
                .cloned()
        })
        .flatten();
    paths.insert(item.path.clone(), thumbnail_path);
}

fn image_content_type(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        _ => "image/jpeg",
    }
}

fn launch_remote_item(stream: &mut TcpStream, body: &[u8], state: &RemoteHostState) {
    let request: LaunchRequest = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(_) => {
            write_json_error(
                stream,
                400,
                "Bad Request",
                "작품 실행 요청 형식이 올바르지 않습니다.",
            );
            return;
        }
    };
    let snapshot = match load_latest_library(state) {
        Ok(snapshot) => snapshot,
        Err(message) => {
            write_json_error(stream, 404, "Not Found", &message);
            return;
        }
    };
    let Some(item) = find_item(&snapshot, request.item_path.trim()) else {
        write_json_error(
            stream,
            404,
            "Not Found",
            "라이브러리에서 작품을 찾지 못했습니다.",
        );
        return;
    };
    if item.missing {
        write_json_error(stream, 409, "Conflict", "현재 경로에서 사라진 작품입니다.");
        return;
    }
    let item_path = item.path.clone();
    let root_path = snapshot.scan.root_path.clone();
    match launch_item(root_path.clone(), item_path.clone()) {
        Ok(launched_path) => {
            if let Some(database_path) = &state.database_path {
                if let Ok(connection) = Connection::open(database_path) {
                    let _ = repository::record_open(&connection, &root_path, &item_path);
                }
            }
            write_serialized(
                stream,
                200,
                "OK",
                &ActionResponse {
                    ok: true,
                    message: format!("{} 실행을 시작했습니다.", item.title),
                    launched_path: Some(launched_path),
                },
            );
        }
        Err(message) => write_json_error(stream, 500, "Internal Server Error", &message),
    }
}

fn handle_remote_input(stream: &mut TcpStream, body: &[u8]) {
    let request: InputRequest = match serde_json::from_slice(body) {
        Ok(request) => request,
        Err(_) => {
            write_json_error(
                stream,
                400,
                "Bad Request",
                "원격 입력 형식이 올바르지 않습니다.",
            );
            return;
        }
    };

    let move_to_requested_position = || match (request.x, request.y) {
        (Some(x), Some(y)) => desktop::move_pointer(x, y),
        _ => Ok(()),
    };
    let result = match request.action.as_str() {
        "move" => desktop::move_pointer(request.x.unwrap_or(0.5), request.y.unwrap_or(0.5)),
        "down" => move_to_requested_position()
            .and_then(|_| desktop::mouse_button(request.button.as_deref().unwrap_or("left"), true)),
        "up" => move_to_requested_position().and_then(|_| {
            desktop::mouse_button(request.button.as_deref().unwrap_or("left"), false)
        }),
        "click" => move_to_requested_position()
            .and_then(|_| desktop::mouse_button(request.button.as_deref().unwrap_or("left"), true))
            .and_then(|_| {
                desktop::mouse_button(request.button.as_deref().unwrap_or("left"), false)
            }),
        "doubleClick" => (0..2).try_for_each(|_| {
            desktop::mouse_button("left", true).and_then(|_| desktop::mouse_button("left", false))
        }),
        "wheel" => {
            desktop::scroll(request.delta.unwrap_or(0));
            Ok(())
        }
        "horizontalWheel" => {
            desktop::scroll_horizontal(request.delta.unwrap_or(0));
            Ok(())
        }
        "key" => desktop::press_key(request.key.as_deref().unwrap_or_default()),
        "text" => desktop::type_text(request.text.as_deref().unwrap_or_default()),
        _ => Err("지원하지 않는 원격 입력입니다.".to_string()),
    };

    match result {
        Ok(()) => write_serialized(
            stream,
            200,
            "OK",
            &ActionResponse {
                ok: true,
                message: "원격 입력을 전달했습니다.".to_string(),
                launched_path: None,
            },
        ),
        Err(message) => write_json_error(stream, 400, "Bad Request", &message),
    }
}

fn mark_seen(state: &RemoteHostState) {
    if let Ok(mut inner) = state.inner.lock() {
        inner.last_seen = Some(Instant::now());
        inner.last_seen_at = Some(unix_seconds());
    }
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    let mut buffer = Vec::with_capacity(2048);
    let mut chunk = [0_u8; 2048];
    let header_end;

    loop {
        let length = stream
            .read(&mut chunk)
            .map_err(|error| format!("요청을 읽지 못했습니다: {error}"))?;
        if length == 0 {
            return Err("비어 있는 요청입니다.".to_string());
        }
        buffer.extend_from_slice(&chunk[..length]);
        if buffer.len() > MAX_REQUEST_BYTES {
            return Err("요청 크기가 너무 큽니다.".to_string());
        }
        if let Some(position) = find_bytes(&buffer, b"\r\n\r\n") {
            header_end = position + 4;
            break;
        }
    }

    let header_text = std::str::from_utf8(&buffer[..header_end])
        .map_err(|_| "요청 헤더가 UTF-8이 아닙니다.".to_string())?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| "요청 줄이 없습니다.".to_string())?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default().to_string();
    let target = request_parts.next().unwrap_or_default();
    let (path, query_text) = target.split_once('?').unwrap_or((target, ""));
    let path = path.to_string();
    let query = parse_query(query_text);
    if method.is_empty() || path.is_empty() {
        return Err("요청 줄이 올바르지 않습니다.".to_string());
    }

    let mut headers = HashMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    let content_length = headers
        .get("content-length")
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0);
    if header_end + content_length > MAX_REQUEST_BYTES {
        return Err("요청 본문이 너무 큽니다.".to_string());
    }

    while buffer.len() < header_end + content_length {
        let length = stream
            .read(&mut chunk)
            .map_err(|error| format!("요청 본문을 읽지 못했습니다: {error}"))?;
        if length == 0 {
            break;
        }
        buffer.extend_from_slice(&chunk[..length]);
    }
    if buffer.len() < header_end + content_length {
        return Err("요청 본문이 중간에 끊겼습니다.".to_string());
    }

    Ok(HttpRequest {
        method,
        path,
        query,
        headers,
        body: buffer[header_end..header_end + content_length].to_vec(),
    })
}

fn parse_query(query: &str) -> HashMap<String, String> {
    query
        .split('&')
        .filter(|part| !part.is_empty())
        .filter_map(|part| {
            let (name, value) = part.split_once('=').unwrap_or((part, ""));
            Some((percent_decode(name)?, percent_decode(value)?))
        })
        .collect()
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let high = hex_value(bytes[index + 1])?;
                let low = hex_value(bytes[index + 2])?;
                decoded.push(high << 4 | low);
                index += 3;
            }
            b'+' => {
                decoded.push(b' ');
                index += 1;
            }
            byte => {
                decoded.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(decoded).ok()
}

fn hex_value(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
}

fn write_serialized<T: Serialize>(stream: &mut TcpStream, status: u16, reason: &str, value: &T) {
    match serde_json::to_vec(value) {
        Ok(body) => write_response(
            stream,
            status,
            reason,
            "application/json; charset=utf-8",
            &body,
        ),
        Err(_) => write_json_error(
            stream,
            500,
            "Internal Server Error",
            "응답 생성에 실패했습니다.",
        ),
    }
}

fn write_json_error(stream: &mut TcpStream, status: u16, reason: &str, message: &str) {
    let body = serde_json::json!({ "ok": false, "message": message }).to_string();
    write_response(
        stream,
        status,
        reason,
        "application/json; charset=utf-8",
        body.as_bytes(),
    );
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
) {
    let cache_control =
        if content_type.starts_with("text/javascript") || content_type.starts_with("text/css") {
            "private, max-age=86400"
        } else {
            "no-store"
        };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: {cache_control}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: Authorization, Content-Type\r\nAccess-Control-Allow-Methods: GET, POST, OPTIONS\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(header.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn local_ipv4_address() -> String {
    UdpSocket::bind(("0.0.0.0", 0))
        .and_then(|socket| {
            socket.connect(("8.8.8.8", 80))?;
            socket.local_addr()
        })
        .map(|address| address.ip().to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn tailscale_ipv4_address() -> Option<String> {
    let output = Command::new("tailscale").args(["ip", "-4"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let address = stdout.lines().next()?.trim();
    address
        .parse::<std::net::Ipv4Addr>()
        .ok()
        .map(|address| address.to_string())
}

fn host_name() -> String {
    "HEART-PC".to_string()
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_pairing_code() -> Result<String, String> {
    let mut bytes = [0_u8; 4];
    fill_random(&mut bytes)?;
    let number = u32::from_le_bytes(bytes) % 900_000 + 100_000;
    Ok(number.to_string())
}

fn new_auth_token() -> Result<String, String> {
    let mut bytes = [0_u8; 32];
    fill_random(&mut bytes)?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn fill_random(buffer: &mut [u8]) -> Result<(), String> {
    getrandom::fill(buffer).map_err(|error| format!("보안 난수를 생성하지 못했습니다: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn start_test_host() -> RemoteHostState {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test HTTP listener");
        let port = listener.local_addr().expect("test listener address").port();
        start_bound_host(None, listener, port, "127.0.0.1".to_string(), false)
            .expect("start HEART Remote test host")
    }

    fn thumbnail_item(path: &str, detected: Option<&str>, custom: Option<&str>) -> ScannedItem {
        ScannedItem {
            id: path.to_string(),
            title: "Sample".to_string(),
            path: path.to_string(),
            thumbnail_path: detected.map(str::to_string),
            item_type: "folder".to_string(),
            file_count: 1,
            favorite: false,
            rating: None,
            notes: String::new(),
            custom_thumbnail_path: custom.map(str::to_string),
            tags: Vec::new(),
            open_count: 0,
            last_opened_at: None,
            missing: false,
            video_files: Vec::new(),
        }
    }

    fn send_http(port: u16, request: &str) -> String {
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to HEART Remote");
        stream.write_all(request.as_bytes()).expect("write request");
        let mut response = String::new();
        stream.read_to_string(&mut response).expect("read response");
        response
    }

    #[test]
    fn pairing_code_is_six_digits() {
        let code = new_pairing_code().expect("secure pairing code");
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|character| character.is_ascii_digit()));
    }

    #[test]
    fn auth_token_has_256_bits_in_hex() {
        let token = new_auth_token().expect("secure session token");
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn screen_stream_accepts_only_the_paired_query_token() {
        let state = unavailable("test state".to_string());
        state.inner.lock().unwrap().auth_token = Some("paired-token".to_string());
        let request = HttpRequest {
            method: "GET".to_string(),
            path: "/api/screen/stream".to_string(),
            query: HashMap::from([("token".to_string(), "paired-token".to_string())]),
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(authorize_screen_stream(&request, &state));

        let invalid_request = HttpRequest {
            query: HashMap::from([("token".to_string(), "other-token".to_string())]),
            ..request
        };
        assert!(!authorize_screen_stream(&invalid_request, &state));
    }

    #[test]
    fn library_refresh_populates_thumbnail_cache_without_database_fallback() {
        let state = unavailable("test state".to_string());
        let snapshot = LibrarySnapshot {
            scan: crate::core::scanner::ScanResult {
                root_path: "C:\\HEART".to_string(),
                categories: vec![CategoryNode {
                    id: "category".to_string(),
                    name: "Category".to_string(),
                    path: "C:\\HEART\\Category".to_string(),
                    children: Vec::new(),
                    items: vec![thumbnail_item(
                        "C:\\HEART\\Category\\A",
                        Some("detected.jpg"),
                        Some("custom.png"),
                    )],
                }],
                unclassified_items: vec![
                    thumbnail_item("C:\\HEART\\B", Some("unclassified.jpg"), None),
                    thumbnail_item("C:\\HEART\\NoCover", None, None),
                ],
                total_items: 3,
            },
            user_categories: Vec::new(),
        };

        refresh_thumbnail_cache(&state, &snapshot);

        assert_eq!(
            resolve_thumbnail_path(&state, "C:\\HEART\\Category\\A").unwrap(),
            Some("custom.png".to_string())
        );
        assert_eq!(
            resolve_thumbnail_path(&state, "C:\\HEART\\B").unwrap(),
            Some("unclassified.jpg".to_string())
        );
        assert_eq!(
            resolve_thumbnail_path(&state, "C:\\HEART\\NoCover").unwrap(),
            None
        );
    }

    #[test]
    fn remote_search_module_is_served() {
        let state = start_test_host();
        let request =
            "GET /remote/search.js HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n";
        let response = send_http(state.status().port, request);
        assert!(response.starts_with("HTTP/1.1 200 OK"));
        assert!(response.contains("filterItemsByQuery"));
    }

    #[test]
    fn remote_app_modules_are_served_with_cache_headers() {
        let state = start_test_host();
        for path in ["app.js", "api.js", "cover-loader.js", "dom.js"] {
            let request = format!(
                "GET /remote/{path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"
            );
            let response = send_http(state.status().port, &request);
            assert!(response.starts_with("HTTP/1.1 200 OK"));
            assert!(response.contains("Cache-Control: private, max-age=86400"));
        }
    }

    #[test]
    fn pair_and_heartbeat_work_over_http() {
        let state = start_test_host();
        let status = state.status();
        let pair_body = serde_json::json!({
            "code": status.pairing_code,
            "deviceName": "Samsung SM-S901N"
        })
        .to_string();
        let pair_request = format!(
            "POST /api/pair HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            pair_body.len(),
            pair_body
        );
        let pair_response = send_http(status.port, &pair_request);
        assert!(pair_response.starts_with("HTTP/1.1 200 OK"));
        let body = pair_response
            .split("\r\n\r\n")
            .nth(1)
            .expect("pair response body");
        let json: serde_json::Value = serde_json::from_str(body).expect("pair response json");
        let token = json["token"].as_str().expect("pair token");

        let heartbeat_request = format!(
            "GET /api/heartbeat HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\nConnection: close\r\n\r\n"
        );
        let heartbeat_response = send_http(status.port, &heartbeat_request);
        assert!(heartbeat_response.starts_with("HTTP/1.1 200 OK"));
        assert!(heartbeat_response.contains("HEART Remote Host"));
        assert!(state.status().connected);
        assert_eq!(
            state.status().paired_device.as_deref(),
            Some("Samsung SM-S901N")
        );
    }
}
