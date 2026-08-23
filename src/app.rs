use crate::bibles::{BibleData, load_bible_from_bytes};
use crate::config::{AppConfig, Bookmark};
use crate::footnotes::FootnoteLink;
use crate::references::{Reference, parse_reference};
use crate::view;
use cosmic::app::Settings as _; // no-op import guard removed below if unused
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::iced::Subscription;
use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::window::Id;
use cosmic::widget::{self, text_input};
use cosmic::{
    Application, ApplicationExt, Core, Element, SingleThreadExecutor, Task, cosmic_config,
};
use std::collections::BTreeSet;
use std::process::exit;

pub(crate) struct BibleOption {
    pub(crate) name: &'static str,
    pub(crate) bytes: &'static [u8],
}

pub(crate) const BIBLE_OPTIONS: &[BibleOption] = &[
    BibleOption {
        name: "NASB",
        bytes: crate::assets::NASB,
    },
    BibleOption {
        name: "CPDV",
        bytes: crate::assets::CPDV,
    },
];

#[derive(Debug, Clone)]
pub(crate) enum VersePopup {
    Menu,
    Note(String),
}

pub struct CharistApp {
    pub(crate) core: Core,
    pub(crate) config: AppConfig,
    pub(crate) selected_bible: Option<usize>,
    pub(crate) bible: Option<BibleData>,
    pub(crate) book_key: Option<String>,
    pub(crate) chapter: Option<usize>,

    pub(crate) modifiers: Modifiers,
    pub(crate) selected_verses: BTreeSet<usize>,
    pub(crate) selection_anchor: Option<usize>,

    pub(crate) reference_text: String,
    pub(crate) reference_error: Option<String>,

    pub(crate) show_footnotes: bool,
    pub(crate) open_footnote: Option<FootnoteLink>,

    pub(crate) verse_popup: Option<(usize, VersePopup)>, // (verse_num, popup state)
}

#[derive(Debug, Clone)]
pub enum Message {
    SelectBible(usize),
    SelectBookIndex(usize),
    SelectChapterIndex(usize),

    VerseClicked(usize),
    VerseRightClicked(usize),

    ModifiersChanged(Modifiers),
    ReferenceInputChanged(String),
    ReferenceSubmitted,
    FocusInput, // Window opened
    FootnoteClicked(FootnoteLink),
    CloseFootnote,
    ToggleFootnotes(bool),
    CrossRefClicked(String),
    UpdateConfig(AppConfig),
    WindowCloseRequested,
    CloseVersePopup,
    CopySelection,
    AddBookmark,
    OpenNoteInput,
    NoteTextChanged(String),
    SaveNoteBookmark,
}

impl Application for CharistApp {
    type Executor = SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = "org.barbel.Charist";

