use crate::app::{CharistApp};
use crate::update::Message;
use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::widget::{row, text, column, container};
use i18n_embed_fl::fl;
use crate::bibles::BibleData;

impl CharistApp {
    pub fn labeled_field<'a>(
        &'a self,
        label: String,
        field: Element<'a, Message>,
        portion: u16,
    ) -> Element<'a, Message> {
        cosmic::widget::column![text::caption(label), field]
            .spacing(6)
            .width(Length::FillPortion(portion))
            .into()
    }

    pub fn icon_tooltip_button<'a>(
        &'a self,
        icon_name: &'static str,
        tooltip: String,
        on_press: Message,
    ) -> Element<'a, Message> {
        widget::tooltip(
            widget::button::icon(widget::icon::from_name(icon_name))
                .on_press(on_press)
                .width(Length::Fixed(36.0))
                .height(Length::Fixed(36.0)),
            text::caption(tooltip),
            widget::tooltip::Position::Bottom,
        )
            .into()
    }
}
