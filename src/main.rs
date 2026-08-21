mod assets;
pub mod bibles;
mod references;
mod debug_utils;

use crate::bibles::{BibleData, Verse, load_bible_from_bytes};
use crate::references::{Reference, parse_reference};
use cosmic::app::Settings;
use cosmic::iced::keyboard::Modifiers;
use cosmic::iced::widget::{mouse_area, rich_text, span, stack};
use cosmic::iced::widget::text::Span as RichSpan;
use cosmic::iced::{Alignment, Background, Border, Color, Length, Subscription};
use cosmic::widget::{
    self, column, container, divider, dropdown, row, scrollable, text, text_input,
};
use cosmic::{Application, ApplicationExt, Core, Element, SingleThreadExecutor, Task};
use std::collections::BTreeSet;

// NOTE: `rich_text`/`span`/`Span` are the iced 0.13-style rich-text primitives,
// re-exported here through `cosmic::iced::widget` the same way this file already
// pulls in `mouse_area`. Depending on the exact libcosmic/iced version you're
// pinned to, the import paths above may need a small tweak (e.g. the module
// could be `cosmic::iced_widget::text` instead) — the shapes of the calls below
// should stay the same either way.

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

/// A single in-text marker (note or cross-ref) with its byte offset into the
/// verse's plain text, ready to be numbered in reading order.
enum FootnoteMarker {
    Note(String),
    CrossRef(Vec<String>),
}

