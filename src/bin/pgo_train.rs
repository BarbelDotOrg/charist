use std::error::Error;
use std::hint::black_box;
use charist::assets::KJV;
use charist::bibles::load_bible_from_bytes;
use charist::search_index::BibleIndex;

fn main() -> Result<(), Box<dyn Error>> {
    let bible = load_bible_from_bytes(black_box(KJV))?;
    let index = BibleIndex::build(&bible)?;
    black_box(&index);
    black_box(&bible);
    Ok(())
}