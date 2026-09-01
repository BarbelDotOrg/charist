use std::collections::HashMap;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct IndexedBible {
    pub name: String,
    pub long_name: String,
    pub description: String,
    pub download_links: Vec<String>,
}

// language to available bibles
pub type BibleIndex = HashMap<String, Vec<IndexedBible>>;

impl BibleIndex {
    pub fn url() -> String {
        "https://barbel.org/charist/index.json".to_string()
    }
}