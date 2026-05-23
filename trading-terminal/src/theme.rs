use ratatui::style::{Color, Style};

pub const BG: Color = Color::Black;
pub const TEXT: Color = Color::Rgb(255, 191, 0); // amber
pub const POSITIVE: Color = Color::Rgb(0, 255, 65);
pub const NEGATIVE: Color = Color::Rgb(255, 67, 61);
pub const HEADER: Color = Color::Gray;
pub const BORDER: Color = HEADER;
pub const HIGHLIGHT: Color = Color::Cyan;

pub fn default_style() -> Style {
    Style::default().fg(TEXT).bg(BG)
}

pub fn positive_style() -> Style {
    Style::default().fg(POSITIVE).bg(BG)
}

pub fn negative_style() -> Style {
    Style::default().fg(NEGATIVE).bg(BG)
}

pub fn header_style() -> Style {
    Style::default().fg(HEADER).bg(BG)
}

pub fn highlight_style() -> Style {
    Style::default().fg(TEXT).bg(HIGHLIGHT) // Or maybe keeping bg black and text cyan based on "Active/focused panel border or highlight: cyan" -- actually reverse or cyan fg
}

pub fn border_style() -> Style {
    Style::default().fg(HEADER).bg(BG)
}

pub fn highlight_border_style() -> Style {
    Style::default().fg(HIGHLIGHT).bg(BG)
}
