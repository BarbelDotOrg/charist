mod assets;
pub mod bibles;
mod references;
mod debug_utils;

use crate::bibles::{BibleData, load_bible_from_bytes};
use crate::references::{Reference, parse_reference};
use cosmic::app::Settings;
use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::widget::mouse_area;
use cosmic::iced::{Alignment, Background, Border, Length, Subscription};
use cosmic::widget::{
    self, column, container, divider, dropdown, row, scrollable, text, text_input,
};
use cosmic::{Application, ApplicationExt, Core, Element, SingleThreadExecutor, Task};
use std::collections::BTreeSet;

struct BibleOption {
    name: &'static str,
    bytes: &'static [u8],
}

const BIBLE_OPTIONS: &[BibleOption] = &[
    BibleOption {
        name: "NASB",
        bytes: assets::NASB,
    },
    BibleOption {
        name: "CPDV",
        bytes: assets::CPDV,
    },
];

struct CharistApp {
    core: Core,
    selected_bible: Option<usize>,
    bible: Option<BibleData>,
    book_key: Option<String>,
    chapter: Option<usize>,

    modifiers: Modifiers,
    selected_verses: BTreeSet<usize>,
    selection_anchor: Option<usize>,

    reference_text: String,
    reference_error: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    SelectBible(usize),
    SelectBookIndex(usize),
    SelectChapterIndex(usize),
    VerseClicked(usize),
    ModifiersChanged(Modifiers),
    ReferenceInputChanged(String),
    ReferenceSubmitted,
    FocusInput, // Window opened
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
        let mut app = CharistApp {
            core,
            selected_bible: None,
            bible: None,
            book_key: None,
            chapter: None,
            modifiers: Modifiers::default(),
            selected_verses: BTreeSet::new(),
            selection_anchor: None,
            reference_text: String::new(),
            reference_error: None,
        };

        if let Some(default) = BIBLE_OPTIONS.first() {
            match load_bible_from_bytes(default.bytes) {
                Ok(data) => {
                    app.selected_bible = Some(0);
                    app.bible = Some(data);
                }
                Err(err) => eprintln!("failed to load default bible '{}': {err}", default.name),
            }
        } else {
            panic!("No bibles available");
        }

        let command = app.update_title();

