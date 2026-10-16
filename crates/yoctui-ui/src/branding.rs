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
// Raised block lettering with circuit-like outlines, echoing the README banner.
const OUTLINED: [&str; 6] = [
    "██╗   ██╗ ██████╗  ██████╗████████╗██╗   ██╗██╗",
    "╚██╗ ██╔╝██╔═══██╗██╔════╝╚══██╔══╝██║   ██║██║",
    " ╚████╔╝ ██║   ██║██║        ██║   ██║   ██║██║",
    "  ╚██╔╝  ██║   ██║██║        ██║   ██║   ██║██║",
    "   ██║   ╚██████╔╝╚██████╗   ██║   ╚██████╔╝██║",
    "   ╚═╝    ╚═════╝  ╚═════╝   ╚═╝    ╚═════╝ ╚═╝",
];

pub(crate) fn render_brand_logo(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    if area.width >= 48 && area.height >= 6 {
        let lines = OUTLINED
            .iter()
            .enumerate()
            .map(|(row, text)| {
                let text = text
                    .chars()
                    .map(|c| {
                        if unicode {
                            c
                        } else {
                            match c {
                                '█' => '#',
                                '═' => '=',
                                '║' => '|',
                                '╔' | '╗' | '╚' | '╝' => '+',
                                c => c,
                            }
                        }
                    })
                    .collect::<String>();
                Line::styled(
                    text,
                    palette.role(
                        if row < 5 {
                            palette.accent
                        } else {
                            palette.success
                        },
                        Modifier::BOLD,
                    ),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
        return;
    }
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

pub(crate) fn render_about(frame: &mut Frame, app: &App, area: Rect) {
    let popup = dialog_popup_rect(area, 84, 19);
    let palette = ThemePalette::for_app(app);
    let block = Block::default().title("About Yoctui").borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block.style(palette.base()), popup);
    let regions = Layout::vertical([Constraint::Length(6), Constraint::Min(0)]).split(inner);
    render_brand_logo(frame, app, regions[0]);
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                format!("yoctui v{}", env!("CARGO_PKG_VERSION")),
                palette.focus(),
            ),
            Line::raw("Yocto development workbench"),
            Line::raw("Build · inspect · develop · debug"),
            Line::raw("Roy Cohen · MIT license"),
            Line::raw(format!("Build SHA: {}", env!("YOCTUI_BUILD_SHA"))),
            Line::raw("https://github.com/0xbadcaffe/yoctui"),
            Line::raw(""),
            Line::raw("F1 keyboard help · g operator guide · Esc dashboard"),
        ])
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(palette.base()),
        regions[1],
    );
}

/// Portable splash; the caller holds the initial frame for at least 750 ms.
pub fn render_startup(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let palette = ThemePalette::for_app(app);
    frame.render_widget(Block::default().style(palette.base()), area);
    let expanded = area.width >= LOGO_WIDTH + 2 && area.height >= 12;
    let width = area.width.min(72);
    let height = area.height.min(if expanded { 13 } else { 4 });
    let content = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    let body = if expanded {
        let regions = Layout::vertical([
            Constraint::Length(if width >= 48 { 6 } else { LOGO_HEIGHT }),
            Constraint::Min(0),
        ])
        .split(content);
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
