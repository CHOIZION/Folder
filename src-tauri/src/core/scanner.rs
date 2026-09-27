mod classification;
mod filesystem;
mod model;

use std::path::{Path, PathBuf};

use classification::{
    detect_file_type, detect_library_kind, is_image, is_item_directory, item_type_for_library,
    LibraryKind,
};
use filesystem::{
    count_all_files, file_name, file_stem, find_thumbnail, path_to_string, read_directory,
    sort_categories, DirectoryContents,
};

pub use filesystem::discover_video_files;
pub use model::{CategoryNode, ScanResult, ScannedItem};

pub fn scan_library(root_path: &str) -> Result<ScanResult, String> {
    let root = PathBuf::from(root_path);
    validate_root(&root, root_path)?;
    let contents = read_directory(&root)?;

    let mut unclassified_items = contents
        .files
        .iter()
        .map(|path| create_single_file_item(path))
        .collect::<Vec<_>>();
    let mut categories = Vec::with_capacity(contents.directories.len());

    for directory in contents.directories {
        let kind = detect_library_kind(&file_name(&directory));
        let contents = read_directory(&directory)?;
        categories.push(scan_category(
            &directory,
            contents,
            kind,
            &mut unclassified_items,
        )?);
    }

    sort_categories(&mut categories, |category| &category.name);
    sort_categories(&mut unclassified_items, |item| &item.title);
    let total_items = count_category_items(&categories) + unclassified_items.len();

    Ok(ScanResult {
        root_path: path_to_string(&root),
        categories,
        unclassified_items,
        total_items,
    })
}

fn validate_root(root: &Path, original: &str) -> Result<(), String> {
    if !root.is_absolute() {
        return Err("라이브러리 폴더의 절대 경로를 입력해 주세요.".to_owned());
    }
    if !root.exists() {
        return Err(format!("폴더가 존재하지 않습니다: {original}"));
    }
    if !root.is_dir() {
        return Err(format!("폴더 경로가 아닙니다: {original}"));
    }
    Ok(())
}

fn scan_category(
    directory: &Path,
    contents: DirectoryContents,
    kind: LibraryKind,
    unclassified: &mut Vec<ScannedItem>,
) -> Result<CategoryNode, String> {
    let mut category = empty_category(directory);
    unclassified.extend(
        contents
            .files
            .iter()
            .map(|path| create_single_file_item(path)),
    );

    for child in contents.directories {
        let child_contents = read_directory(&child)?;
        if is_item_directory(kind, &child_contents) {
            category
                .items
                .push(create_directory_item(&child, &child_contents.files, kind));
        } else if child_contents.files.is_empty() && child_contents.directories.is_empty() {
            category.children.push(empty_category(&child));
        } else {
            // Pass the already-read directory contents down. The old scanner read every
            // category directory twice, which was especially costly over network drives.
            category
                .children
                .push(scan_category(&child, child_contents, kind, unclassified)?);
        }
    }

    sort_categories(&mut category.children, |child| &child.name);
    sort_categories(&mut category.items, |item| &item.title);
    Ok(category)
}

fn empty_category(path: &Path) -> CategoryNode {
    CategoryNode {
        id: path_to_string(path),
        name: file_name(path),
        path: path_to_string(path),
        children: Vec::new(),
        items: Vec::new(),
    }
}

fn create_directory_item(
    directory: &Path,
    direct_files: &[PathBuf],
    kind: LibraryKind,
) -> ScannedItem {
    ScannedItem {
        id: path_to_string(directory),
        title: file_name(directory),
        path: path_to_string(directory),
        thumbnail_path: find_thumbnail(direct_files),
        item_type: item_type_for_library(kind).to_owned(),
        file_count: count_all_files(directory),
        favorite: false,
        rating: None,
        notes: String::new(),
        custom_thumbnail_path: None,
        tags: Vec::new(),
        open_count: 0,
        last_opened_at: None,
        missing: false,
        video_files: matches!(kind, LibraryKind::Video)
            .then(|| discover_video_files(directory))
            .unwrap_or_default(),
    }
}

fn create_single_file_item(path: &Path) -> ScannedItem {
    let item_type = detect_file_type(path);
    ScannedItem {
        id: path_to_string(path),
        title: file_stem(path),
        path: path_to_string(path),
        thumbnail_path: is_image(path).then(|| path_to_string(path)),
        item_type: item_type.to_owned(),
        file_count: 1,
        favorite: false,
        rating: None,
        notes: String::new(),
        custom_thumbnail_path: None,
        tags: Vec::new(),
        open_count: 0,
        last_opened_at: None,
        missing: false,
        video_files: if item_type == "video" {
            vec![path_to_string(path)]
        } else {
            Vec::new()
        },
    }
}

fn count_category_items(categories: &[CategoryNode]) -> usize {
    categories
        .iter()
        .map(|category| category.items.len() + count_category_items(&category.children))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_requires_an_explicit_absolute_path() {
        assert!(scan_library("").is_err());
        assert!(scan_library(".").is_err());
    }

    #[test]
    fn library_kind_aliases_are_stable() {
        assert!(matches!(detect_library_kind("렌파이"), LibraryKind::Game));
        assert!(matches!(detect_library_kind("videos"), LibraryKind::Video));
        assert!(matches!(detect_library_kind("기타"), LibraryKind::Generic));
    }

    #[test]
    fn known_file_types_are_case_insensitive() {
        assert_eq!(detect_file_type(Path::new("movie.MKV")), "video");
        assert_eq!(detect_file_type(Path::new("game.EXE")), "game");
        assert_eq!(detect_file_type(Path::new("cover.PNG")), "image");
    }
}