        (app, command)
    }

    fn subscription(&self) -> Subscription<Message> {
        cosmic::iced::event::listen_with(|event, _status, _id| match event {
            cosmic::iced::Event::Keyboard(cosmic::iced::keyboard::Event::ModifiersChanged(m)) => {
                Some(Message::ModifiersChanged(m))
            }
            cosmic::iced::Event::Window(cosmic::iced::window::Event::Focused) => {
                Some(Message::FocusInput)
            }
            _ => None,
        })
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
                    }
                }
            }
            Message::SelectChapterIndex(idx) => {
                self.chapter = Some(idx + 1);
                self.clear_selection();
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
                // Clear a stale error as soon as the user starts fixing their input.
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
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content = column![
            self.view_reference_bar(),
            self.view_controls(),
            self.view_verses(),
        ]
        .spacing(16)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

impl CharistApp {
    fn clear_selection(&mut self) {
        self.selected_verses.clear();
        self.selection_anchor = None;
    }

    /// Resolve a parsed Reference against the currently loaded bible and
    /// update book/chapter/verse-selection state. Returns a human-readable
    /// error instead of failing silently if anything doesn't line up.
    fn apply_reference(&mut self, reference: Reference) -> Result<(), String> {
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
            (None, _) => (0, 0), // no verse given -> just navigate to the chapter
        };

        if start > 0 && (start > verse_count || end > verse_count) {
            return Err(format!(
                "{} {} only has {} verses",
                book.name, chapter, verse_count
            ));
        }

        self.book_key = Some(book_key);
        self.chapter = Some(chapter);

        if start > 0 {
            self.selected_verses = (start..=end).collect();
            self.selection_anchor = Some(start);
        } else {
            self.clear_selection();
        }

        Ok(())
    }

    fn view_reference_bar(&self) -> Element<'_, Message> {
        let input = text_input("e.g. John 3:16, Gen 1:1-5, Romans 8, Apocalypse", &self.reference_text)
            .id(widget::Id::new("reference_input"))
            .on_input(Message::ReferenceInputChanged)
            .on_submit(|_| Message::ReferenceSubmitted)
            .width(Length::Fill);

        let mut bar = column![input].spacing(6).width(Length::Fill);

        if let Some(err) = &self.reference_error {
            let banner = container(text::caption(err.clone()))
                .padding(10)
                .width(Length::Fill)
                .style(error_banner_style);
            bar = bar.push(banner);
        }

        bar.into()
    }

    fn labeled_field<'a>(
        &'a self,
        label: &'static str,
        field: Element<'a, Message>,
        portion: u16,
    ) -> Element<'a, Message> {
        column![text::caption(label), field]
            .spacing(6)
            .width(Length::FillPortion(portion))
            .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let bar = row![
            self.labeled_field("Translation", self.view_bible_dropdown(), 2),
            self.labeled_field("Book", self.view_book_dropdown(), 3),
            self.labeled_field("Chapter", self.view_chapter_dropdown(), 1),
        ]
        .spacing(24)
        .align_y(Alignment::End)
        .width(Length::Fill);

        container(bar)
            .padding(16)
            .width(Length::Fill)
            .class(cosmic::theme::Container::Card)
            .into()
    }

    fn view_bible_dropdown(&self) -> Element<'_, Message> {
        let options: Vec<String> = BIBLE_OPTIONS.iter().map(|o| o.name.to_string()).collect();
        dropdown(options, self.selected_bible, Message::SelectBible).into()
    }

    fn view_book_dropdown(&self) -> Element<'_, Message> {
        let Some(bible) = &self.bible else {
            return text::body("Pick a translation first").into();
        };

        let options: Vec<String> = bible
            .book_order
            .iter()
            .filter_map(|k| bible.books.get(k).map(|b| b.name.clone()))
            .collect();

        let selected = self
            .book_key
            .as_ref()
            .and_then(|k| bible.book_order.iter().position(|x| x == k));

        dropdown(options, selected, Message::SelectBookIndex).into()
    }

    fn view_chapter_dropdown(&self) -> Element<'_, Message> {
        let (Some(bible), Some(key)) = (&self.bible, &self.book_key) else {
            return text::body("Pick a book first").into();
        };
        let Some(book) = bible.books.get(key) else {
            return text::body("Unknown book").into();
        };

        let options: Vec<String> = (1..=book.chapters.len()).map(|n| n.to_string()).collect();
        let selected = self.chapter.map(|c| c - 1);

        dropdown(options, selected, Message::SelectChapterIndex).into()
    }

    fn view_verses(&self) -> Element<'_, Message> {
        let (Some(bible), Some(key), Some(chapter)) = (&self.bible, &self.book_key, self.chapter)
        else {
            let placeholder = container(text::body("Select a translation, book, and chapter"))
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill);

            return container(placeholder)
                .class(cosmic::theme::Container::Card)
                .width(Length::Fill)
                .height(Length::Fill)
                .into();
        };
        let Some(book) = bible.books.get(key) else {
            return text::body("Unknown book").into();
        };
        let Some(verses) = book.chapters.get(chapter - 1) else {
            return text::body("Unknown chapter").into();
        };

        let header = column![
            text::title3(format!("{} {}", book.name, chapter)),
            text::caption(bible.meta.module.clone()),
        ]
        .spacing(2);

        let mut verse_list = column![].spacing(4);
        for (i, verse) in verses.iter().enumerate() {
            let verse_num = i + 1;
            let is_selected = self.selected_verses.contains(&verse_num);

            let verse_row = row![
                container(text::caption(verse_num.to_string()))
                    .width(Length::Fixed(32.0))
                    .align_x(Alignment::End),
                text::body(verse.text().to_string()),
            ]
            .spacing(14)
            .align_y(Alignment::Start);

            let clickable = container(verse_row)
                .padding([6, 10])
                .width(Length::Fill)
                .style(move |theme: &cosmic::Theme| verse_style(theme, is_selected));

            verse_list =
                verse_list.push(mouse_area(clickable).on_press(Message::VerseClicked(verse_num)));
        }

        let body = column![
            header,
            divider::horizontal::default(),
            scrollable(verse_list.padding([4, 4])).height(Length::Fill),
        ]
        .spacing(16)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

        container(body)
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn update_title(&mut self) -> cosmic::app::Task<Message> {
        if let Some(id) = self.core.main_window_id() {
            self.set_window_title("Charist".into(), id)
        } else {
            Task::none()
        }
    }
}

fn verse_style(theme: &cosmic::Theme, is_selected: bool) -> container::Style {
    let cosmic = theme.cosmic();
    if is_selected {
        container::Style {
            background: Some(Background::Color(cosmic.accent.base.into())),
            border: Border {
                radius: cosmic.radius_s().into(),
                width: 0.0,
                color: Default::default(),
            },
            ..Default::default()
        }
    } else {
        container::Style::default()
    }
}

fn error_banner_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(cosmic.destructive.base.into())),
        border: Border {
            radius: cosmic.radius_s().into(),
            width: 0.0,
            color: Default::default(),
        },
        ..Default::default()
    }
}

fn main() -> cosmic::iced::Result {
    let settings = Settings::default();
    cosmic::app::run::<CharistApp>(settings, ())
}