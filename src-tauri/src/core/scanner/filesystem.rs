use std::{
    fs,
    path::{Path, PathBuf},
};

use super::classification::{is_image, is_video};

#[derive(Debug)]
pub(super) struct DirectoryContents {
    pub files: Vec<PathBuf>,
    pub directories: Vec<PathBuf>,
}

pub(super) fn read_directory(path: &Path) -> Result<DirectoryContents, String> {
    let entries = fs::read_dir(path)
        .map_err(|error| format!("폴더를 읽을 수 없습니다: {} ({error})", path.display()))?;
    let mut contents = DirectoryContents {
        files: Vec::new(),
        directories: Vec::new(),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if should_ignore(&path) {
            continue;
        }
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            contents.directories.push(path);
        } else if file_type.is_file() {
            contents.files.push(path);
        }
    }
    sort_paths(&mut contents.files);
    sort_paths(&mut contents.directories);
    Ok(contents)
}

pub(super) fn count_all_files(directory: &Path) -> usize {
    let Ok(entries) = fs::read_dir(directory) else {
        return 0;
    };
    entries.flatten().fold(0, |total, entry| {
        let path = entry.path();
        if should_ignore(&path) {
            return total;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_file() => total + 1,
            Ok(kind) if kind.is_dir() => total + count_all_files(&path),
            _ => total,
        }
    })
}

pub(super) fn find_thumbnail(files: &[PathBuf]) -> Option<String> {
    files
        .iter()
        .filter(|path| is_image(path))
        .min_by_key(|path| thumbnail_priority(path))
        .map(|path| path_to_string(path))
}

pub fn discover_video_files(path: &Path) -> Vec<String> {
    if path.is_file() {
        return if is_video(path) {
            vec![path_to_string(path)]
        } else {
            Vec::new()
        };
    }
    let Ok(entries) = fs::read_dir(path) else {
        return Vec::new();
    };
    let mut videos: Vec<PathBuf> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            entry
                .file_type()
                .ok()
                .filter(|kind| kind.is_file() && is_video(&path))
                .map(|_| path)
        })
        .collect();
    sort_paths(&mut videos);
    videos.iter().map(|path| path_to_string(path)).collect()
}

pub(super) fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path_to_string(path))
}

pub(super) fn file_stem(path: &Path) -> String {
    path.file_stem()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| file_name(path))
}

pub(super) fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

pub(super) fn sort_categories<T, F>(values: &mut [T], name: F)
where
    F: Fn(&T) -> &str,
{
    values.sort_by_cached_key(|value| name(value).to_lowercase());
}

fn sort_paths(paths: &mut [PathBuf]) {
    paths.sort_by_cached_key(|path| file_name(path).to_lowercase());
}

fn should_ignore(path: &Path) -> bool {
    let name = file_name(path).to_lowercase();
    matches!(name.as_str(), "desktop.ini" | "thumbs.db" | ".ds_store") || name.starts_with("~$")
}

fn thumbnail_priority(path: &Path) -> (u8, String) {
    let name = file_name(path).to_lowercase();
    let priority = if name.contains("cover") {
        0
    } else if name.contains("folder") {
        1
    } else if name.contains("thumb") {
        2
    } else if name.contains("poster") {
        3
    } else {
        10
    };
    (priority, name)
}
