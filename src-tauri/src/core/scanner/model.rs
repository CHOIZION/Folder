use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub root_path: String,
    pub categories: Vec<CategoryNode>,
    pub unclassified_items: Vec<ScannedItem>,
    pub total_items: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CategoryNode {
    pub id: String,
    pub name: String,
    pub path: String,
    pub children: Vec<CategoryNode>,
    pub items: Vec<ScannedItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScannedItem {
    pub id: String,
    pub title: String,
    pub path: String,
    pub thumbnail_path: Option<String>,
    pub item_type: String,
    pub file_count: usize,
    #[serde(default)]
    pub favorite: bool,
    #[serde(default)]
    pub rating: Option<i64>,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub custom_thumbnail_path: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub open_count: i64,
    #[serde(default)]
    pub last_opened_at: Option<String>,
    #[serde(default)]
    pub missing: bool,
    #[serde(default)]
    pub video_files: Vec<String>,
}
