#![windows_subsystem = "windows"] // no console window on windows

use std::env;
use std::hint::black_box;
use charist::assets::KJV;
use charist::bibles::load_bible_from_bytes;
use charist::run;
use charist::search_index::BibleIndex;

fn main() -> cosmic::iced::Result {
    let args = env::args().collect::<Vec<String>>();
    if let Some(arg) = args.get(1) && arg == "pgo" {
        let bible = load_bible_from_bytes(KJV).unwrap();
        let index = BibleIndex::build(&bible).unwrap();
        black_box(bible);
        black_box(index);
        Ok(())
    } else {
        run()
    }
}