    fn core(&self) -> &Core {
        &self.core
    }
    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(
        core: Core,
        _flags: Self::Flags,
    ) -> (CharistApp, cosmic::Task<cosmic::Action<Message>>) {
        let config = Config::new(Self::APP_ID, AppConfig::VERSION)
            .map(|context| {
                AppConfig::get_entry(&context).unwrap_or_else(|(_errors, config)| config)
            })
            .unwrap_or_default();

        let mut app = CharistApp {
            core,
            config: config.clone(),
            selected_bible: None,
            bible: None,
            book_key: None,
            chapter: None,
            modifiers: Modifiers::default(),
            selected_verses: BTreeSet::new(),
            selection_anchor: None,
            reference_text: String::new(),
            reference_error: None,
            show_footnotes: true,
            open_footnote: None,
            verse_popup: None,
        };

        let bible_opt = BIBLE_OPTIONS
            .get(config.bible_index)
            .unwrap_or_else(|| &BIBLE_OPTIONS[0]);

        match load_bible_from_bytes(bible_opt.bytes) {
            Ok(data) => {
                app.selected_bible = BIBLE_OPTIONS
                    .iter()
                    .position(|b| std::ptr::eq(b, bible_opt));
                app.bible = Some(data);

                // Restore book/chapter only if still valid for this translation.
                if let Some(book_key) = &config.book_key {
                    if let Some(bible) = &app.bible {
                        if let Some(book) = bible.books.get(book_key) {
                            let chapter = config.chapter.unwrap_or(1);
                            if chapter >= 1 && chapter <= book.chapters.len() {
                                app.book_key = Some(book_key.clone());
                                app.chapter = Some(chapter);
                            }
                        }
                    }
                }
            }
            Err(err) => eprintln!("failed to load default bible '{}': {err}", bible_opt.name),
        }

        let command = app.update_title();
        (app, command)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            cosmic::iced::event::listen_with(|event, _status, _id| match event {
                cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::ModifiersChanged(
                    m,
                )) => Some(Message::ModifiersChanged(m)),
                cosmic::iced::Event::Window(cosmic::iced::window::Event::Focused) => {
                    Some(Message::FocusInput)
                }
                cosmic::iced::Event::Window(cosmic::iced::window::Event::CloseRequested) => {
                    Some(Message::WindowCloseRequested)
                }
                _ => None,
            }),
            self.core()
                .watch_config::<AppConfig>(Self::APP_ID)
                .map(|update| Message::UpdateConfig(update.config)),
        ])
    }

    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::SelectBible(idx) => {
                if let Some(opt) = BIBLE_OPTIONS.get(idx) {
                    match load_bible_from_bytes(opt.bytes) {
                        Ok(data) => {
                            self.selected_bible = Some(idx);
                            self.bible = Some(data);
                            self.book_key = None;
                            self.chapter = None;
                            self.clear_selection();
                            self.open_footnote = None;
                        }
                        Err(err) => eprintln!("failed to load bible '{}': {err}", opt.name),
                    }
                }
            }
            Message::SelectBookIndex(idx) => {
                if let Some(bible) = &self.bible {
                    if let Some(key) = bible.book_order.get(idx) {
                        self.book_key = Some(key.clone());
                        self.chapter = None;
                        self.clear_selection();
                        self.open_footnote = None;
                    }
                }
            }
            Message::SelectChapterIndex(idx) => {
                self.chapter = Some(idx + 1);
                self.clear_selection();
                self.open_footnote = None;
            }
            Message::ModifiersChanged(m) => {
                self.modifiers = m;
            }
            Message::VerseClicked(verse_num) => {
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
            Message::ReferenceInputChanged(s) => {
                self.reference_text = s;
                self.reference_error = None;
            }
            Message::ReferenceSubmitted => {
                self.reference_error = None;
                match parse_reference(&self.reference_text) {
                    Some(reference) => {
                        if let Err(err) = self.apply_reference(reference) {
                            self.reference_error = Some(err);
                        }
                    }
                    None => {
                        self.reference_error =
                            Some(format!("Couldn't understand \"{}\"", self.reference_text));
                    }
                }
            }
            Message::FocusInput => {
                return text_input::focus(widget::Id::new("reference_input"));
            }
            Message::FootnoteClicked(link) => {
                self.open_footnote = Some(link);
            }
            Message::CloseFootnote => {
                self.open_footnote = None;
            }
            Message::ToggleFootnotes(enabled) => {
                self.show_footnotes = enabled;
                if !enabled {
                    self.open_footnote = None;
                }
            }
            Message::CrossRefClicked(reference_text) => {
                self.open_footnote = None;
                match parse_reference(&reference_text) {
                    Some(reference) => {
                        if let Err(err) = self.apply_reference(reference) {
                            self.reference_error = Some(err);
                        }
                    }
                    None => {
                        self.reference_error =
                            Some(format!("Couldn't understand \"{reference_text}\""));
                    }
                }
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::WindowCloseRequested => {
                self.save_config();
            }
            Message::VerseRightClicked(verse_num) => {
                if !self.selected_verses.contains(&verse_num) {
                    self.selected_verses = BTreeSet::from([verse_num]);
                    self.selection_anchor = Some(verse_num);
                }
                self.verse_popup = Some((verse_num, VersePopup::Menu));
            }

            Message::CloseVersePopup => {
                self.verse_popup = None;
            }

            Message::OpenNoteInput => {
                if let Some((verse_num, _)) = self.verse_popup {
                    self.verse_popup = Some((verse_num, VersePopup::Note(String::new())));
                }
            }

            Message::NoteTextChanged(s) => {
                if let Some((_, VersePopup::Note(text))) = &mut self.verse_popup {
                    *text = s;
                }
            }

            Message::AddBookmark => {
                self.push_bookmark(None);
                self.verse_popup = None;
            }

            Message::SaveNoteBookmark => {
                if let Some((_, VersePopup::Note(text))) = self.verse_popup.take() {
                    let label = if text.trim().is_empty() {
                        None
                    } else {
                        Some(text)
                    };
                    self.push_bookmark(label);
                }
            }

            Message::CopySelection => {
                self.verse_popup = None;
                // build the verse text from self.selected_verses / self.bible, then:
                // return cosmic::iced::clipboard::write(text);
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        view::view(self)
    }

    fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
        Some(Message::WindowCloseRequested)
    }
}

