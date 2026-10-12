//! Portable terminal-cell branding; never changes the host window or dock icon.
use super::*;

const LOGO_WIDTH: u16 = 40;
const LOGO_HEIGHT: u16 = 5;
const GLYPHS: [[&str; 5]; 6] = [
    ["#   #", " # # ", "  #  ", "  #  ", "  #  "],
    [" ### ", "#   #", "#   #", "#   #", " ### "],
    [" ####", "#    ", "#    ", "#    ", " ####"],
    ["#####", "  #  ", "  #  ", "  #  ", "  #  "],
    ["#   #", "#   #", "#   #", "#   #", " ### "],
    ["#####", "  #  ", "  #  ", "  #  ", "#####"],
];

pub(crate) fn render_brand_logo(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    let lines = (0..usize::from(LOGO_HEIGHT))
        .map(|row| {
            let spans = GLYPHS
                .iter()
                .enumerate()
                .flat_map(|(index, glyph)| {
                    let pixels = glyph[row]
                        .chars()
                        .map(|pixel| {
                            if pixel == '#' && unicode {
                                '█'
                            } else {
                                pixel
                            }
                        })
                        .collect::<String>();
                    let color = if index % 2 == 0 {
                        palette.accent
                    } else {
                        palette.success
                    };
                    [
                        Span::styled(pixels, palette.role(color, Modifier::BOLD)),
                        Span::raw("  "),
                    ]
                })
                .collect::<Vec<_>>();
            Line::from(spans)
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

/// One immediate frame while the existing daemon attachment is in progress.
/// No sleeps, window-system dependencies, or changes to startup/focus policy.
pub fn render_startup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let palette = ThemePalette::for_app(app);
    frame.render_widget(Block::default().style(palette.base()), area);
    let expanded = area.width >= LOGO_WIDTH + 2 && area.height >= 12;
    let width = area.width.min(64);
    let height = area.height.min(if expanded { 12 } else { 4 });
    let content = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let body = if expanded {
        let regions =
            Layout::vertical([Constraint::Length(LOGO_HEIGHT), Constraint::Min(0)]).split(content);
        render_brand_logo(frame, app, regions[0]);
        regions[1]
    } else {
        content
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!("yoctui v{}", env!("CARGO_PKG_VERSION")),
                palette.focus(),
            ),
            Line::raw("Yocto development workbench"),
            Line::raw(""),
            Line::styled(
                "Connecting to daemon...",
                palette.role(palette.secondary_foreground, Modifier::DIM),
            ),
        ])
        .alignment(Alignment::Center)
        .style(palette.base()),
        body,
    );
}
