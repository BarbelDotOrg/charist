use serde::Deserialize;
use std::collections::HashMap;

const BIBLE_INDEX_URL: &str = "https://barbel.org/charist/index.json";

#[derive(Debug, Clone, Deserialize)]
pub struct IndexedBible {
    pub name: String,
    pub long_name: String,
    pub description: String,
    pub download_links: Vec<String>,
}

// language to available bibles
pub type BibleIndex = HashMap<String, Vec<IndexedBible>>;

pub async fn fetch_bible_index() -> Result<BibleIndex, Box<dyn std::error::Error>> {
    Ok(reqwest::get(BIBLE_INDEX_URL).await?.json().await?)
}
