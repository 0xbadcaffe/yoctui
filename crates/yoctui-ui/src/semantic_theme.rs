//! Semantic theme.
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SemanticTheme {
    pub background: Color,
    pub primary_foreground: Color,
    pub secondary_foreground: Color,
    pub focused_border: Color,
    pub inactive_border: Color,
    pub selection_foreground: Color,
    pub selection_background: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub running: Color,
    pub pending: Color,
    pub accent: Color,
    pub muted: Color,
    pub progress: Color,
    pub graph_cpu: Color,
    pub graph_memory: Color,
    pub graph_disk_read: Color,
    pub graph_disk_write: Color,
    pub graph_network_rx: Color,
    pub graph_network_tx: Color,
    pub disabled: Color,
    pub informational: Color,
    pub heading: Color,
    pub table_header: Color,
    pub syntax_keyword: Color,
    pub syntax_name: Color,
    pub syntax_operator: Color,
    pub syntax_value: Color,
    pub syntax_comment: Color,
    pub attribute_only: bool,
}

pub(crate) type ThemePalette = SemanticTheme;

impl SemanticTheme {
    pub(crate) fn for_app(app: &App) -> Self {
        Self::for_theme(app.theme, app.color_enabled)
    }

    pub fn for_theme(theme: Theme, color_enabled: bool) -> Self {
        if !color_enabled {
            return Self::monochrome();
        }
        match theme {
            Theme::DarkPro => Self::packrat([
                (4, 12, 17),
                (8, 18, 24),
                (41, 55, 63),
                (13, 57, 132),
                (218, 222, 224),
                (155, 166, 172),
                (83, 96, 104),
                (42, 178, 218),
                (139, 211, 0),
                (226, 170, 0),
                (244, 67, 54),
                (176, 126, 214),
            ]),
            Theme::WhiteClassic => Self::packrat([
                (248, 248, 248),
                (240, 240, 240),
                (192, 192, 192),
                (178, 208, 232),
                (20, 20, 20),
                (74, 74, 74),
                (136, 136, 136),
                (0, 107, 107),
                (26, 122, 0),
                (139, 105, 20),
                (204, 34, 0),
                (123, 61, 160),
            ]),
            Theme::MatrixGreen => Self::packrat([
                (0, 0, 0),
                (6, 14, 6),
                (20, 60, 20),
                (0, 50, 0),
                (0, 204, 68),
                (0, 119, 34),
                (0, 51, 17),
                (0, 255, 136),
                (0, 221, 0),
                (136, 255, 0),
                (255, 51, 0),
                (136, 255, 136),
            ]),
            Theme::VscodeDark => Self::packrat([
                (30, 30, 30),
                (37, 37, 38),
                (68, 68, 68),
                (38, 79, 120),
                (212, 212, 212),
                (154, 154, 154),
                (106, 106, 106),
                (79, 193, 255),
                (106, 153, 85),
                (220, 220, 170),
                (244, 71, 71),
                (197, 134, 192),
            ]),
            Theme::VscodeLight => Self::packrat([
                (255, 255, 255),
                (243, 243, 243),
                (200, 200, 200),
                (173, 214, 255),
                (0, 0, 0),
                (68, 68, 68),
                (138, 138, 138),
                (0, 112, 193),
                (9, 134, 88),
                (120, 83, 0),
                (205, 49, 49),
                (175, 0, 219),
            ]),
            Theme::AccessibleDark => Self::packrat([
                (18, 18, 18),
                (28, 28, 30),
                (92, 92, 98),
                (45, 65, 72),
                (238, 238, 238),
                (188, 188, 192),
                (120, 120, 126),
                (86, 180, 233),
                (0, 158, 115),
                (240, 228, 66),
                (213, 94, 0),
                (204, 121, 167),
            ]),
            Theme::SoftLight => Self::packrat([
                (252, 252, 253),
                (241, 243, 245),
                (164, 170, 178),
                (198, 220, 232),
                (30, 34, 39),
                (78, 84, 92),
                (130, 136, 145),
                (0, 103, 148),
                (35, 120, 74),
                (139, 100, 0),
                (184, 44, 52),
                (128, 70, 160),
            ]),
            Theme::HighContrast => Self::packrat([
                (0, 0, 0),
                (12, 12, 12),
                (210, 210, 210),
                (70, 70, 70),
                (255, 255, 255),
                (220, 220, 220),
                (150, 150, 150),
                (0, 220, 255),
                (50, 255, 100),
                (255, 235, 40),
                (255, 70, 70),
                (255, 100, 235),
            ]),
            Theme::Monochrome => Self::monochrome(),
        }
    }

