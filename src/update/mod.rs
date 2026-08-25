mod bible;
mod bookmark;
mod reference;
mod search;
mod settings;
mod verse;

pub use bible::BibleMessage;
pub use bookmark::BookmarkMessage;
pub use reference::ReferenceMessage;
pub use search::SearchMessage;
pub use settings::SettingsMessage;
pub use verse::VerseMessage;

use crate::app::CharistApp;
use crate::config::AppConfig;
use cosmic::cosmic_config::{Config, CosmicConfigEntry};
use cosmic::widget::{self, text_input};
use cosmic::{Application, Task};

#[derive(Debug, Clone)]
pub enum Message {
    Bible(BibleMessage),
    Reference(ReferenceMessage),
    Verse(VerseMessage),
    Bookmark(BookmarkMessage),
    Settings(SettingsMessage),
    Search(SearchMessage),

    ModifiersChanged(cosmic::iced::keyboard::Modifiers),
    FocusInput, // Window opened
    CloseModal,
    UpdateConfig(AppConfig),
    WindowCloseRequested,

    NoOp,
}

impl CharistApp {
    /// Top-level dispatcher. `Application::update` in app.rs just calls this.
    pub(crate) fn update(&mut self, message: Message) -> Task<cosmic::Action<Message>> {
        match message {
            Message::Bible(msg) => return self.update_bible(msg),
            Message::Reference(msg) => return self.update_reference(msg),
            Message::Verse(msg) => return self.update_verse(msg),
            Message::Bookmark(msg) => return self.update_bookmark(msg),
            Message::Settings(msg) => self.update_settings(msg),
            Message::Search(msg) => return self.update_search(msg),

            Message::ModifiersChanged(m) => {
                self.modifiers = m;
            }
            Message::FocusInput => {
                return text_input::focus(widget::Id::new("reference_input"));
            }
            Message::CloseModal => {
                self.modal = None;
            }
            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::WindowCloseRequested => {
                self.save_config();
            }
            Message::NoOp => {
                // :D
            }
        }
        Task::none()
    }

    pub(crate) fn clear_selection(&mut self) {
        self.selected_verses.clear();
        self.selection_anchor = None;
    }

    pub(crate) fn save_config(&mut self) {
        self.config.bible_index = self.selected_bible.unwrap_or(0);
        self.config.book_key = self.book_key.clone();
        self.config.chapter = self.chapter;

        if let Ok(context) = Config::new(<CharistApp as Application>::APP_ID, AppConfig::VERSION) {
            if let Err(err) = self.config.write_entry(&context) {
                eprintln!("failed to save config: {err}");
            }
        }
    }
}
