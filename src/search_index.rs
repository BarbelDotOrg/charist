use crate::bibles::BibleData;
use crate::consts::{MAX_SEARCH_RESULTS, SEARCH_INDEX_MEMORY_BUDGET};
use crate::debug_utils::trace;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{FAST, Field, STORED, STRING, Schema, TEXT, Value};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, doc};

#[derive(Clone)]
pub struct BibleIndex {
    pub index: Index,
    pub reader: IndexReader,
    // Field handles
    pub book_key: Field,
    pub book_name: Field,
    pub chapter: Field,
    pub verse: Field,
    pub text: Field,
}

impl BibleIndex {
    pub fn build(bible: &BibleData) -> Result<Self, tantivy::TantivyError> {
        trace("Build bible index", || {
            let mut schema_builder = Schema::builder();

            // 1. Define fields
            // String/Numeric stored fields for metadata retrieval
            let book_key = schema_builder.add_text_field("book_key", STRING | STORED);
            let book_name = schema_builder.add_text_field("book_name", STRING | STORED);
            let chapter = schema_builder.add_u64_field("chapter", FAST | STORED);
            let verse = schema_builder.add_u64_field("verse", FAST | STORED);

            // Standard English tokenized text for full-text search + stored snippet
            let text = schema_builder.add_text_field("text", TEXT | STORED);

            let schema = schema_builder.build();

            // 2. Create in-memory index
            let index = Index::create_in_ram(schema);

            // 3. Populate index (allocate 50MB RAM buffer for index writer)
            let mut writer: IndexWriter = index.writer(SEARCH_INDEX_MEMORY_BUDGET)?;

            for key in &bible.book_order {
                if let Some(book) = bible.books.get(key) {
                    for (ch_idx, chapter_verses) in book.chapters.iter().enumerate() {
                        for (v_idx, v) in chapter_verses.iter().enumerate() {
                            writer.add_document(doc!(
                                book_key => key.as_str(),
                                book_name => book.name.as_str(),
                                chapter => (ch_idx + 1) as u64,
                                verse => (v_idx + 1) as u64,
                                text => v.text(),
                            ))?;
                        }
                    }
                }
            }

            writer.commit()?;

            // 4. Create reusable reader
            let reader = index
                .reader_builder()
                .reload_policy(ReloadPolicy::OnCommitWithDelay)
                .try_into()?;

            Ok(Self {
                index,
                reader,
                book_key,
                book_name,
                chapter,
                verse,
                text,
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub(crate) book_key: String,
    pub(crate) book_name: String,
    pub(crate) chapter: usize,
    pub(crate) verse: usize,
    pub(crate) snippet: String,
}

pub fn search_results(bible_index: &BibleIndex, query_str: &str) -> Vec<SearchResult> {
    let searcher = bible_index.reader.searcher();

    // Set up QueryParser targeting the `text` field
    let query_parser = QueryParser::for_index(&bible_index.index, vec![bible_index.text]);

    // Parse search string into Tantivy query (supports quotes "in the beginning", +/-, AND/OR)
    let query = match query_parser.parse_query(query_str) {
        Ok(q) => q,
        Err(_) => return Vec::new(), // Handle invalid syntax gracefully
    };

    // Remove the `&` borrowing operator in front of TopDocs
    let top_docs = match searcher.search(
        &query,
        &TopDocs::with_limit(MAX_SEARCH_RESULTS).order_by_score(),
    ) {
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
}