/// The payload carried by a clicked superscript, used both as the rich-text
/// `Link` type and as the data shown in the popup.
#[derive(Debug, Clone)]
enum FootnoteLink {
    Note { number: usize, text: String },
    CrossRef { number: usize, refs: Vec<String> },
}

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

    show_footnotes: bool,
    open_footnote: Option<FootnoteLink>,
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
    FootnoteClicked(FootnoteLink),
    CloseFootnote,
    ToggleFootnotes(bool),
    CrossRefClicked(String),
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
            show_footnotes: true,
            open_footnote: None,
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
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Self::Message> {
        let content = column![
        self.view_controls(),
        self.view_error_banner(),
        self.view_verses(),
    ]
            .spacing(16)
            .padding(20)
            .width(Length::Fill)
            .height(Length::Fill);

        let base: Element<'_, Message> = widget::container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        if let Some(link) = &self.open_footnote {
            stack![base, self.view_footnote_popup(link)].into()
        } else {
            base
        }
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
        self.open_footnote = None;

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

    fn view_error_banner(&self) -> Element<'_, Message> {
        match &self.reference_error {
            Some(err) => container(text::caption(err.clone()))
                .padding(10)
                .width(Length::Fill)
                .style(error_banner_style)
                .into(),
            None => widget::Space::new().width(0.0).height(Length::Shrink).into(),
        }
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
        let reference_input =
            text_input("e.g. John 3:16, Gen 1:1-5, Romans 8, Apocalypse", &self.reference_text)
                .id(widget::Id::new("reference_input"))
                .on_input(Message::ReferenceInputChanged)
                .on_submit(|_| Message::ReferenceSubmitted)
                .width(Length::Fill);

        let footnotes_toggle = column![
        text::caption("Footnotes"),
        row![
            widget::checkbox(self.show_footnotes).on_toggle(Message::ToggleFootnotes),
            text::body("Show notes & cross-refs"),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    ]
            .spacing(6)
            .width(Length::FillPortion(2));

        let bar = row![
        self.labeled_field("Reference", reference_input.into(), 4),
        self.labeled_field("Translation", self.view_bible_dropdown(), 2),
        self.labeled_field("Book", self.view_book_dropdown(), 3),
        self.labeled_field("Chapter", self.view_chapter_dropdown(), 1),
        footnotes_toggle,
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

        let mut verse_list = column![].spacing(4);
        for (i, verse) in verses.iter().enumerate() {
            let verse_num = i + 1;
            let is_selected = self.selected_verses.contains(&verse_num);

            let verse_row = row![
                container(text::caption(verse_num.to_string()))
                    .width(Length::Fixed(32.0))
                    .align_x(Alignment::End),
                self.view_verse_text(verse),
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

        let body =
            scrollable(verse_list.padding([4, 4])).height(Length::Fill)
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill);

        container(body)
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Renders a verse's text. When footnotes are enabled and the verse has
    /// notes/cross-refs, the text is built as rich text with a numbered,
    /// clickable superscript inserted at each marker's offset. Otherwise it
    /// falls back to a plain text widget.
    fn view_verse_text<'a>(&self, verse: &'a Verse) -> Element<'a, Message> {
        let text_str = verse.text();

        if !self.show_footnotes {
            return text::body(text_str).into();
        }

        let markers = collect_footnote_markers(verse);
        if markers.is_empty() {
            return text::body(text_str).into();
        }

        let mut spans: Vec<RichSpan<'a, FootnoteLink>> = Vec::new();
        let mut cursor = 0usize;
        let mut note_count = 0usize;
        let mut cross_count = 0usize;

        for (offset, marker) in markers.into_iter() {
            let offset = snap_to_word_end(text_str, offset);

            if offset > cursor {
                spans.push(span(&text_str[cursor..offset]));
            }

            let (marker_text, color, link) = match marker {
                FootnoteMarker::Note(t) => {
                    note_count += 1;
                    (
                        to_superscript(note_count),
                        Color::from_rgb(0.25, 0.5, 0.95),
                        FootnoteLink::Note { number: note_count, text: t },
                    )
                }
                FootnoteMarker::CrossRef(r) => {
                    cross_count += 1;
                    (
                        to_superscript_letter(cross_count),
                        Color::from_rgb(0.15, 0.6, 0.35),
                        FootnoteLink::CrossRef { number: cross_count, refs: r },
                    )
                }
            };

            spans.push(span(marker_text).color(color).link(link));

            cursor = offset;
        }

        if cursor < text_str.len() {
            spans.push(span(&text_str[cursor..]));
        }

        rich_text(spans)
            .on_link_click(Message::FootnoteClicked)
            .into()
    }

    /// A modal-style overlay showing the content of whichever footnote was
    /// clicked. Clicking the backdrop or "Close" dismisses it; clicking a
    /// cross-reference navigates there and dismisses it too.
    fn view_footnote_popup(&self, link: &FootnoteLink) -> Element<'_, Message> {
        let (title, body): (String, Element<'_, Message>) = match link {
            FootnoteLink::Note { number, text } => (
                format!("Note {}", to_superscript(*number)),
                text::body(text.clone()).into(),
            ),
            FootnoteLink::CrossRef { number, refs } => {
                let mut list = column![].spacing(6);
                for r in refs {
                    list = list.push(self.view_cross_ref_item(r));
                }
                (
                    format!("Cross reference {}", to_superscript_letter(*number)),
                    list.into(),
                )
            }
        };

        let card_content = column![
            row![
                text::title4(title),
                // horizontal_space(),
                widget::button::text("Close").on_press(Message::CloseFootnote),
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill),
            divider::horizontal::default(),
            body,
        ]
            .spacing(10)
            .padding(16)
            .width(Length::Fixed(380.0));

        let card = container(card_content).class(cosmic::theme::Container::Card);

        let centered = container(card)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill);

        mouse_area(
            container(centered)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(backdrop_style),
        )
            .on_press(Message::CloseFootnote)
            .into()
    }

    /// One clickable row inside a cross-reference popup: the resolved label
    /// ("Genesis 1:1") plus a short preview of the target verse when it can
    /// be resolved against the currently loaded bible, falling back to the
    /// raw reference text otherwise. Clicking it navigates there.
    fn view_cross_ref_item(&self, raw: &str) -> Element<'_, Message> {
        let (label, snippet) = self
            .preview_for_reference(raw)
            .unwrap_or_else(|| (raw.to_string(), String::new()));

        let mut item = column![text::body(label)].spacing(2);
        if !snippet.is_empty() {
            item = item.push(text::caption(snippet));
        }

        let clickable = container(item)
            .padding(10)
            .width(Length::Fill)
            .style(cross_ref_item_style);

        mouse_area(clickable)
            .on_press(Message::CrossRefClicked(raw.to_string()))
            .into()
    }

    /// Resolve a raw cross-reference string against the loaded bible into a
    /// display label and a short preview snippet of the target verse. Read-
    /// only counterpart to `apply_reference` — doesn't touch app state.
    fn preview_for_reference(&self, raw: &str) -> Option<(String, String)> {
        let bible = self.bible.as_ref()?;
        let reference = parse_reference(raw)?;

        let query = reference.osis_book.trim().to_lowercase();
        let book_key = bible.book_order.iter().find(|k| {
            bible
                .books
                .get(*k)
                .map(|b| {
                    b.osis_name.to_lowercase() == query
                        || b.name.to_lowercase() == query
                        || b.abbreviation.to_lowercase() == query
                })
                .unwrap_or(false)
        })?;
        let book = bible.books.get(book_key)?;

        let chapter = reference.chapter.unwrap_or(1);
        let chapter_verses = book.chapters.get(chapter.checked_sub(1)?)?;
        let start = reference.start_verse.unwrap_or(1);
        let verse = chapter_verses.get(start.checked_sub(1)?)?;

        let label = match (reference.start_verse, reference.end_verse) {
            (Some(s), Some(e)) if e > s => format!("{} {}:{}-{}", book.name, chapter, s, e),
            (Some(s), _) => format!("{} {}:{}", book.name, chapter, s),
            (None, _) => format!("{} {}", book.name, chapter),
        };

        const MAX_CHARS: usize = 110;
        let full = verse.text();
        let snippet = if full.chars().count() > MAX_CHARS {
            let mut s: String = full.chars().take(MAX_CHARS).collect();
            s.push('…');
            s
        } else {
            full.to_string()
        };

        Some((label, snippet))
    }

    fn update_title(&mut self) -> cosmic::app::Task<Message> {
        if let Some(id) = self.core.main_window_id() {
            self.set_window_title("Charist".into(), id)
        } else {
            Task::none()
        }
    }
}

