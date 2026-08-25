//! Dialog shells, popup clearing, and bounded dialog geometry.

use super::*;

pub(super) fn dialog_styles(app: &App) -> DialogStyles {
    let palette = ThemePalette::for_app(app);
    DialogStyles {
        base: palette.base(),
        focused_border: palette.focus(),
        heading: palette.role(palette.heading, Modifier::BOLD),
        selected: palette.selected(),
        disabled: palette.role(palette.disabled, Modifier::DIM),
        validation: palette.role(palette.error, Modifier::BOLD),
        hint: palette.role(palette.secondary_foreground, Modifier::DIM),
        destructive: palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
    }
}

pub(super) fn dialog_shell(app: &App, title: impl Into<String>, tone: DialogTone) -> DialogShell {
    DialogShell::new(title, tone, dialog_styles(app))
}

pub(super) fn dialog_block(
    app: &App,
    title: impl Into<String>,
    tone: DialogTone,
) -> Block<'static> {
    dialog_shell(app, title, tone).block()
}

pub(super) fn clear_popup(frame: &mut Frame, app: &App, area: Rect) {
    frame.render_widget(Clear, area);
    let palette = ThemePalette::for_app(app);
    frame.render_widget(
        Block::default()
            .style(palette.base())
            .border_style(palette.focus()),
        area,
    );
}

pub(super) fn dialog_popup_rect(area: Rect, preferred_width: u16, preferred_height: u16) -> Rect {
    bounded_dialog_rect(area, preferred_width, preferred_height)
}