impl CharistApp {
    pub(crate) fn clear_selection(&mut self) {
        self.selected_verses.clear();
        self.selection_anchor = None;
    }

    /// Resolve a parsed Reference against the currently loaded bible and
    /// update book/chapter/verse-selection state. Returns a human-readable
    /// error instead of failing silently if anything doesn't line up.
    pub(crate) fn apply_reference(&mut self, reference: Reference) -> Result<(), String> {
        let Some(bible) = &self.bible else {
            return Err("Pick a translation first".to_string());
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
            return Err(format!(
                "No book matching \"{}\" in {}",
                reference.osis_book, bible.meta.module
            ));
        };

        let book = bible.books.get(&book_key).expect("just found by key");

        let chapter = reference.chapter.unwrap_or(1);
        if chapter == 0 || chapter > book.chapters.len() {
            return Err(format!("{} has no chapter {}", book.name, chapter));
        }

        let verse_count = book.chapters[chapter - 1].len();
        let (start, end) = match (reference.start_verse, reference.end_verse) {
            (Some(s), Some(e)) => (s, e.max(s)),
            (Some(s), None) => (s, s),
            (None, _) => (0, 0),
        };

        if start > 0 && (start > verse_count || end > verse_count) {
            return Err(format!(
                "{} {} only has {} verses",
                book.name, chapter, verse_count
            ));
        }

        self.book_key = Some(book_key);
        self.chapter = Some(chapter);
        self.open_footnote = None;

        if start > 0 {
            self.selected_verses = (start..=end).collect();
            self.selection_anchor = Some(start);
        } else {
            self.clear_selection();
        }

        Ok(())
    }

    pub(crate) fn update_title(&mut self) -> cosmic::app::Task<Message> {
        if let Some(id) = self.core.main_window_id() {
            #[cfg(target_os = "windows")]
            {
                self.set_window_title("Charist".into())
            }

            #[cfg(not(target_os = "windows"))]
            {
                self.set_window_title("Charist".into(), id)
            }
        } else {
            Task::none()
        }
    }

    fn save_config(&mut self) {
        self.config.bible_index = self.selected_bible.unwrap_or(0);
        self.config.book_key = self.book_key.clone();
        self.config.chapter = self.chapter;

        if let Ok(context) = cosmic_config::Config::new(Self::APP_ID, AppConfig::VERSION) {
            if let Err(err) = self.config.write_entry(&context) {
                eprintln!("failed to save config: {err}");
            }
        }
    }

    fn push_bookmark(&mut self, label: Option<String>) {
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
            // self.save_config();
        }
    }
}
