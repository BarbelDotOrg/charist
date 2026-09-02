use crate::app::CharistApp;
use crate::library::catalog::{BibleCatalog, fetch_bible_catalog};
use crate::library::library::{
    delete_installed, download_bible, list_installed, load_bible_from_disk,
};
use crate::update::Message;
use cosmic::Task;

#[derive(Debug, Clone)]
pub enum BibleMessage {
    SelectInstalled(String),
    FetchCatalog,
    CatalogLoaded(Result<BibleCatalog, String>),
    Download(String, String), // name, url
    DownloadFinished(String, Result<(), String>),
    Delete(String),
    SelectBookIndex(usize),
    SelectChapterIndex(usize),
}

impl CharistApp {
    pub(crate) fn update_bible(&mut self, message: BibleMessage) -> Task<cosmic::Action<Message>> {
        match message {
            BibleMessage::SelectInstalled(name) => self.select_installed_bible(name),

            BibleMessage::SelectBookIndex(idx) => {
                if let Some(bible) = &self.bible {
                    if let Some(key) = bible.book_order.get(idx) {
                        self.book_key = Some(key.clone());
                        self.chapter = None;
                        self.clear_selection();
                        self.modal = None;
                    }
                }
                Task::none()
            }

            BibleMessage::SelectChapterIndex(idx) => {
                self.chapter = Some(idx + 1);
                self.clear_selection();
                self.modal = None;
                Task::none()
            }

            BibleMessage::FetchCatalog => {
                self.catalog_loading = true;
                self.download_error = None;
                return cosmic::Task::perform(fetch_bible_catalog(), |res| {
                    cosmic::action::app(Message::Bible(BibleMessage::CatalogLoaded(res)))
                });
            }

            BibleMessage::CatalogLoaded(res) => {
                self.catalog_loading = false;
                match res {
                    Ok(catalog) => self.remote_catalog = Some(catalog),
                    Err(e) => self.download_error = Some(e),
                }
                Task::none()
            }

            BibleMessage::Download(name, url) => {
                self.downloading = Some(name.clone());
                self.download_error = None;
                let name_for_result = name.clone();
                return cosmic::Task::perform(download_bible(name, url), move |res| {
                    cosmic::action::app(Message::Bible(BibleMessage::DownloadFinished(
                        name_for_result.clone(),
                        res,
                    )))
                });
            }

            BibleMessage::DownloadFinished(name, res) => {
                self.downloading = None;
                match res {
                    Ok(()) => {
                        self.installed_bibles = list_installed();
                    }
                    Err(e) => {
                        self.download_error = Some(format!("failed to library '{name}': {e}"))
                    }
                }
                Task::none()
            }

            BibleMessage::Delete(name) => {
                if let Err(e) = delete_installed(&name) {
                    self.download_error = Some(format!("failed to delete '{name}': {e}"));
                }
                self.installed_bibles = list_installed();

                if self.config.selected_bible.as_deref() == Some(name.as_str()) {
                    self.bible = None;
                    self.bible_index = None;
                    self.book_key = None;
                    self.chapter = None;
                    self.config.selected_bible = None;
                    self.clear_selection();
                    self.modal = None;
                    // persist config here, same as your existing save-on-change path
                }
                Task::none()
            }
        }
    }
}

impl CharistApp {
    fn select_installed_bible(&mut self, name: String) -> cosmic::app::Task<Message> {
        match load_bible_from_disk(&name) {
            Some(data) => match crate::search_index::BibleIndex::build(&data) {
                Ok(index) => {
                    self.bible_index = Some(index);
                    self.bible = Some(data);
                    self.config.selected_bible = Some(name);
                    self.book_key = None;
                    self.chapter = None;
                    self.clear_selection();
                    self.modal = None;
                    // persist config here, same as your existing save-on-change path
                }
                Err(err) => eprintln!("failed to build search index for '{name}': {err}"),
            },
            None => eprintln!("failed to load bible '{name}' from disk"),
        }
        Task::none()
    }
}
