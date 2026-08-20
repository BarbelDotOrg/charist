use crate::debug_utils::trace;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct BibleData {
    pub meta: Meta,
    pub book_order: Vec<String>,
    pub books: HashMap<String, Book>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Meta {
    pub module: String,
    pub generated_at: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Book {
    pub name: String,
    pub osis_name: String,
    pub abbreviation: String,
    pub chapters: Vec<Vec<Verse>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Verse(pub String, pub Vec<Note>, pub Vec<CrossRef>);

#[derive(Debug, Deserialize, Clone)]
pub struct Note {
    pub offset: usize,
    pub text: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CrossRef {
    pub offset: usize,
    pub references: Vec<String>,
}

impl Verse {
    pub fn text(&self) -> &str {
        &self.0
    }
}

/// Load from an embedded/static byte slice (what you want for `resources::NASB` etc.)
pub fn load_bible_from_bytes(
    bytes: &[u8],
) -> Result<BibleData, ciborium::de::Error<std::io::Error>> {
    trace("Load bible", || ciborium::de::from_reader(bytes))
}
