use tantivy::schema::{Schema, TEXT, STORED, STRING, FAST, Field, IndexRecordOption, TextFieldIndexing, TextOptions};
use tantivy::{Index, IndexReader, IndexWriter, doc, ReloadPolicy};
use tantivy::query::QueryParser;
use log::trace;
use crate::bibles::BibleData;
use crate::debug_utils::trace;

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
            let mut writer: IndexWriter = index.writer(50_000_000)?;

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
            let reader = index.reader_builder()
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

