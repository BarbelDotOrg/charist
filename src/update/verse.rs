use crate::app::{CharistApp, Modal, VersePopup};
use crate::config::CopyIncludeReferencePolicy;
use crate::footnotes::FootnoteLink;
use crate::update::Message;
use cosmic::Task;
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub enum VerseMessage {
    Clicked(usize),
    RightClicked(usize),
    ClosePopup,
    OpenNoteInput,
    NoteTextChanged(String),
    CopySelection,
    FootnoteClicked(FootnoteLink),
    ToggleFootnotes(bool),
}

impl CharistApp {
    pub(crate) fn update_verse(&mut self, message: VerseMessage) -> Task<cosmic::Action<Message>> {
        match message {
            VerseMessage::Clicked(verse_num) => {
                if self.modifiers.shift() {
                    if let Some(anchor) = self.selection_anchor {
                        let (lo, hi) = if anchor <= verse_num {
                            (anchor, verse_num)
                        } else {
                            (verse_num, anchor)
                        };
                        self.selected_verses = (lo..=hi).collect();
                    } else {
                        self.selected_verses = BTreeSet::from([verse_num]);
                        self.selection_anchor = Some(verse_num);
                    }
                } else {
                    self.selected_verses = BTreeSet::from([verse_num]);
                    self.selection_anchor = Some(verse_num);
                }
            }
            VerseMessage::RightClicked(verse_num) => {
                if !self.selected_verses.contains(&verse_num) {
                    self.selected_verses = BTreeSet::from([verse_num]);
                    self.selection_anchor = Some(verse_num);
                }
                self.verse_popup = Some((verse_num, VersePopup::Menu));
            }
            VerseMessage::ClosePopup => {
                self.verse_popup = None;
            }
            VerseMessage::OpenNoteInput => {
                if let Some((verse_num, _)) = self.verse_popup {
                    self.verse_popup = Some((verse_num, VersePopup::Note(String::new())));
                }
            }
            VerseMessage::NoteTextChanged(s) => {
                if let Some((_, VersePopup::Note(text))) = &mut self.verse_popup {
                    *text = s;
                }
            }
            VerseMessage::CopySelection => {
                self.verse_popup = None;
                if let Some(text) = self.selection_text() {
                    return cosmic::iced::clipboard::write(text);
                }
            }
            VerseMessage::FootnoteClicked(link) => {
                self.modal = Some(Modal::Footnote(link));
            }
            VerseMessage::ToggleFootnotes(enabled) => {
                self.config.show_footnotes = enabled;
                if !enabled {
                    self.modal = None;
                }
            }
        }
        Task::none()
    }

    pub(crate) fn selection_text(&self) -> Option<String> {
        let book_key = self.book_key.as_ref()?;
        let chapter = self.chapter?;
        let bible = self.bible.as_ref()?;
        let book = bible.books.get(book_key)?;
        let verses = book.chapters.get(chapter - 1)?;
        let start = *self.selected_verses.iter().next()?;
        let end = *self.selected_verses.iter().last()?;

        let reference = if start == end {
            format!("{} {}:{}", book.name, chapter, start)
        } else {
            format!("{} {}:{}-{}", book.name, chapter, start, end)
        };

        let delimiter = if self.config.copy_delimitate_with_newline {
            "\n"
        } else {
            " "
        };

        let body = self
            .selected_verses
            .iter()
            .filter_map(|&num| {
                verses.get(num - 1).map(|v| {
                    if self.config.copy_includes_verse_numbers {
                        format!("{num} {}", v.text())
                    } else {
                        v.text().to_string()
                    }
                })
            })
            .collect::<Vec<_>>()
            .join(delimiter);

        Some(match self.config.copy_includes_reference_policy {
            CopyIncludeReferencePolicy::DoNot => body,
            CopyIncludeReferencePolicy::Top => format!("{reference}\n{body}"),
            CopyIncludeReferencePolicy::Bottom => format!("{body}\n{reference}"),
        })
    }
}
