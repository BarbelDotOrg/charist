use crate::app::{BIBLE_OPTIONS, CharistApp, Message, Modal, VersePopup};
use crate::bibles::Verse;
use crate::footnotes::{
    FootnoteLink, FootnoteMarker, collect_footnote_markers, snap_to_word_end, to_superscript,
    to_superscript_letter,
};
use crate::style::{backdrop_style, cross_ref_item_style, error_banner_style, verse_style};
use cosmic::Element;
use cosmic::iced::widget::text::Span as RichSpan;
use cosmic::iced::widget::{mouse_area, rich_text, span, stack};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{
    self, button, column, container, divider, dropdown, popover, row, scrollable, text, text_input,
};
use crate::fl;

pub(crate) fn view(app: &CharistApp) -> Element<'_, Message> {
    let content = column![
        app.view_controls(),
        app.view_error_banner(),
        app.view_verses(),
    ]
        .spacing(16)
        .padding(20)
        .width(Length::Fill)
        .height(Length::Fill);

    let base: Element<'_, Message> = widget::container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

    if app.modal.is_some() {
        stack![base, app.view_modal()].into()
    } else {
        base
    }
}

impl CharistApp {
    fn view_reference_bar(&self) -> Element<'_, Message> {
        let input = text_input(
            fl!("reference-placeholder"),
            &self.reference_text,
        )
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
            None => widget::Space::new()
                .width(0.0)
                .height(Length::Shrink)
                .into(),
        }
    }

    fn labeled_field<'a>(
        &'a self,
        label: String,
        field: Element<'a, Message>,
        portion: u16,
    ) -> Element<'a, Message> {
        column![text::caption(label), field]
            .spacing(6)
            .width(Length::FillPortion(portion))
            .into()
    }

    fn view_modal_shell<'a>(
        &'a self,
        title: String,
        body: Element<'a, Message>,
        width: f32,
    ) -> Element<'a, Message> {
        let card_content = column![
            row![
                text::title4(title),
                widget::button::text(fl!("close-button")).on_press(Message::CloseModal),
            ]
            .align_y(Alignment::Center)
            .width(Length::Fill),
            divider::horizontal::default(),
            body,
        ]
            .spacing(10)
            .padding(16)
            .width(Length::Fixed(width));

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
            .on_press(Message::CloseModal)
            .into()
    }

    fn view_controls(&self) -> Element<'_, Message> {
        let reference_input = text_input(
            "e.g. John 3:16, Gen 1:1-5, Romans 8, Apocalypse",
            &self.reference_text,
        )
            .id(widget::Id::new("reference_input"))
            .on_input(Message::ReferenceInputChanged)
            .on_submit(|_| Message::ReferenceSubmitted)
            .width(Length::Fill);

        let bookmarks_button =
            widget::button::icon(widget::icon::from_name("user-bookmarks-symbolic"))
                .on_press(Message::ToggleBookmarks);

        let footnotes_toggle = column![
            row![
                widget::checkbox(self.config.show_footnotes).on_toggle(Message::ToggleFootnotes),
                text::caption(fl!("show-footnotes-label")),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
            .spacing(6)
            .width(Length::FillPortion(2));

        let bar = row![
            self.labeled_field(fl!("label-reference"), reference_input.into(), 4),
            self.labeled_field(fl!("label-translation"), self.view_bible_dropdown(), 2),
            self.labeled_field(fl!("label-book"), self.view_book_dropdown(), 3),
            self.labeled_field(fl!("label-chapter"), self.view_chapter_dropdown(), 1),
            footnotes_toggle,
            bookmarks_button,
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
            return text::body(fl!("pick-translation-first")).into();
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
            return text::body(fl!("pick-book-first")).into();
        };
        let Some(book) = bible.books.get(key) else {
            return text::body(fl!("unknown-book")).into();
        };

        let options: Vec<String> = (1..=book.chapters.len()).map(|n| n.to_string()).collect();
        let selected = self.chapter.map(|c| c - 1);

        dropdown(options, selected, Message::SelectChapterIndex).into()
    }

    fn view_verses(&self) -> Element<'_, Message> {
        let (Some(bible), Some(key), Some(chapter)) = (&self.bible, &self.book_key, self.chapter)
        else {
            let placeholder = container(text::body(fl!("select-translation-book-chapter")))
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
            return text::body(fl!("unknown-book")).into();
        };
        let Some(verses) = book.chapters.get(chapter - 1) else {
            return text::body(fl!("unknown-chapter")).into();
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

            let mouse_wrapped = mouse_area(clickable)
                .on_press(Message::VerseClicked(verse_num))
                .on_right_press(Message::VerseRightClicked(verse_num));

            let is_popup_open = matches!(&self.verse_popup, Some((v, _)) if *v == verse_num);

            let verse_element: Element<'_, Message> = if is_popup_open {
                popover(mouse_wrapped)
                    .popup(self.verse_popup_content())
                    .into()
            } else {
                mouse_wrapped.into()
            };

            verse_list = verse_list.push(verse_element);
        }
        let body = scrollable(verse_list.padding([4, 4]))
            .height(Length::Fill)
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill);
        container(body)
            .class(cosmic::theme::Container::Card)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn view_verse_text<'a>(&self, verse: &'a Verse) -> Element<'a, Message> {
        let text_str = verse.text();

        if !self.config.show_footnotes {
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
                        FootnoteLink::Note {
                            number: note_count,
                            text: t,
                        },
                    )
                }
                FootnoteMarker::CrossRef(r) => {
                    cross_count += 1;
                    (
                        to_superscript_letter(cross_count),
                        Color::from_rgb(0.15, 0.6, 0.35),
                        FootnoteLink::CrossRef {
                            number: cross_count,
                            refs: r,
                        },
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

    fn view_footnote_popup(&self, link: &FootnoteLink) -> Element<'_, Message> {
        let (title, body): (String, Element<'_, Message>) = match link {
            FootnoteLink::Note { number, text } => (
                fl!("note-title", number = to_superscript(*number)),
                text::body(text.clone()).into(),
            ),
            FootnoteLink::CrossRef { number, refs } => {
                let mut list = column![].spacing(6);
                for r in refs {
                    list = list.push(self.view_cross_ref_item(r));
                }
                (
                    fl!("cross-reference-title", number = to_superscript_letter(*number)),
                    list.into(),
                )
            }
        };

        let card_content = column![
            row![
                text::title4(title),
                widget::button::text(fl!("close-button")).on_press(Message::CloseModal),
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
            .on_press(Message::CloseModal)
            .into()
    }

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

    fn preview_for_reference(&self, raw: &str) -> Option<(String, String)> {
        let bible = self.bible.as_ref()?;
        let reference = crate::references::parse_reference(raw)?;

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

        Some((label, verse.text().to_string()))
    }

    fn verse_popup_content(&self) -> Element<'_, Message> {
        let content: Element<'_, Message> = match &self.verse_popup {
            Some((_, VersePopup::Menu)) => column![
                button::text(fl!("copy-button")).on_press(Message::CopySelection),
                button::text(fl!("add-bookmark-button")).on_press(Message::AddBookmark),
                button::text(fl!("add-bookmark-note-button")).on_press(Message::OpenNoteInput),
            ]
                .spacing(4)
                .into(),
            Some((_, VersePopup::Note(text))) => column![
                text_input(fl!("note-placeholder"), text).on_input(Message::NoteTextChanged),
                row![
                    button::suggested(fl!("save-button")).on_press(Message::SaveNoteBookmark),
                    button::standard(fl!("cancel-button")).on_press(Message::CloseVersePopup),
                ]
                .spacing(8),
            ]
                .spacing(8)
                .into(),
            None => column![].into(),
        };

        container(content)
            .padding(12)
            .width(Length::Fixed(220.0))
            .class(cosmic::theme::Container::Card)
            .into()
    }

    fn view_modal(&self) -> Element<'_, Message> {
        match self.modal.as_ref().unwrap() {
            Modal::Footnote(link) => {
                let (title, body): (String, Element<'_, Message>) = match link {
                    FootnoteLink::Note { number, text } => (
                        fl!("note-title", number = to_superscript(*number)),
                        text::body(text.clone()).into(),
                    ),
                    FootnoteLink::CrossRef { number, refs } => {
                        let mut list = column![].spacing(6);
                        for r in refs {
                            list = list.push(self.view_cross_ref_item(r));
                        }
                        (
                            fl!("cross-reference-title", number = to_superscript_letter(*number)),
                            list.into(),
                        )
                    }
                };
                self.view_modal_shell(title, body, 380.0)
            }
            Modal::Bookmarks => self.view_modal_shell(
                fl!("bookmarks-title"),
                self.bookmarks_list_content(),
                380.0,
            ),
        }
    }

    fn bookmarks_list_content(&self) -> Element<'_, Message> {
        if self.config.bookmarks.is_empty() {
            return text::body(fl!("no-bookmarks-yet")).into();
        }

        let mut list = column![].spacing(6);

        for (idx, bm) in self.config.bookmarks.iter().enumerate() {
            let book_name = self
                .bible
                .as_ref()
                .and_then(|b| b.books.get(&bm.book_key))
                .map(|b| b.name.clone())
                .unwrap_or_else(|| bm.book_key.clone());

            let verse_suffix = match (bm.verse_start, bm.verse_end) {
                (Some(s), Some(e)) if s == e => format!(":{s}"),
                (Some(s), Some(e)) => format!(":{s}-{e}"),
                _ => String::new(),
            };

            let mut label_col = column![text::body(format!(
                "{book_name} {}{verse_suffix}",
                bm.chapter
            ))]
                .spacing(2);

            if let Some(label) = &bm.label {
                label_col = label_col.push(text::caption(label.clone()));
            }

            let row_content = row![
                label_col,
                // widget::horizontal_space(),
                widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
                    .on_press(Message::RemoveBookmark(idx)),
            ]
                .align_y(Alignment::Center)
                .spacing(8)
                .width(Length::Fill);

            let clickable = container(row_content)
                .padding(10)
                .width(Length::Fill)
                .style(cross_ref_item_style);

            list = list.push(mouse_area(clickable).on_press(Message::JumpToBookmark(idx)));
        }

        scrollable(list).height(Length::Fixed(320.0)).into()
    }
}
