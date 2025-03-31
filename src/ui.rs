// src/ui.rs
use iced::Color;
use once_cell::sync::Lazy;

pub struct Styles {
    pub bg: Color,
    pub fg: Color,
    pub footer_bg: Color,
    pub footer_fg: Color,
    pub header_bg: Color,
    pub header_fg: Color,
}

pub static DARK_THEME: Lazy<Styles> = Lazy::new(|| Styles {
    bg: Color::from_rgb(0.0, 0.0, 0.0),
    fg: Color::from_rgb(1.0, 1.0, 1.0),
    footer_bg: Color::from_rgb(0.0078, 0.325, 0.6118), // #02539c
    footer_fg: Color::from_rgb(1.0, 1.0, 1.0),
    header_bg: Color::from_rgb(0.2, 0.2, 0.2),
    header_fg: Color::from_rgb(1.0, 1.0, 1.0),
});

pub static LIGHT_THEME: Lazy<Styles> = Lazy::new(|| Styles {
    bg: Color::from_rgb(1.0, 1.0, 1.0),
    fg: Color::from_rgb(0.0, 0.0, 0.0),
    footer_bg: Color::from_rgb(0.0078, 0.325, 0.6118), // #02539c
    footer_fg: Color::from_rgb(1.0, 1.0, 1.0),
    header_bg: Color::from_rgb(0.8784, 0.8784, 0.8784), // #e0e0e0
    header_fg: Color::from_rgb(0.0, 0.0, 0.0),
});
