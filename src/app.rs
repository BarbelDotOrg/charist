use crate::bibles::BibleData;
use crate::config::AppConfig;
use crate::consts::APP_ID;
use crate::footnotes::FootnoteLink;
use crate::library::catalog::BibleCatalog;
use crate::library::library::{
    InstalledBible, ensure_default_bible_installed, list_installed, load_bible_from_disk,
};
use crate::search_index::BibleIndex;
use crate::update::{Message, SearchMessage, VerseMessage};
use crate::view; // no-op import guard removed below if unused
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::iced::Subscription;
use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::window::Id;
use cosmic::{Application, ApplicationExt, Core, Element, SingleThreadExecutor, Task};
use std::collections::BTreeSet;

pub(crate) struct BibleOption {
    pub(crate) name: &'static str,
    pub(crate) bytes: &'static [u8],
}

#[derive(Debug, Clone)]
pub(crate) enum Modal {
    Footnote(FootnoteLink),
    Bookmarks,
    Settings,
    Search,
    BibleManagement,
}

#[derive(Debug, Clone)]
pub(crate) enum VersePopup {
    Menu,
    Note(String),
}

pub struct CharistApp {
    pub(crate) core: Core,
    pub(crate) config: AppConfig,

    pub(crate) installed_bibles: Vec<InstalledBible>,
    pub(crate) remote_catalog: Option<BibleCatalog>,
    pub(crate) catalog_loading: bool,
    pub(crate) downloading: Option<String>,
    pub(crate) download_error: Option<String>,

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

    pub(crate) search_query: String,
    pub(crate) bible_index: Option<BibleIndex>,
}

impl Application for CharistApp {
    type Executor = SingleThreadExecutor;
    type Flags = ();
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

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
            .map(|context| AppConfig::get_entry(&context).unwrap_or_else(|(_e, c)| c))
            .unwrap_or_default();

        if let Err(err) = ensure_default_bible_installed() {
            eprintln!("failed to write default bible to disk: {err}");
        }

        let installed = list_installed();

        let mut app = CharistApp {
            core,
            config: config.clone(),
            installed_bibles: installed.clone(),
            remote_catalog: None,
            catalog_loading: false,
            downloading: None,
            download_error: None,
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
            search_query: String::new(),
            bible_index: None,
        };

        let name_to_load = config
            .selected_bible
            .clone()
            .filter(|n| installed.iter().any(|b| &b.name == n))
            .or_else(|| installed.first().map(|b| b.name.clone()));

        if let Some(name) = name_to_load {
            if let Some(data) = load_bible_from_disk(&name) {
                app.bible_index = crate::search_index::BibleIndex::build(&data).ok();
                app.bible = Some(data);
                app.config.selected_bible = Some(name);

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
                    } else if modifiers.command()
                        && key == cosmic::iced::keyboard::Key::Character("f".into())
                    {
                        Some(Message::Search(SearchMessage::Toggle))
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
