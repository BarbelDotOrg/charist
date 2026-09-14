use crate::app::CharistApp;
use crate::update::Message;
use cosmic::Element;
use cosmic::iced::Length;
use cosmic::widget;
use cosmic::widget::text;

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

    pub fn icon_tooltip_button(
        &self,
        icon_name: &str,
        tooltip: String,
        on_press: Message,
    ) -> Element<Message> {
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