    pub fn widget_styles(self) -> WidgetStyles {
        WidgetStyles {
            primary: self.base(),
            success: self.role(self.success, Modifier::BOLD),
            warning: self.role(self.warning, Modifier::BOLD),
            error: self.role(self.error, Modifier::BOLD | Modifier::UNDERLINED),
            running: self.role(self.running, Modifier::BOLD),
            pending: self.role(self.pending, Modifier::DIM),
            disabled: self.role(self.disabled, Modifier::DIM),
            accent: self.role(self.accent, Modifier::BOLD),
            muted: self.role(self.muted, Modifier::DIM),
            informational: self.role(self.informational, Modifier::ITALIC),
            progress: self.role(self.progress, Modifier::BOLD),
            graph_cpu: self.role(self.graph_cpu, Modifier::BOLD),
            graph_memory: self.role(self.graph_memory, Modifier::BOLD),
            graph_disk_read: self.role(self.graph_disk_read, Modifier::BOLD),
            graph_disk_write: self.role(self.graph_disk_write, Modifier::BOLD),
            graph_network_rx: self.role(self.graph_network_rx, Modifier::BOLD),
            graph_network_tx: self.role(self.graph_network_tx, Modifier::BOLD),
            selected: self.selected(),
        }
    }

    pub(crate) fn packrat(colors: [(u8, u8, u8); 12]) -> Self {
        let [
            bg,
            _bg2,
            border,
            selected,
            fg,
            fg2,
            fg3,
            cyan,
            green,
            yellow,
            red,
            magenta,
        ] = colors;
        let rgb = |(r, g, b)| Color::Rgb(r, g, b);
        Self {
            background: rgb(bg),
            primary_foreground: rgb(fg),
            secondary_foreground: rgb(fg2),
            focused_border: rgb(cyan),
            inactive_border: rgb(border),
            selection_foreground: rgb(fg),
            selection_background: rgb(selected),
            success: rgb(green),
            warning: rgb(yellow),
            error: rgb(red),
            running: rgb(green),
            pending: rgb(yellow),
            accent: rgb(magenta),
            muted: rgb(fg2),
            progress: rgb(green),
            graph_cpu: rgb(cyan),
            graph_memory: rgb(magenta),
            graph_disk_read: rgb(cyan),
            graph_disk_write: rgb(magenta),
            graph_network_rx: rgb(green),
            graph_network_tx: rgb(yellow),
            disabled: rgb(fg3),
            informational: rgb(cyan),
            heading: rgb(fg),
            table_header: rgb(fg),
            syntax_keyword: rgb(cyan),
            syntax_name: rgb(yellow),
            syntax_operator: rgb(magenta),
            syntax_value: rgb(green),
            syntax_comment: rgb(fg2),
            attribute_only: false,
        }
    }

    pub(crate) fn monochrome() -> Self {
        Self {
            background: Color::Reset,
            primary_foreground: Color::Reset,
            secondary_foreground: Color::Reset,
            focused_border: Color::Reset,
            inactive_border: Color::Reset,
            selection_foreground: Color::Reset,
            selection_background: Color::Reset,
            success: Color::Reset,
            warning: Color::Reset,
            error: Color::Reset,
            running: Color::Reset,
            pending: Color::Reset,
            accent: Color::Reset,
            muted: Color::Reset,
            progress: Color::Reset,
            graph_cpu: Color::Reset,
            graph_memory: Color::Reset,
            graph_disk_read: Color::Reset,
            graph_disk_write: Color::Reset,
            graph_network_rx: Color::Reset,
            graph_network_tx: Color::Reset,
            disabled: Color::Reset,
            informational: Color::Reset,
            heading: Color::Reset,
            table_header: Color::Reset,
            syntax_keyword: Color::Reset,
            syntax_name: Color::Reset,
            syntax_operator: Color::Reset,
            syntax_value: Color::Reset,
            syntax_comment: Color::Reset,
            attribute_only: true,
        }
    }

    pub(crate) fn base(self) -> Style {
        Style::default()
            .fg(self.primary_foreground)
            .bg(self.background)
    }

    pub(crate) fn focus(self) -> Style {
        let style = Style::default().fg(self.focused_border);
        if self.attribute_only {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    pub(crate) fn selected(self) -> Style {
        if self.attribute_only {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
                .fg(self.selection_foreground)
                .bg(self.selection_background)
        }
    }

    pub(crate) fn role(self, color: Color, modifier: Modifier) -> Style {
        let style = Style::default().fg(color);
        if self.attribute_only {
            style.add_modifier(modifier)
        } else {
            style
        }
    }
}