/// Combine a verse's notes and cross-refs into a single, offset-ordered list.
/// The resulting position in the list is what gives each marker its number.
fn collect_footnote_markers(verse: &Verse) -> Vec<(usize, FootnoteMarker)> {
    let mut items: Vec<(usize, FootnoteMarker)> = Vec::new();

    for note in &verse.1 {
        items.push((note.offset, FootnoteMarker::Note(note.text.clone())));
    }
    for cross_ref in &verse.2 {
        items.push((
            cross_ref.offset,
            FootnoteMarker::CrossRef(cross_ref.references.clone()),
        ));
    }

    items.sort_by_key(|(offset, _)| *offset);
    items
}

/// If `idx` lands inside a word (non-whitespace on both sides), push it
/// forward to the end of that word so the marker doesn't split it. Offsets
/// that already fall on whitespace/punctuation are left alone.
fn snap_to_word_end(s: &str, idx: usize) -> usize {
    let idx = clamp_to_char_boundary(s, idx.min(s.len()));

    let before_is_word = s[..idx].chars().next_back().is_some_and(|c| !c.is_whitespace());
    let after_is_word = s[idx..].chars().next().is_some_and(|c| !c.is_whitespace());

    if before_is_word && after_is_word {
        match s[idx..].find(char::is_whitespace) {
            Some(rel) => idx + rel,
            None => s.len(),
        }
    } else {
        idx
    }
}

fn clamp_to_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx > s.len() {
        idx = s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn to_superscript(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            other => other,
        })
        .collect()
}

fn to_superscript_letter(n: usize) -> String {
    const LETTERS: [char; 26] = [
        'ᵃ', 'ᵇ', 'ᶜ', 'ᵈ', 'ᵉ', 'ᶠ', 'ᵍ', 'ʰ', 'ⁱ', 'ʲ', 'ᵏ', 'ˡ', 'ᵐ', 'ⁿ', 'ᵒ', 'ᵖ', 'q', 'ʳ',
        'ˢ', 'ᵗ', 'ᵘ', 'ᵛ', 'ʷ', 'ˣ', 'ʸ', 'ᶻ',
    ];
    if n >= 1 && n <= LETTERS.len() {
        LETTERS[n - 1].to_string()
    } else {
        format!("({n})")
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

fn backdrop_style(_theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
        ..Default::default()
    }
}

fn cross_ref_item_style(theme: &cosmic::Theme) -> container::Style {
    let cosmic = theme.cosmic();
    container::Style {
        background: Some(Background::Color(cosmic.palette.neutral_3.into())),
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