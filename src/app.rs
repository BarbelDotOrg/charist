use crate::bibles::{BibleData, load_bible_from_bytes};
use crate::config::{AppConfig, Bookmark, CopyIncludeReferencePolicy};
use crate::fl;
use crate::footnotes::FootnoteLink;
use crate::references::{Reference, parse_reference};
use crate::update::{Message, VerseMessage};
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

#[derive(Debug, Clone)]
pub(crate) enum Modal {
    Footnote(FootnoteLink),
    Bookmarks,
    Settings,
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

    pub(crate) modal: Option<Modal>, // replaces open_footnote
    pub(crate) verse_popup: Option<(usize, VersePopup)>,
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

    fn update(&mut self, message: Self::Message) -> cosmic::app::Task<Self::Message> {
        CharistApp::update(self, message)
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
            modal: None,
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
                cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::KeyPressed {
                    key,
                    modifiers,
                    ..
                }) => {
                    if modifiers.command()
                        && key == cosmic::iced::keyboard::Key::Character("c".into())
                    {
                        Some(Message::Verse(VerseMessage::CopySelection))
                    } else {
                        None
                    }
                }
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

    fn view(&self) -> Element<'_, Self::Message> {
        view::view(self)
    }

    fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
        Some(Message::WindowCloseRequested)
    }
}

impl CharistApp {
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
}
