use cosmic::cosmic_config::{self, CosmicConfigEntry, cosmic_config_derive::CosmicConfigEntry};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub book_key: String,
    pub chapter: usize,
    pub verse_start: Option<usize>,
    pub verse_end: Option<usize>,
    pub label: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CopyIncludeReferencePolicy {
    DoNot,
    Top,
    Bottom,
}

#[derive(Debug, Clone, CosmicConfigEntry, Eq, PartialEq)]
#[version = 0]
pub struct AppConfig {
    pub bible_index: usize,
    pub book_key: Option<String>,
    pub chapter: Option<usize>,
    pub bookmarks: Vec<Bookmark>,

    pub copy_includes_reference_policy: CopyIncludeReferencePolicy,
    pub copy_includes_verse_numbers: bool,
    pub copy_delimitate_with_newline: bool,

    pub show_footnotes: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            bible_index: 0,
            book_key: None,
            chapter: None,
            bookmarks: vec![],

            copy_includes_reference_policy: CopyIncludeReferencePolicy::DoNot,
            copy_includes_verse_numbers: false,
            copy_delimitate_with_newline: true,

            show_footnotes: true,
        }
    }
}
