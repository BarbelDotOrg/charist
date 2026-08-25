use crate::app::{BIBLE_OPTIONS, CharistApp};
use crate::bibles::load_bible_from_bytes;
use crate::update::Message;
use cosmic::Task;

#[derive(Debug, Clone)]
pub enum BibleMessage {
    Select(usize),
    SelectBookIndex(usize),
    SelectChapterIndex(usize),
}

impl CharistApp {
    pub(crate) fn update_bible(&mut self, message: BibleMessage) -> Task<cosmic::Action<Message>> {
        match message {
            BibleMessage::Select(idx) => {
                if let Some(opt) = BIBLE_OPTIONS.get(idx) {
                    match load_bible_from_bytes(opt.bytes) {
                        Ok(data) => {
                            self.selected_bible = Some(idx);
                            self.bible = Some(data);
                            self.book_key = None;
                            self.chapter = None;
                            self.clear_selection();
                            self.modal = None;
                        }
                        Err(err) => eprintln!("failed to load bible '{}': {err}", opt.name),
                    }
                }
            }
            BibleMessage::SelectBookIndex(idx) => {
                if let Some(bible) = &self.bible {
                    if let Some(key) = bible.book_order.get(idx) {
                        self.book_key = Some(key.clone());
                        self.chapter = None;
                        self.clear_selection();
                        self.modal = None;
                    }
                }
            }
            BibleMessage::SelectChapterIndex(idx) => {
                self.chapter = Some(idx + 1);
                self.clear_selection();
                self.modal = None;
            }
        }
        Task::none()
    }
}
