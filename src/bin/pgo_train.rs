use charist::assets::KJV;
use charist::bibles::load_bible_from_bytes;
use charist::search_index::{BibleIndex, search_results};
use std::error::Error;
use std::hint::black_box;

fn main() -> Result<(), Box<dyn Error>> {
    let bible = load_bible_from_bytes(black_box(KJV))?;
    let index = BibleIndex::build(&bible)?;
    let res = search_results(&index, "babylon the great is fallen");
    black_box(index);
    black_box(bible);
    black_box(res);
    Ok(())
}
