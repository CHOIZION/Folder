mod discovery;
mod process;

use std::path::{Path, PathBuf};

use discovery::{
    find_first_file, find_game_executable, has_extension, IMAGE_EXTENSIONS, VIDEO_EXTENSIONS,
};

const HONEYVIEW_PATH: &str = r"C:\Program Files\Honeyview\Honeyview.exe";

#[tauri::command]
pub fn stop_launched_item() -> Result<(), String> {
    process::stop()
}

#[tauri::command]
pub fn launch_item(root_path: String, item_path: String) -> Result<String, String> {
    let root = PathBuf::from(root_path);
    let item = PathBuf::from(&item_path);
    if !item.exists() {
        return Err(format!("작품 경로가 존재하지 않습니다: {item_path}"));
    }

    match top_level_category(&root, &item).as_deref() {
        Some("만화" | "코믹스" | "comics" | "이미지" | "그림" | "사진" | "images" | "photos") => {
            launch_honeyview(&item)
        }
        Some("렌파이" | "renpy" | "쯔꾸르" | "rpgm" | "게임" | "games") => {
            launch_game(&item)
        }
        Some("영상" | "비디오" | "애니메이션" | "videos" | "movies") => {
            launch_media(&item)
        }
        _ => launch_automatically(&item),
    }
}

#[tauri::command]
pub fn launch_media_file(item_path: String, media_path: String) -> Result<String, String> {
    let item = PathBuf::from(&item_path);
    let media = PathBuf::from(&media_path);
    if !item.exists() {
        return Err(format!("작품 경로가 존재하지 않습니다: {item_path}"));
    }
    if !media.is_file() {
        return Err(format!("영상 파일이 존재하지 않습니다: {media_path}"));
    }
    if !VIDEO_EXTENSIONS
        .iter()
        .any(|extension| has_extension(&media, extension))
    {
        return Err(format!("지원하지 않는 영상 파일입니다: {media_path}"));
    }

    let canonical_item = canonicalize(&item, "작품")?;
    let canonical_media = canonicalize(&media, "영상")?;
    let belongs_to_item = if canonical_item.is_dir() {
        canonical_media.starts_with(&canonical_item)
    } else {
        canonical_media == canonical_item
    };
    if !belongs_to_item {
        return Err("선택한 영상이 해당 작품 폴더에 포함되어 있지 않습니다.".to_owned());
    }

    process::open_with_default_app(&canonical_media)?;
    Ok(path_string(&canonical_media))
}

fn top_level_category(root: &Path, item: &Path) -> Option<String> {
    item.strip_prefix(root)
        .ok()?
        .components()
        .next()
        .map(|part| part.as_os_str().to_string_lossy().trim().to_lowercase())
}

fn launch_honeyview(item: &Path) -> Result<String, String> {
    let viewer = Path::new(HONEYVIEW_PATH);
    let target = if item.is_dir() {
        find_first_file(item, IMAGE_EXTENSIONS, 4).unwrap_or_else(|| item.to_path_buf())
    } else {
        item.to_path_buf()
    };
    if !viewer.exists() {
        process::open_with_default_app(&target)?;
        return Ok(path_string(&target));
    }
    process::spawn_with_argument(
        viewer,
        &target,
        viewer.parent().unwrap_or(item),
        "Honeyview 실행에 실패했습니다",
    )?;
    Ok(path_string(&target))
}

fn launch_game(item: &Path) -> Result<String, String> {
    let executable = if item.is_file() && has_extension(item, "exe") {
        item.to_path_buf()
    } else {
        find_game_executable(item)
            .ok_or_else(|| format!("실행할 게임 EXE 파일을 찾지 못했습니다: {}", item.display()))?
    };
    process::spawn(
        &executable,
        executable.parent().unwrap_or(item),
        "게임 실행에 실패했습니다",
    )?;
    Ok(path_string(&executable))
}

fn launch_media(item: &Path) -> Result<String, String> {
    let target = if item.is_dir() {
        find_first_file(item, VIDEO_EXTENSIONS, 5)
            .ok_or_else(|| format!("재생할 영상 파일을 찾지 못했습니다: {}", item.display()))?
    } else {
        item.to_path_buf()
    };
    process::open_with_default_app(&target)?;
    Ok(path_string(&target))
}

fn launch_automatically(item: &Path) -> Result<String, String> {
    if item.is_dir() {
        if let Some(executable) = find_game_executable(item) {
            process::spawn(
                &executable,
                executable.parent().unwrap_or(item),
                "실행 파일 실행에 실패했습니다",
            )?;
            return Ok(path_string(&executable));
        }
        if let Some(video) = find_first_file(item, VIDEO_EXTENSIONS, 5) {
            process::open_with_default_app(&video)?;
            return Ok(path_string(&video));
        }
        if let Some(image) = find_first_file(item, IMAGE_EXTENSIONS, 4) {
            return launch_honeyview(&image);
        }
    }
    process::open_with_default_app(item)?;
    Ok(path_string(item))
}

fn canonicalize(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("{label} 경로를 확인할 수 없습니다: {error}"))
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
