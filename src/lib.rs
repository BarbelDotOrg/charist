use crate::app::CharistApp;
use cosmic::app::Settings;

pub mod app;
pub mod assets;
pub mod bibles;
pub mod config;
pub mod consts;
pub mod debug_utils;
pub mod footnotes;
pub mod i18n;
pub mod library;
pub mod references;
pub mod search_index;
pub mod state;
pub mod style;
pub mod update;
pub mod view;
pub mod elements;

pub fn run() -> cosmic::iced::Result {
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();
    i18n::init(&requested_languages);

    let settings = Settings::default();
    cosmic::app::run::<CharistApp>(settings, ())
}
