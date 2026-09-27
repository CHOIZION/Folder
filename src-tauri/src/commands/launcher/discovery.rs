use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "webp", "bmp", "gif", "avif", "jxl", "heic",
];
pub(super) const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "wmv", "mov", "webm", "m4v", "mpg", "mpeg", "ts", "m2ts", "flv", "3gp",
];

pub(super) fn find_game_executable(root: &Path) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }
    let mut candidates = Vec::new();
    collect_executables(root, 0, 4, &mut candidates);
    candidates
        .into_iter()
        .min_by_key(|path| executable_priority(root, path))
}

pub(super) fn find_first_file(
    root: &Path,
    extensions: &[&str],
    max_depth: usize,
) -> Option<PathBuf> {
    let mut first = None;
    visit_matching_files(root, 0, max_depth, extensions, &mut first);
    first
}

pub(super) fn has_extension(path: &Path, expected: &str) -> bool {
    path.extension()
        .and_then(|value| value.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case(expected))
}

fn collect_executables(current: &Path, depth: usize, max_depth: usize, output: &mut Vec<PathBuf>) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            if !is_ignored_directory(&path) {
                collect_executables(&path, depth + 1, max_depth, output);
            }
        } else if kind.is_file() && has_extension(&path, "exe") && !is_ignored_executable(&path) {
            output.push(path);
        }
    }
}

fn visit_matching_files(
    current: &Path,
    depth: usize,
    max_depth: usize,
    extensions: &[&str],
    first: &mut Option<PathBuf>,
) {
    if depth > max_depth {
        return;
    }
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_dir() {
            visit_matching_files(&path, depth + 1, max_depth, extensions, first);
        } else if kind.is_file() && extensions.iter().any(|value| has_extension(&path, value)) {
            let replace = first
                .as_ref()
                .is_none_or(|current| compare_path(&path, current).is_lt());
            if replace {
                *first = Some(path);
            }
        }
    }
}

fn compare_path(left: &Path, right: &Path) -> std::cmp::Ordering {
    left.to_string_lossy()
        .to_lowercase()
        .cmp(&right.to_string_lossy().to_lowercase())
}

fn executable_priority(root: &Path, path: &Path) -> (u8, usize, usize, String) {
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let folder = root
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    let depth = path
        .strip_prefix(root)
        .map(|value| value.components().count())
        .unwrap_or(99);
    let rank = if stem == "game" {
        0
    } else if !folder.is_empty() && stem == folder {
        1
    } else if depth == 1 {
        2
    } else {
        3
    };
    (rank, depth, path.as_os_str().len(), stem)
}

fn is_ignored_directory(path: &Path) -> bool {
    matches!(
        path.file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "lib"
            | "renpy"
            | "game"
            | "www"
            | "wwwroot"
            | "node_modules"
            | "engine"
            | "runtime"
            | "redist"
            | "redistributable"
    )
}

fn is_ignored_executable(path: &Path) -> bool {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_lowercase();
    [
        "unins",
        "uninstall",
        "setup",
        "install",
        "launcher",
        "update",
        "updater",
        "crash",
        "report",
        "dxsetup",
        "vc_redist",
        "unitycrashhandler",
    ]
    .iter()
    .any(|word| name.contains(word))
}
