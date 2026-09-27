use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use regex::Regex;

use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use crate::core::scanner::{discover_video_files, CategoryNode, ScanResult, ScannedItem};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibrarySnapshot {
    #[serde(flatten)]
    pub scan: ScanResult,
    pub user_categories: Vec<UserCategory>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserCategory {
    pub id: i64,
    pub name: String,
    pub item_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataUpdate {
    pub path: String,
    pub favorite: bool,
    pub rating: Option<i64>,
    pub notes: String,
    pub custom_thumbnail_path: Option<String>,
    pub tags: Vec<String>,
}

pub fn save_scan(
    connection: &mut Connection,
    scan: &ScanResult,
) -> Result<LibrarySnapshot, String> {
    let scan_json = serde_json::to_string(scan)
        .map_err(|error| format!("스캔 결과 직렬화에 실패했습니다: {error}"))?;

    let transaction = connection
        .transaction()
        .map_err(|error| format!("DB 트랜잭션 시작에 실패했습니다: {error}"))?;

    transaction
        .execute(
            r#"
            INSERT INTO library_roots(path, scan_json, last_scanned_at)
            VALUES (?1, ?2, CURRENT_TIMESTAMP)
            ON CONFLICT(path) DO UPDATE SET
                scan_json = excluded.scan_json,
                last_scanned_at = CURRENT_TIMESTAMP
            "#,
            params![&scan.root_path, &scan_json],
        )
        .map_err(|error| format!("라이브러리 루트 저장에 실패했습니다: {error}"))?;

    transaction
        .execute(
            "UPDATE items SET missing = 1 WHERE root_path = ?1",
            params![&scan.root_path],
        )
        .map_err(|error| format!("기존 작품 상태 초기화에 실패했습니다: {error}"))?;

    let scanned_items = all_items(scan);
    let scanned_paths: HashSet<&str> = scanned_items
        .iter()
        .map(|item| item.path.as_str())
        .collect();

    // 렌파이 작품은 폴더명 끝의 버전만 달라지는 업데이트가 자주 발생합니다.
    // 새 경로가 기존 DB에 없고, 사라진 이전 버전과 기준 제목이 정확히 일치할 때
    // 기존 메타데이터(태그, 메모, 평점, 즐겨찾기, 실행 기록, 사용자 카테고리)를
    // 새 경로로 자동 이전합니다.
    migrate_renpy_updates(
        &transaction,
        &scan.root_path,
        &scanned_items,
        &scanned_paths,
    )?;

    for item in scanned_items {
        upsert_item(&transaction, &scan.root_path, item)?;
    }

    transaction
        .commit()
        .map_err(|error| format!("DB 저장 확정에 실패했습니다: {error}"))?;

    load_snapshot(connection, &scan.root_path)
}

pub fn load_snapshot(connection: &Connection, root_path: &str) -> Result<LibrarySnapshot, String> {
    let scan_json: String = connection
        .query_row(
            "SELECT scan_json FROM library_roots WHERE path = ?1",
            params![root_path],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| format!("라이브러리 조회에 실패했습니다: {error}"))?
        .ok_or_else(|| "저장된 라이브러리가 없습니다. 다시 스캔해 주세요.".to_string())?;

    let mut scan: ScanResult = serde_json::from_str(&scan_json)
        .map_err(|error| format!("저장된 라이브러리 해석에 실패했습니다: {error}"))?;

    let metadata = load_item_metadata(connection, root_path)?;
    enrich_categories(&mut scan.categories, &metadata);
    for item in &mut scan.unclassified_items {
        enrich_item(item, &metadata);
    }

    let user_categories = load_user_categories(connection, root_path)?;

    Ok(LibrarySnapshot {
        scan,
        user_categories,
    })
}

pub fn load_thumbnail_path(
    connection: &Connection,
    item_path: &str,
) -> Result<Option<String>, String> {
    connection
        .query_row(
            r#"
            SELECT COALESCE(NULLIF(TRIM(custom_thumbnail_path), ''), detected_thumbnail_path)
            FROM items
            WHERE path = ?1 AND missing = 0
            "#,
            params![item_path],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map(Option::flatten)
        .map_err(|error| format!("작품 표지 경로 조회에 실패했습니다: {error}"))
}

pub fn update_metadata(
    connection: &mut Connection,
    root_path: &str,
    update: MetadataUpdate,
) -> Result<LibrarySnapshot, String> {
    if let Some(rating) = update.rating {
        if !(0..=5).contains(&rating) {
            return Err("평점은 0점부터 5점까지만 저장할 수 있습니다.".to_string());
        }
    }

    let transaction = connection
        .transaction()
        .map_err(|error| format!("DB 트랜잭션 시작에 실패했습니다: {error}"))?;

    let changed = transaction
        .execute(
            r#"
            UPDATE items
            SET favorite = ?2,
                rating = ?3,
                notes = ?4,
                custom_thumbnail_path = ?5
            WHERE path = ?1
            "#,
            params![
                &update.path,
                update.favorite as i64,
                update.rating,
                update.notes.trim(),
                clean_optional(update.custom_thumbnail_path)
            ],
        )
        .map_err(|error| format!("작품 정보 저장에 실패했습니다: {error}"))?;

    if changed == 0 {
        return Err("수정할 작품을 DB에서 찾지 못했습니다.".to_string());
    }

    transaction
        .execute(
            "DELETE FROM item_tags WHERE item_path = ?1",
            params![&update.path],
        )
        .map_err(|error| format!("기존 태그 삭제에 실패했습니다: {error}"))?;

    for tag in normalize_tags(update.tags) {
        transaction
            .execute(
                "INSERT OR IGNORE INTO tags(name) VALUES (?1)",
                params![&tag],
            )
            .map_err(|error| format!("태그 생성에 실패했습니다: {error}"))?;

        transaction
            .execute(
                r#"
                INSERT OR IGNORE INTO item_tags(item_path, tag_id)
                SELECT ?1, id FROM tags WHERE name = ?2 COLLATE NOCASE
                "#,
                params![&update.path, &tag],
            )
            .map_err(|error| format!("작품 태그 연결에 실패했습니다: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("작품 정보 저장 확정에 실패했습니다: {error}"))?;

    load_snapshot(connection, root_path)
}

pub fn record_open(
    connection: &Connection,
    root_path: &str,
    item_path: &str,
) -> Result<LibrarySnapshot, String> {
    connection
        .execute(
            r#"
            UPDATE items
            SET open_count = open_count + 1,
                last_opened_at = CURRENT_TIMESTAMP
            WHERE path = ?1
            "#,
            params![item_path],
        )
        .map_err(|error| format!("실행 기록 저장에 실패했습니다: {error}"))?;

    load_snapshot(connection, root_path)
}

pub fn create_user_category(
    connection: &Connection,
    root_path: &str,
    name: &str,
) -> Result<LibrarySnapshot, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("카테고리 이름을 입력해 주세요.".to_string());
    }

    connection
        .execute(
            r#"
            INSERT INTO user_categories(root_path, name, sort_order)
            VALUES (
                ?1,
                ?2,
                COALESCE((SELECT MAX(sort_order) + 1 FROM user_categories WHERE root_path = ?1), 0)
            )
            "#,
            params![root_path, name],
        )
        .map_err(|error| format!("사용자 카테고리 생성에 실패했습니다: {error}"))?;

    load_snapshot(connection, root_path)
}

pub fn delete_user_category(
    connection: &Connection,
    root_path: &str,
    category_id: i64,
) -> Result<LibrarySnapshot, String> {
    connection
        .execute(
            "DELETE FROM user_categories WHERE id = ?1 AND root_path = ?2",
            params![category_id, root_path],
        )
        .map_err(|error| format!("사용자 카테고리 삭제에 실패했습니다: {error}"))?;

    load_snapshot(connection, root_path)
}

pub fn set_item_user_categories(
    connection: &mut Connection,
    root_path: &str,
    item_path: &str,
    category_ids: Vec<i64>,
) -> Result<LibrarySnapshot, String> {
    let transaction = connection
        .transaction()
        .map_err(|error| format!("DB 트랜잭션 시작에 실패했습니다: {error}"))?;

    transaction
        .execute(
            r#"
            DELETE FROM user_category_items
            WHERE item_path = ?1
              AND category_id IN (
                SELECT id FROM user_categories WHERE root_path = ?2
              )
            "#,
            params![item_path, root_path],
        )
        .map_err(|error| format!("기존 사용자 카테고리 연결 해제에 실패했습니다: {error}"))?;

    for category_id in category_ids {
        transaction
            .execute(
                r#"
                INSERT OR IGNORE INTO user_category_items(category_id, item_path)
                SELECT id, ?2
                FROM user_categories
                WHERE id = ?1 AND root_path = ?3
                "#,
                params![category_id, item_path, root_path],
            )
            .map_err(|error| format!("사용자 카테고리 연결에 실패했습니다: {error}"))?;
    }

    transaction
        .commit()
        .map_err(|error| format!("사용자 카테고리 저장 확정에 실패했습니다: {error}"))?;

    load_snapshot(connection, root_path)
}

fn migrate_renpy_updates(
    transaction: &Transaction<'_>,
    root_path: &str,
    scanned_items: &[&ScannedItem],
    scanned_paths: &HashSet<&str>,
) -> Result<(), String> {
    for new_item in scanned_items {
        if !is_renpy_item(new_item) || item_exists(transaction, &new_item.path)? {
            continue;
        }

        let Some(new_base_title) = renpy_base_title(&new_item.title) else {
            continue;
        };

        let mut statement = transaction
            .prepare(
                r#"
                SELECT path, title
                FROM items
                WHERE root_path = ?1
                  AND missing = 1
                  AND item_type = 'game'
                  AND path <> ?2
                "#,
            )
            .map_err(|error| format!("렌파이 업데이트 후보 조회 준비에 실패했습니다: {error}"))?;

        let rows = statement
            .query_map(params![root_path, &new_item.path], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|error| format!("렌파이 업데이트 후보 조회에 실패했습니다: {error}"))?;

        let mut candidates = Vec::new();
        for row in rows {
            let (old_path, old_title) =
                row.map_err(|error| format!("렌파이 업데이트 후보 변환에 실패했습니다: {error}"))?;

            // 현재 스캔에도 존재하는 작품은 이전 버전으로 간주하지 않습니다.
            if scanned_paths.contains(old_path.as_str()) || !is_renpy_path(&old_path) {
                continue;
            }

            if renpy_base_title(&old_title).as_deref() == Some(new_base_title.as_str()) {
                candidates.push(old_path);
            }
        }

        // 후보가 정확히 하나일 때만 자동 이전합니다. 모호하면 신규 작품으로 둡니다.
        if candidates.len() == 1 {
            migrate_item_path(transaction, &candidates[0], new_item)?;
        }
    }

    Ok(())
}

fn item_exists(transaction: &Transaction<'_>, path: &str) -> Result<bool, String> {
    transaction
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM items WHERE path = ?1)",
            params![path],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .map_err(|error| format!("작품 경로 확인에 실패했습니다: {error}"))
}

fn migrate_item_path(
    transaction: &Transaction<'_>,
    old_path: &str,
    new_item: &ScannedItem,
) -> Result<(), String> {
    transaction
        .execute(
            r#"
            INSERT INTO items(
                path, root_path, title, item_type,
                detected_thumbnail_path, file_count, missing,
                favorite, rating, notes, custom_thumbnail_path,
                open_count, last_opened_at, last_seen_at
            )
            SELECT
                ?2, root_path, ?3, ?4,
                ?5, ?6, 0,
                favorite, rating, notes, custom_thumbnail_path,
                open_count, last_opened_at, CURRENT_TIMESTAMP
            FROM items
            WHERE path = ?1
            "#,
            params![
                old_path,
                &new_item.path,
                &new_item.title,
                &new_item.item_type,
                &new_item.thumbnail_path,
                new_item.file_count as i64,
            ],
        )
        .map_err(|error| {
            format!(
                "렌파이 업데이트 정보 이전에 실패했습니다: {old_path} -> {} ({error})",
                new_item.path
            )
        })?;

    transaction
        .execute(
            r#"
            INSERT OR IGNORE INTO item_tags(item_path, tag_id)
            SELECT ?2, tag_id FROM item_tags WHERE item_path = ?1
            "#,
            params![old_path, &new_item.path],
        )
        .map_err(|error| format!("렌파이 태그 이전에 실패했습니다: {error}"))?;

    transaction
        .execute(
            r#"
            INSERT OR IGNORE INTO user_category_items(category_id, item_path)
            SELECT category_id, ?2 FROM user_category_items WHERE item_path = ?1
            "#,
            params![old_path, &new_item.path],
        )
        .map_err(|error| format!("렌파이 사용자 카테고리 이전에 실패했습니다: {error}"))?;

    transaction
        .execute("DELETE FROM items WHERE path = ?1", params![old_path])
        .map_err(|error| format!("렌파이 이전 버전 정리에 실패했습니다: {error}"))?;

    Ok(())
}

fn is_renpy_item(item: &ScannedItem) -> bool {
    item.item_type == "game" && is_renpy_path(&item.path)
}

fn is_renpy_path(path: &str) -> bool {
    path.split(['/', '\\']).any(|component| {
        let name = component.trim().to_lowercase();
        matches!(name.as_str(), "렌파이" | "renpy")
    })
}

fn renpy_base_title(title: &str) -> Option<String> {
    static VERSION_SUFFIX: OnceLock<Regex> = OnceLock::new();
    let regex = VERSION_SUFFIX.get_or_init(|| {
        Regex::new(
            r"(?ix)
            ^(?P<base>.*?)
            (?:[\s._-]+)?
            (?:v(?:ersion)?[\s._-]*)?
            [\[(]?
            \d+(?:\.\d+){1,3}[a-z]?
            [\])]?
            \s*$",
        )
        .expect("렌파이 버전 정규식이 올바르지 않습니다")
    });

    let captures = regex.captures(title.trim())?;
    let base = captures.name("base")?.as_str().trim();
    if base.is_empty() {
        return None;
    }

    Some(normalize_identity_text(base))
}

fn normalize_identity_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn upsert_item(
    transaction: &Transaction<'_>,
    root_path: &str,
    item: &ScannedItem,
) -> Result<(), String> {
    transaction
        .execute(
            r#"
            INSERT INTO items(
                path, root_path, title, item_type,
                detected_thumbnail_path, file_count,
                missing, last_seen_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, CURRENT_TIMESTAMP)
            ON CONFLICT(path) DO UPDATE SET
                root_path = excluded.root_path,
                title = excluded.title,
                item_type = excluded.item_type,
                detected_thumbnail_path = excluded.detected_thumbnail_path,
                file_count = excluded.file_count,
                missing = 0,
                last_seen_at = CURRENT_TIMESTAMP
            "#,
            params![
                &item.path,
                root_path,
                &item.title,
                &item.item_type,
                &item.thumbnail_path,
                item.file_count as i64
            ],
        )
        .map_err(|error| format!("작품 저장에 실패했습니다: {} ({error})", item.path))?;

    Ok(())
}

fn all_items(scan: &ScanResult) -> Vec<&ScannedItem> {
    let mut items = Vec::new();
    collect_category_items(&scan.categories, &mut items);
    items.extend(scan.unclassified_items.iter());
    items
}

fn collect_category_items<'a>(categories: &'a [CategoryNode], items: &mut Vec<&'a ScannedItem>) {
    for category in categories {
        items.extend(category.items.iter());
        collect_category_items(&category.children, items);
    }
}

fn load_item_metadata(
    connection: &Connection,
    root_path: &str,
) -> Result<HashMap<String, ScannedItem>, String> {
    let mut tags_by_path = load_tags_by_path(connection, root_path)?;
    let mut statement = connection
        .prepare(
            r#"
            SELECT
                path, title, detected_thumbnail_path, item_type, file_count,
                favorite, rating, notes, custom_thumbnail_path,
                open_count, last_opened_at, missing
            FROM items
            WHERE root_path = ?1
            "#,
        )
        .map_err(|error| format!("작품 정보 조회 준비에 실패했습니다: {error}"))?;

    let rows = statement
        .query_map(params![root_path], |row| {
            let path: String = row.get(0)?;
            Ok((
                path.clone(),
                ScannedItem {
                    id: path.clone(),
                    path,
                    title: row.get(1)?,
                    thumbnail_path: row.get(2)?,
                    item_type: row.get(3)?,
                    file_count: row.get::<_, i64>(4)?.max(0) as usize,
                    favorite: row.get::<_, i64>(5)? != 0,
                    rating: row.get(6)?,
                    notes: row.get(7)?,
                    custom_thumbnail_path: row.get(8)?,
                    tags: Vec::new(),
                    open_count: row.get(9)?,
                    last_opened_at: row.get(10)?,
                    missing: row.get::<_, i64>(11)? != 0,
                    video_files: Vec::new(),
                },
            ))
        })
        .map_err(|error| format!("작품 정보 조회에 실패했습니다: {error}"))?;

    let mut metadata = HashMap::new();
    for row in rows {
        let (path, mut item) =
            row.map_err(|error| format!("작품 정보 변환에 실패했습니다: {error}"))?;
        item.tags = tags_by_path.remove(&path).unwrap_or_default();
        metadata.insert(path, item);
    }

    Ok(metadata)
}

fn load_tags_by_path(
    connection: &Connection,
    root_path: &str,
) -> Result<HashMap<String, Vec<String>>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT item_tags.item_path, tags.name
            FROM tags
            JOIN item_tags ON item_tags.tag_id = tags.id
            JOIN items ON items.path = item_tags.item_path
            WHERE items.root_path = ?1
            ORDER BY item_tags.item_path, tags.name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("태그 조회 준비에 실패했습니다: {error}"))?;

    let rows = statement
        .query_map(params![root_path], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("태그 조회에 실패했습니다: {error}"))?;

    let mut tags_by_path: HashMap<String, Vec<String>> = HashMap::new();
    for row in rows {
        let (path, tag) = row.map_err(|error| format!("태그 변환에 실패했습니다: {error}"))?;
        tags_by_path.entry(path).or_default().push(tag);
    }
    Ok(tags_by_path)
}

