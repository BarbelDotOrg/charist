use crate::app::{CharistApp, Modal};
use crate::debug_utils::trace;
use crate::update::Message;
use cosmic::widget::{Id, text_input};
use cosmic::{Action, Task};
use std::collections::BTreeSet;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;

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

#[derive(Debug, Clone)]
pub(crate) struct SearchResult {
    pub(crate) book_key: String,
    pub(crate) book_name: String,
    pub(crate) chapter: usize,
    pub(crate) verse: usize,
    pub(crate) snippet: String,
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
        const MAX_RESULTS: usize = 100;

        trace("search", || {
            let (Some(bible_index), query_str) = (&self.bible_index, self.search_query.trim())
            else {
                return Vec::new();
            };

            if query_str.is_empty() {
                return Vec::new();
            }

            let searcher = bible_index.reader.searcher();

            // Set up QueryParser targeting the `text` field
            let query_parser = QueryParser::for_index(&bible_index.index, vec![bible_index.text]);

            // Parse search string into Tantivy query (supports quotes "in the beginning", +/-, AND/OR)
            let query = match query_parser.parse_query(query_str) {
                Ok(q) => q,
                Err(_) => return Vec::new(), // Handle invalid syntax gracefully
            };

            // Remove the `&` borrowing operator in front of TopDocs
            let top_docs =
                match searcher.search(&query, &TopDocs::with_limit(MAX_RESULTS).order_by_score()) {
                    Ok(docs) => docs,
                    Err(_) => return Vec::new(),
                };

            let mut results = Vec::with_capacity(top_docs.len());

            for (_score, doc_address) in top_docs {
                let retrieved_doc: tantivy::TantivyDocument = match searcher.doc(doc_address) {
                    Ok(doc) => doc,
                    Err(_) => continue,
                };

                let book_key = retrieved_doc
                    .get_first(bible_index.book_key)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let book_name = retrieved_doc
                    .get_first(bible_index.book_name)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                let chapter = retrieved_doc
                    .get_first(bible_index.chapter)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;

                let verse = retrieved_doc
                    .get_first(bible_index.verse)
                    .and_then(|v| v.as_u64())
                    .unwrap_or(1) as usize;

                let snippet = retrieved_doc
                    .get_first(bible_index.text)
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string();

                results.push(SearchResult {
                    book_key,
                    book_name,
                    chapter,
                    verse,
                    snippet,
                });
            }

            results
        })
    }
}
