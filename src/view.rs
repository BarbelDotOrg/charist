use crate::app::{CharistApp, Modal, VersePopup};
use crate::bibles::Verse;
use crate::config::CopyIncludeReferencePolicy;
use crate::fl;
use crate::footnotes::{
    FootnoteLink, FootnoteMarker, collect_footnote_markers, snap_to_word_end, to_superscript,
    to_superscript_letter,
};
use crate::library::library::is_installed;
use crate::style::{backdrop_style, cross_ref_item_style, error_banner_style, verse_style};
use crate::update::{
    BibleMessage as BM, BookmarkMessage as BkM, Message, ReferenceMessage as RM,
    SearchMessage as SeM, SettingsMessage as SM, VerseMessage as VM,
};
use cosmic::Element;
use cosmic::iced::widget::text::Span as RichSpan;
use cosmic::iced::widget::{mouse_area, rich_text, span, stack};
use cosmic::iced::{Alignment, Color, Length};
use cosmic::widget::{
    self, button, column, container, divider, dropdown, icon, popover, row, scrollable, text,
    text_input,
};

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
        let input = text_input(fl!("reference-placeholder"), &self.reference_text)
            .id(widget::Id::new("reference_input"))
            .on_input(|s| Message::Reference(RM::InputChanged(s)))
            .on_submit(|_| Message::Reference(RM::Submitted))
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
        // Optional: keep a max width to avoid huge cards on ultra-wide windows.
        max_width: Option<f32>,
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
        .width(Length::Fill); // fill whatever width the portion below gives it

        let mut card = container(card_content).class(cosmic::theme::Container::Card);
        if let Some(mw) = max_width {
            card = card.max_width(mw);
        }

        // Swallow clicks on the card itself so they don't bubble to the
        // backdrop's on_press below and close the modal.
        let card = mouse_area(card).on_press(Message::NoOp);

        // 1 : 6 : 1 portions => center column is 6/8 = 75% of the row's width.
        let sized_row = row![
            widget::Space::new().width(Length::FillPortion(1)),
            container(card).width(Length::FillPortion(6)),
            widget::Space::new().width(Length::FillPortion(1)),
        ]
        .width(Length::Fill);

        let centered = container(sized_row)
            .width(Length::Fill)
            .height(Length::Fill)
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
        .on_input(|s| Message::Reference(RM::InputChanged(s)))
        .on_submit(|_| Message::Reference(RM::Submitted))
        .width(Length::Fill);

        let bookmarks_button =
            widget::button::icon(widget::icon::from_name("user-bookmarks-symbolic"))
                .on_press(Message::Bookmark(BkM::Toggle));

        let bibles_button =
            widget::button::icon(widget::icon::from_name("accessories-dictionary-symbolic"))
                .on_press(Message::OpenModal(Modal::BibleManagement));

        let settings_button =
            widget::button::icon(widget::icon::from_name("preferences-system-symbolic"))
                .on_press(Message::Settings(SM::Toggle));

        let search_button = button::icon(icon::from_name("edit-find-symbolic"))
            .on_press(Message::Search(SeM::Toggle));

        let bar = row![
            self.labeled_field(fl!("label-reference"), reference_input.into(), 4),
            self.labeled_field(fl!("label-translation"), self.view_bible_dropdown(), 2),
            self.labeled_field(fl!("label-book"), self.view_book_dropdown(), 3),
            self.labeled_field(fl!("label-chapter"), self.view_chapter_dropdown(), 1),
            column![
                row![bibles_button, settings_button],
                row![search_button, bookmarks_button]
            ]
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
        if self.installed_bibles.is_empty() {
            return text::caption(fl!("no-bibles-installed-short")).into();
        }
        let options: Vec<String> = self
            .installed_bibles
            .iter()
            .map(|b| b.name.clone())
            .collect();
        let selected = self
            .config
            .selected_bible
            .as_ref()
            .and_then(|name| self.installed_bibles.iter().position(|b| &b.name == name));

        let names: Vec<String> = self
            .installed_bibles
            .iter()
            .map(|b| b.name.clone())
            .collect();
        dropdown(options, selected, move |idx| {
            Message::Bible(BM::SelectInstalled(names[idx].clone()))
        })
        .into()
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

        dropdown(options, selected, |idx| {
            Message::Bible(BM::SelectBookIndex(idx))
        })
        .into()
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

        dropdown(options, selected, |idx| {
            Message::Bible(BM::SelectChapterIndex(idx))
        })
        .into()
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
                .on_press(Message::Verse(VM::Clicked(verse_num)))
                .on_right_press(Message::Verse(VM::RightClicked(verse_num)));

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
            .on_link_click(|link| Message::Verse(VM::FootnoteClicked(link)))
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
            .on_press(Message::Reference(RM::CrossRefClicked(raw.to_string())))
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
                button::text(fl!("copy-button")).on_press(Message::Verse(VM::CopySelection)),
                button::text(fl!("add-bookmark-button")).on_press(Message::Bookmark(BkM::Add)),
                button::text(fl!("add-bookmark-note-button"))
                    .on_press(Message::Verse(VM::OpenNoteInput)),
            ]
            .spacing(4)
            .into(),
            Some((_, VersePopup::Note(text))) => column![
                text_input(fl!("note-placeholder"), text)
                    .on_input(|s| Message::Verse(VM::NoteTextChanged(s))),
                row![
                    button::suggested(fl!("save-button"))
                        .on_press(Message::Bookmark(BkM::SaveNote)),
                    button::standard(fl!("cancel-button")).on_press(Message::Verse(VM::ClosePopup)),
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
                            fl!(
                                "cross-reference-title",
                                number = to_superscript_letter(*number)
                            ),
                            list.into(),
                        )
                    }
                };
                self.view_modal_shell(title, body, None)
            }
            Modal::Bookmarks => self.view_modal_shell(
                fl!("bookmarks-title"),
                self.bookmarks_list_content(),
                Some(560.0),
            ),
            Modal::Settings => {
                self.view_modal_shell(fl!("settings-title"), self.settings_content(), Some(640.0))
            }
            Modal::Search => {
                self.view_modal_shell(fl!("search-title"), self.search_content(), Some(680.0))
            }
            Modal::BibleManagement => self.view_modal_shell(
                fl!("bible-management-title"),
                self.settings_bibles_section(),
                Some(560.0),
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
                widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
                    .on_press(Message::Bookmark(BkM::Remove(idx))),
            ]
            .align_y(Alignment::Center)
            .spacing(8)
            .width(Length::Fill);

            let clickable = container(row_content)
                .padding(10)
                .width(Length::Fill)
                .style(cross_ref_item_style);

            list = list.push(mouse_area(clickable).on_press(Message::Bookmark(BkM::JumpTo(idx))));
        }

        scrollable(list).height(Length::Fixed(320.0)).into()
    }

    fn settings_content(&self) -> Element<'_, Message> {
        let footnotes_row = row![
            widget::toggler(self.config.show_footnotes)
                .on_toggle(|enabled| Message::Verse(VM::ToggleFootnotes(enabled))),
            text::body(fl!("show-footnotes-label")),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let verse_numbers_row = row![
            widget::toggler(self.config.copy_includes_verse_numbers)
                .on_toggle(|enabled| Message::Settings(SM::ToggleCopyVerseNumbers(enabled))),
            text::body(fl!("copy-verse-numbers-label")),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let newline_row = row![
            widget::toggler(self.config.copy_delimitate_with_newline)
                .on_toggle(|enabled| Message::Settings(SM::ToggleCopyDelimiter(enabled))),
            text::body(fl!("copy-newline-label")),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let policy_options = vec![
            fl!("copy-reference-none"),
            fl!("copy-reference-top"),
            fl!("copy-reference-bottom"),
        ];
        let policy_selected = match self.config.copy_includes_reference_policy {
            CopyIncludeReferencePolicy::DoNot => 0,
            CopyIncludeReferencePolicy::Top => 1,
            CopyIncludeReferencePolicy::Bottom => 2,
        };
        let policy_dropdown = dropdown(policy_options, Some(policy_selected), |idx| {
            let policy = match idx {
                1 => CopyIncludeReferencePolicy::Top,
                2 => CopyIncludeReferencePolicy::Bottom,
                _ => CopyIncludeReferencePolicy::DoNot,
            };
            Message::Settings(SM::SetCopyReferencePolicy(policy))
        });

        column![
            text::title4(fl!("settings-general-section")),
            footnotes_row,
            divider::horizontal::default(),
            text::title4(fl!("settings-copy-section")),
            verse_numbers_row,
            newline_row,
            self.labeled_field(fl!("copy-reference-label"), policy_dropdown.into(), 1,),
            divider::horizontal::default(),
        ]
        .spacing(12)
        .width(Length::Fill)
        .into()
    }

    fn search_content(&self) -> Element<'_, Message> {
        let input = text_input(fl!("search-placeholder"), &self.search_query)
            .id(widget::Id::new("search_input"))
            .on_input(|s| Message::Search(SeM::QueryChanged(s)))
            .width(Length::Fill);

        let mut column_body = column![input].spacing(10).width(Length::Fill);

        if self.search_query.trim().is_empty() {
            return column_body.into();
        }

        let results = self.search_results();

        if results.is_empty() {
            column_body = column_body.push(text::caption(fl!("no-search-results")));
            return column_body.into();
        }

        let mut list = column![].spacing(6);
        for r in &results {
            let item = column![
                text::body(format!("{} {}:{}", r.book_name, r.chapter, r.verse)),
                text::caption(r.snippet.clone()),
            ]
            .spacing(2);

            let clickable = container(item)
                .padding(10)
                .width(Length::Fill)
                .style(cross_ref_item_style);

            list = list.push(
                mouse_area(clickable).on_press(Message::Search(SeM::ResultClicked {
                    book_key: r.book_key.clone(),
                    chapter: r.chapter,
                    verse: r.verse,
                })),
            );
        }

        column_body = column_body.push(scrollable(list).height(Length::Fixed(360.0)));
        column_body.into()
    }

    fn settings_bibles_section(&self) -> Element<'_, Message> {
        let mut col = column![].spacing(8).width(Length::Fill);

        if self.installed_bibles.is_empty() {
            col = col.push(text::caption(fl!("no-bibles-installed")));
        }
        for ib in &self.installed_bibles {
            let is_selected = self.config.selected_bible.as_deref() == Some(ib.name.as_str());
            let action: Element<'_, Message> = if is_selected {
                text::caption(fl!("current-bible-label")).into()
            } else {
                button::text(fl!("use-button"))
                    .on_press(Message::Bible(BM::SelectInstalled(ib.name.clone())))
                    .into()
            };
            col = col.push(
                row![
                    container(text::body(ib.name.clone())).width(Length::Fill), // shrinkable text column, wraps instead of pushing
                    container(action).width(Length::Shrink),                    // never squeezed
                    widget::button::icon(widget::icon::from_name("edit-delete-symbolic"))
                        .on_press(Message::Bible(BM::Delete(ib.name.clone()))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }

        col = col.push(divider::horizontal::default());

        match &self.remote_catalog {
            None => {
                let btn: Element<'_, Message> = if self.catalog_loading {
                    text::caption(fl!("loading-label")).into()
                } else {
                    button::suggested(fl!("browse-bibles-button"))
                        .on_press(Message::Bible(BM::FetchCatalog))
                        .into()
                };
                col = col.push(btn);
            }
            Some(catalog) => {
                for (lang, entries) in catalog {
                    col = col.push(text::title4(lang.clone()));
                    for entry in entries {
                        let already = is_installed(&entry.name);
                        let action: Element<'_, Message> = if already {
                            text::caption(fl!("installed-label")).into()
                        } else if self.downloading.as_deref() == Some(entry.name.as_str()) {
                            text::caption(fl!("downloading-label")).into()
                        } else {
                            let url = entry.download_links.first().cloned().unwrap_or_default();
                            button::text(fl!("library-button"))
                                .on_press(Message::Bible(BM::Download(entry.name.clone(), url)))
                                .into()
                        };

                        let text_col = column![
                            text::body(entry.long_name.clone()),
                            text::caption(entry.description.clone()),
                        ]
                        .spacing(2)
                        .width(Length::Fill); // wraps + shrinks, doesn't own the button's space

                        col = col.push(
                            row![
                                container(text_col).width(Length::Fill),
                                container(action).width(Length::Shrink),
                            ]
                            .spacing(8)
                            .align_y(Alignment::Center),
                        );
                    }
                }
            }
        }

        if let Some(err) = &self.download_error {
            col = col.push(text::caption(err.clone()));
        }

        // Cap the list's own height and make it scroll independently of the
        // rest of the settings modal (toggles etc. above it stay fixed).
        scrollable(col)
            .height(Length::Fixed(280.0))
            .width(Length::Fill)
            .into()
    }
}
