use crate::app::{CharistApp, Modal, VersePopup};
use crate::config::Bookmark;
use crate::update::Message;
use cosmic::Task;

#[derive(Debug, Clone)]
pub enum BookmarkMessage {
    Toggle,
    Add,
    SaveNote,
    JumpTo(usize),
    Remove(usize),
}

impl CharistApp {
    pub(crate) fn update_bookmark(
        &mut self,
        message: BookmarkMessage,
    ) -> Task<cosmic::Action<Message>> {
        match message {
            BookmarkMessage::Toggle => {
                self.modal = match self.modal {
                    Some(Modal::Bookmarks) => None,
                    _ => Some(Modal::Bookmarks),
                };
            }
            BookmarkMessage::Add => {
                self.push_bookmark(None);
                self.verse_popup = None;
            }
            BookmarkMessage::SaveNote => {
                if let Some((_, VersePopup::Note(text))) = self.verse_popup.take() {
                    let label = if text.trim().is_empty() {
                        None
                    } else {
                        Some(text)
                    };
                    self.push_bookmark(label);
                }
            }
            BookmarkMessage::JumpTo(idx) => {
                if let Some(bookmark) = self.config.bookmarks.get(idx).cloned() {
                    if let Some(bible) = &self.bible {
                        if bible.books.contains_key(&bookmark.book_key) {
                            self.book_key = Some(bookmark.book_key);
                            self.chapter = Some(bookmark.chapter);
                            match (bookmark.verse_start, bookmark.verse_end) {
                                (Some(s), Some(e)) => {
                                    self.selected_verses = (s..=e).collect();
                                    self.selection_anchor = Some(s);
                                }
                                _ => self.clear_selection(),
                            }
                        }
                    }
                }
                self.modal = None;
            }
            BookmarkMessage::Remove(idx) => {
                if idx < self.config.bookmarks.len() {
                    self.config.bookmarks.remove(idx);
                }
            }
        }
        Task::none()
    }

    pub(crate) fn push_bookmark(&mut self, label: Option<String>) {
        if let (Some(book_key), Some(chapter)) = (&self.book_key, self.chapter) {
            let (verse_start, verse_end) = if !self.selected_verses.is_empty() {
                let start = *self.selected_verses.iter().next().unwrap();
                let end = *self.selected_verses.iter().last().unwrap();
                (Some(start), Some(end))
            } else {
                (None, None)
            };

            self.config.bookmarks.push(Bookmark {
                book_key: book_key.clone(),
                chapter,
                verse_start,
                verse_end,
                label,
            });
        }
    }
}
