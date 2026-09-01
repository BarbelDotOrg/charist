use crate::consts::BIBLE_INDEX_URL;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct IndexedBible {
    pub name: String,
    pub long_name: String,
    pub description: String,
    pub download_links: Vec<String>,
}

// language -> available bibles
pub type BibleCatalog = HashMap<String, Vec<IndexedBible>>;

pub async fn fetch_bible_catalog() -> Result<BibleCatalog, String> {
    reqwest::get(BIBLE_INDEX_URL)
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}
