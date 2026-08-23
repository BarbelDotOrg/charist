use crate::app::CharistApp;
use cosmic::app::Settings;

mod app;
mod assets;
pub mod bibles;
mod config;
mod debug_utils;
mod footnotes;
mod references;
mod state;
mod style;
mod view;

fn main() -> cosmic::iced::Result {
    let settings = Settings::default();
    cosmic::app::run::<CharistApp>(settings, ())
}
