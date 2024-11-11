use ratatui::style::{Color, Modifier, Style};

pub const HELP: Style = Style::new().fg(Color::DarkGray);
pub const HIGHLIGHT: Style =
  Style::new().bg(Color::DarkGray).fg(Color::LightCyan).add_modifier(Modifier::BOLD);
pub const KEY_HIGHLIGHT: Style = Style::new().fg(Color::Yellow);
pub const LABEL: Style = Style::new().fg(Color::LightBlue);
pub const NORMAL: Style = Style::new().fg(Color::Gray);
pub const TITLE: Style = Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD);

pub fn focused_border() -> Style { Style::default().fg(Color::LightBlue) }

pub fn unfocused_border() -> Style { Style::default().fg(Color::Blue) }

pub fn border_style(focused: bool) -> Style {
  if focused {
    focused_border()
  } else {
    unfocused_border()
  }
}
