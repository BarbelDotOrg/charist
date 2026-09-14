use crate::app::{CharistApp, Modal};
use crate::search_index::{SearchResult, search_results};
use crate::update::Message;
use cosmic::widget::{Id, text_input};
use cosmic::{Action, Task};
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
pub enum SearchMessage {
    Toggle,
    QueryChanged(String),
    ResultClicked {
        book_key: String,
        chapter: usize,
        verse: usize,
    },
}

impl CharistApp {
    pub(crate) fn update_search(&mut self, message: SearchMessage) -> Task<Action<Message>> {
        match message {
            SearchMessage::Toggle => match self.modal {
                Some(Modal::Search) => {
                    self.modal = None;
                }
                _ => {
                    self.modal = Some(Modal::Search);
                    self.search_query.clear();
                    self.verse_popup = None;
                    return text_input::focus(Id::new("search_input"));
                }
            },
            SearchMessage::QueryChanged(q) => {
                self.search_query = q;
            }
            SearchMessage::ResultClicked {
                book_key,
                chapter,
                verse,
            } => {
                if let Some(bible) = &self.bible {
                    if bible.books.contains_key(&book_key) {
                        self.book_key = Some(book_key);
                        self.chapter = Some(chapter);
                        self.selected_verses = BTreeSet::from([verse]);
                        self.selection_anchor = Some(verse);
                    }
                }
                self.modal = None;
            }
        }
        Task::none()
    }
}

impl CharistApp {
    pub(crate) fn search_results(&self) -> Vec<SearchResult> {
        let (Some(bible_index), query_str) = (&self.bible_index, self.search_query.trim()) else {
            return Vec::new();
        };

        if query_str.is_empty() {
            return Vec::new();
        }
        search_results(bible_index, query_str)
    }
}