fn enrich_categories(categories: &mut [CategoryNode], metadata: &HashMap<String, ScannedItem>) {
    for category in categories {
        for item in &mut category.items {
            enrich_item(item, metadata);
        }
        enrich_categories(&mut category.children, metadata);
    }
}

fn enrich_item(item: &mut ScannedItem, metadata: &HashMap<String, ScannedItem>) {
    if let Some(stored) = metadata.get(&item.path) {
        item.favorite = stored.favorite;
        item.rating = stored.rating;
        item.notes.clone_from(&stored.notes);
        item.custom_thumbnail_path
            .clone_from(&stored.custom_thumbnail_path);
        item.tags.clone_from(&stored.tags);
        item.open_count = stored.open_count;
        item.last_opened_at.clone_from(&stored.last_opened_at);
        item.missing = stored.missing;
        if item.item_type == "video" && item.video_files.is_empty() {
            item.video_files = discover_video_files(std::path::Path::new(&item.path));
        }
    }
}

fn load_user_categories(
    connection: &Connection,
    root_path: &str,
) -> Result<Vec<UserCategory>, String> {
    let mut statement = connection
        .prepare(
            r#"
            SELECT id, name
            FROM user_categories
            WHERE root_path = ?1
            ORDER BY sort_order, name COLLATE NOCASE
            "#,
        )
        .map_err(|error| format!("사용자 카테고리 조회 준비에 실패했습니다: {error}"))?;

    let rows = statement
        .query_map(params![root_path], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("사용자 카테고리 조회에 실패했습니다: {error}"))?;

    let mut categories = Vec::new();
    for row in rows {
        let (id, name) =
            row.map_err(|error| format!("사용자 카테고리 변환에 실패했습니다: {error}"))?;
        categories.push(UserCategory {
            id,
            name,
            item_paths: Vec::new(),
        });
    }

    drop(statement);
    let category_indexes = categories
        .iter()
        .enumerate()
        .map(|(index, category)| (category.id, index))
        .collect::<HashMap<_, _>>();
    let mut item_statement = connection
        .prepare(
            r#"
            SELECT user_category_items.category_id, user_category_items.item_path
            FROM user_category_items
            JOIN user_categories ON user_categories.id = user_category_items.category_id
            WHERE user_categories.root_path = ?1
            ORDER BY user_category_items.category_id, user_category_items.item_path
            "#,
        )
        .map_err(|error| format!("사용자 카테고리 작품 조회 준비에 실패했습니다: {error}"))?;
    let item_rows = item_statement
        .query_map(params![root_path], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| format!("사용자 카테고리 작품 조회에 실패했습니다: {error}"))?;
    for row in item_rows {
        let (category_id, item_path) =
            row.map_err(|error| format!("사용자 카테고리 작품 변환에 실패했습니다: {error}"))?;
        if let Some(index) = category_indexes.get(&category_id) {
            categories[*index].item_paths.push(item_path);
        }
    }

    Ok(categories)
}

