use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub book_key: String,
    pub chapter: usize,
    pub verse_start: Option<usize>,
    pub verse_end: Option<usize>,
    pub label: Option<String>,
}

#[derive(Debug, Default, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 0]
pub struct AppConfig {
    pub bible_index: usize,
    pub book_key: Option<String>,
    pub chapter: Option<usize>,
    pub bookmarks: Vec<Bookmark>,
}
