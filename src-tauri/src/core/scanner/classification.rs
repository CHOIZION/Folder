use std::path::{Path, PathBuf};

use super::filesystem::DirectoryContents;

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "webp", "gif", "bmp", "avif", "jfif"];
const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "mkv", "avi", "mov", "wmv", "webm", "m4v", "flv", "ts", "mpeg", "mpg",
];
const DOCUMENT_EXTENSIONS: &[&str] = &[
    "pdf", "epub", "txt", "doc", "docx", "html", "htm", "rtf", "mobi",
];

#[derive(Debug, Clone, Copy)]
pub(super) enum LibraryKind {
    ImageBook,
    ImageCollection,
    Game,
    Video,
    Document,
    Generic,
}

pub(super) fn detect_library_kind(category_name: &str) -> LibraryKind {
    match category_name.trim().to_lowercase().as_str() {
        "만화" | "코믹스" | "comics" => LibraryKind::ImageBook,
        "이미지" | "그림" | "사진" | "images" | "photos" => LibraryKind::ImageCollection,
        "렌파이" | "renpy" | "쯔꾸르" | "rpgm" | "게임" | "games" => LibraryKind::Game,
        "영상" | "비디오" | "애니메이션" | "videos" | "movies" => LibraryKind::Video,
        "소설" | "문서" | "전자책" | "documents" | "books" => LibraryKind::Document,
        _ => LibraryKind::Generic,
    }
}

pub(super) fn is_item_directory(kind: LibraryKind, contents: &DirectoryContents) -> bool {
    match kind {
        LibraryKind::ImageBook => {
            let images = count_images(&contents.files);
            images >= 2
                || (images == 1 && contents.directories.is_empty())
                || contains_extension(&contents.files, &["pdf", "cbz", "cbr", "zip", "rar", "7z"])
        }
        LibraryKind::ImageCollection => contains_extension(&contents.files, IMAGE_EXTENSIONS),
        LibraryKind::Game => contains_extension(&contents.files, &["exe"]),
        LibraryKind::Video => contains_extension(&contents.files, VIDEO_EXTENSIONS),
        LibraryKind::Document => contains_extension(&contents.files, DOCUMENT_EXTENSIONS),
        LibraryKind::Generic => {
            let images = count_images(&contents.files);
            contains_extension(
                &contents.files,
                &[
                    "exe", "mp4", "mkv", "avi", "mov", "wmv", "webm", "pdf", "epub", "txt", "cbz",
                    "cbr", "zip", "rar", "7z",
                ],
            ) || images >= 2
                || (images == 1 && contents.directories.is_empty())
        }
    }
}

pub(super) fn item_type_for_library(kind: LibraryKind) -> &'static str {
    match kind {
        LibraryKind::Game => "game",
        LibraryKind::Video => "video",
        LibraryKind::ImageBook | LibraryKind::ImageCollection => "image",
        LibraryKind::Document => "document",
        LibraryKind::Generic => "folder",
    }
}

pub(super) fn detect_file_type(path: &Path) -> &'static str {
    if has_extension(path, &["exe"]) {
        return "game";
    }
    if has_extension(path, VIDEO_EXTENSIONS) {
        return "video";
    }
    if has_extension(path, IMAGE_EXTENSIONS) {
        return "image";
    }
    if has_extension(path, DOCUMENT_EXTENSIONS) {
        return "document";
    }
    if has_extension(path, &["zip", "rar", "7z", "tar", "cbz", "cbr"]) {
        return "archive";
    }
    "folder"
}

pub(super) fn is_image(path: &Path) -> bool {
    has_extension(path, IMAGE_EXTENSIONS)
}

pub(super) fn is_video(path: &Path) -> bool {
    has_extension(path, VIDEO_EXTENSIONS) || has_extension(path, &["m2ts", "3gp"])
}

fn count_images(files: &[PathBuf]) -> usize {
    files.iter().filter(|path| is_image(path)).count()
}

fn contains_extension(files: &[PathBuf], extensions: &[&str]) -> bool {
    files.iter().any(|path| has_extension(path, extensions))
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extensions
                .iter()
                .any(|value| extension.eq_ignore_ascii_case(value))
        })
}