fn normalize_tags(tags: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(tags.len());
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() || !seen.insert(tag.to_lowercase()) {
            continue;
        }
        normalized.push(tag.to_string());
    }
    normalized
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value.and_then(|text| {
        let text = text.trim();
        (!text.is_empty()).then(|| text.to_string())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::create_schema;

    fn sample_item(path: &str) -> ScannedItem {
        ScannedItem {
            id: path.to_owned(),
            title: "Sample".to_owned(),
            path: path.to_owned(),
            thumbnail_path: None,
            item_type: "folder".to_owned(),
            file_count: 1,
            favorite: false,
            rating: None,
            notes: String::new(),
            custom_thumbnail_path: None,
            tags: Vec::new(),
            open_count: 0,
            last_opened_at: None,
            missing: false,
            video_files: Vec::new(),
        }
    }

    #[test]
    fn snapshot_round_trip_preserves_tags_and_user_categories() {
        let mut connection = Connection::open_in_memory().expect("open in-memory database");
        connection
            .execute_batch("PRAGMA foreign_keys = ON")
            .unwrap();
        create_schema(&connection).expect("create schema");
        let root = "C:\\HEART";
        let item_path = "C:\\HEART\\Sample";
        let scan = ScanResult {
            root_path: root.to_owned(),
            categories: Vec::new(),
            unclassified_items: vec![sample_item(item_path)],
            total_items: 1,
        };

        save_scan(&mut connection, &scan).expect("save scan");
        connection
            .execute(
                "UPDATE items SET detected_thumbnail_path = 'detected.jpg' WHERE path = ?1",
                [item_path],
            )
            .expect("set detected thumbnail");
        assert_eq!(
            load_thumbnail_path(&connection, item_path)
                .unwrap()
                .as_deref(),
            Some("detected.jpg")
        );
        update_metadata(
            &mut connection,
            root,
            MetadataUpdate {
                path: item_path.to_owned(),
                favorite: true,
                rating: Some(5),
                notes: "note".to_owned(),
                custom_thumbnail_path: None,
                tags: vec!["topic:Travel".to_owned(), "TOPIC:travel".to_owned()],
            },
        )
        .expect("update metadata");
        let snapshot = create_user_category(&connection, root, "Best").expect("create category");
        let category_id = snapshot.user_categories[0].id;
        set_item_user_categories(&mut connection, root, item_path, vec![category_id])
            .expect("assign category");

        let snapshot = load_snapshot(&connection, root).expect("load snapshot");
        let item = &snapshot.scan.unclassified_items[0];
        assert!(item.favorite);
        assert_eq!(item.rating, Some(5));
        assert_eq!(item.tags, vec!["topic:Travel"]);
        assert_eq!(snapshot.user_categories[0].item_paths, vec![item_path]);
    }
}
