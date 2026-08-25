use crate::app::CharistApp;
use crate::fl;
use crate::references::{Reference, parse_reference};
use crate::update::Message;
use cosmic::Task;

#[derive(Debug, Clone)]
pub enum ReferenceMessage {
    InputChanged(String),
    Submitted,
    CrossRefClicked(String),
}

impl CharistApp {
    pub(crate) fn update_reference(
        &mut self,
        message: ReferenceMessage,
    ) -> Task<cosmic::Action<Message>> {
        match message {
            ReferenceMessage::InputChanged(s) => {
                self.reference_text = s;
                self.reference_error = None;
            }
            ReferenceMessage::Submitted => {
                self.reference_error = None;
                match parse_reference(&self.reference_text) {
                    Some(reference) => {
                        if let Err(err) = self.apply_reference(reference) {
                            self.reference_error = Some(err);
                        }
                    }
                    None => {
                        self.reference_error = Some(fl!(
                            "couldnt-understand",
                            reference = self.reference_text.clone()
                        ));
                    }
                }
            }
            ReferenceMessage::CrossRefClicked(reference_text) => {
                self.modal = None;
                match parse_reference(&reference_text) {
                    Some(reference) => {
                        if let Err(err) = self.apply_reference(reference) {
                            self.reference_error = Some(err);
                        }
                    }
                    None => {
                        self.reference_error =
                            Some(fl!("couldnt-understand", reference = reference_text));
                    }
                }
            }
        }
        Task::none()
    }

    /// Resolve a parsed Reference against the currently loaded bible and
    /// update book/chapter/verse-selection state. Returns a human-readable
    /// error instead of failing silently if anything doesn't line up.
    pub(crate) fn apply_reference(&mut self, reference: Reference) -> Result<(), String> {
        let Some(bible) = &self.bible else {
            return Err(fl!("pick-translation-first"));
        };

        let query = reference.osis_book.trim().to_lowercase();
        let book_key = bible
            .book_order
            .iter()
            .find(|k| {
                bible
                    .books
                    .get(*k)
                    .map(|b| {
                        b.osis_name.to_lowercase() == query
                            || b.name.to_lowercase() == query
                            || b.abbreviation.to_lowercase() == query
                    })
                    .unwrap_or(false)
            })
            .cloned();

        let Some(book_key) = book_key else {
            return Err(fl!(
                "no-book-matching",
                query = reference.osis_book.clone(),
                module = bible.meta.module.clone()
            ));
        };

        let book = bible.books.get(&book_key).expect("just found by key");

        let chapter = reference.chapter.unwrap_or(1);
        if chapter == 0 || chapter > book.chapters.len() {
            return Err(fl!(
                "no-chapter-in-book",
                book = book.name.clone(),
                chapter = chapter
            ));
        }

        let verse_count = book.chapters[chapter - 1].len();
        let (start, end) = match (reference.start_verse, reference.end_verse) {
            (Some(s), Some(e)) => (s, e.max(s)),
            (Some(s), None) => (s, s),
            (None, _) => (0, 0),
        };

        if start > 0 && (start > verse_count || end > verse_count) {
            return Err(fl!(
                "not-enough-verses",
                book = book.name.clone(),
                chapter = chapter,
                count = verse_count
            ));
        }

        self.book_key = Some(book_key);
        self.chapter = Some(chapter);
        self.modal = None;

        if start > 0 {
            self.selected_verses = (start..=end).collect();
            self.selection_anchor = Some(start);
        } else {
            self.clear_selection();
        }

        Ok(())
    }
}
