use super::*;
use ratatui::{
    Terminal,
    backend::TestBackend,
    prelude::{Color, Modifier},
    widgets::Paragraph,
};

fn styles() -> PaneStyles {
    PaneStyles {
        base: Style::default().fg(Color::White),
        border: Style::default().fg(Color::DarkGray),
        focused_border: Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        selected: Style::default().bg(Color::Blue),
        inactive_selected: Style::default().add_modifier(Modifier::UNDERLINED),
        table_header: Style::default().add_modifier(Modifier::BOLD),
        muted: Style::default().add_modifier(Modifier::DIM),
    }
}

include!("foundation.rs");
include!("widgets.rs");
