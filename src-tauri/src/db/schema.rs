use rusqlite::Connection;

pub fn create_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS library_roots (
                path TEXT PRIMARY KEY,
                scan_json TEXT NOT NULL,
                last_scanned_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS items (
                path TEXT PRIMARY KEY,
                root_path TEXT NOT NULL,
                title TEXT NOT NULL,
                item_type TEXT NOT NULL,
                detected_thumbnail_path TEXT,
                file_count INTEGER NOT NULL DEFAULT 0,
                missing INTEGER NOT NULL DEFAULT 0,
                favorite INTEGER NOT NULL DEFAULT 0,
                rating INTEGER,
                notes TEXT NOT NULL DEFAULT '',
                custom_thumbnail_path TEXT,
                open_count INTEGER NOT NULL DEFAULT 0,
                last_opened_at TEXT,
                last_seen_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(root_path) REFERENCES library_roots(path) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_items_root_path
            ON items(root_path);

            CREATE TABLE IF NOT EXISTS tags (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE COLLATE NOCASE
            );

            CREATE TABLE IF NOT EXISTS item_tags (
                item_path TEXT NOT NULL,
                tag_id INTEGER NOT NULL,
                PRIMARY KEY(item_path, tag_id),
                FOREIGN KEY(item_path) REFERENCES items(path) ON DELETE CASCADE,
                FOREIGN KEY(tag_id) REFERENCES tags(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS user_categories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                root_path TEXT NOT NULL,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                UNIQUE(root_path, name COLLATE NOCASE),
                FOREIGN KEY(root_path) REFERENCES library_roots(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS user_category_items (
                category_id INTEGER NOT NULL,
                item_path TEXT NOT NULL,
                PRIMARY KEY(category_id, item_path),
                FOREIGN KEY(category_id) REFERENCES user_categories(id) ON DELETE CASCADE,
                FOREIGN KEY(item_path) REFERENCES items(path) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )
        .map_err(|error| format!("데이터베이스 테이블 생성에 실패했습니다: {error}"))?;

    Ok(())
}
