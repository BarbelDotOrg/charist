use cosmic::iced::{Background, Border, Color};
use cosmic::widget::container;

pub fn verse_style(theme: &cosmic::Theme, is_selected: bool) -> container::Style {
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

pub fn error_banner_style(theme: &cosmic::Theme) -> container::Style {
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

pub fn backdrop_style(_theme: &cosmic::Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.45))),
        ..Default::default()
    }
}

pub fn cross_ref_item_style(theme: &cosmic::Theme) -> container::Style {
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
