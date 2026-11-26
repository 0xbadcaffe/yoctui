//! Shared bounded text and metadata presentation helpers.

use super::*;

pub(super) fn matches_metadata(query: &str, values: &[&str]) -> bool {
    let query = query.to_lowercase();
    matches_folded_metadata(&query, values)
}

pub(super) fn matches_folded_metadata(query: &str, values: &[&str]) -> bool {
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(query))
}

/// Borrow the complete inventory for the common empty-search case. Only a
/// filtered view allocates indices; no cached state can outlive an inventory update.
pub(super) struct MetadataRows<'a, T> {
    values: &'a [T],
    indices: Option<Vec<usize>>,
}

impl<'a, T> MetadataRows<'a, T> {
    pub(super) fn new(
        values: &'a [T],
        query: &str,
        mut matches: impl FnMut(&T, &str) -> bool,
    ) -> Self {
        let indices = if query.is_empty() {
            None
        } else {
            let query = query.to_lowercase();
            Some(
                values
                    .iter()
                    .enumerate()
                    .filter_map(|(i, value)| matches(value, &query).then_some(i))
                    .collect(),
            )
        };
        Self { values, indices }
    }

    pub(super) fn len(&self) -> usize {
        self.indices.as_ref().map_or(self.values.len(), Vec::len)
    }

    pub(super) fn ordinal(&self, original: usize) -> Option<usize> {
        self.indices.as_ref().map_or_else(
            || (original < self.values.len()).then_some(original),
            |indices| indices.binary_search(&original).ok(),
        )
    }

    pub(super) fn get(&self, ordinal: usize) -> Option<(usize, &'a T)> {
        let original = match &self.indices {
            Some(indices) => *indices.get(ordinal)?,
            None => ordinal,
        };
        self.values.get(original).map(|value| (original, value))
    }
}

/// The frame background covers a contiguous buffer. Avoid coordinate lookup
/// per cell; preserve Buffer::set_style's clipping behavior for partial areas.
pub(super) fn paint_frame_style(buffer: &mut Buffer, area: Rect, style: Style) {
    if buffer.area == area {
        for cell in &mut buffer.content {
            cell.set_style(style);
        }
    } else {
        buffer.set_style(area, style);
    }
}

pub(super) fn bounded_cell_text(value: &str, width: u16) -> String {
    if Line::from(value).width() <= usize::from(width) {
        return value.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    let budget = usize::from(width.saturating_sub(1));
    let mut output = String::new();
    let mut remaining = budget;
    let span = Span::raw(value);
    for grapheme in span.styled_graphemes(Style::default()) {
        let cells = unicode_width::UnicodeWidthStr::width(grapheme.symbol);
        if cells > remaining {
            break;
        }
        remaining -= cells;
        output.push_str(grapheme.symbol);
    }
    output.push('…');
    output
}

pub(super) fn timestamp_text(timestamp: SystemTime) -> String {
    timestamp.duration_since(UNIX_EPOCH).map_or_else(
        |_| "before Unix epoch".into(),
        |duration| format!("{}s since Unix epoch", duration.as_secs()),
    )
}
pub(super) fn file_browser_breadcrumb(browser: &LayerBrowser, width: u16) -> String {
    let directory = browser
        .selected_entry()
        .and_then(|entry| entry.path.parent())
        .unwrap_or(&browser.root);
    let relative = directory.strip_prefix(&browser.root).unwrap_or(directory);
    let path = if browser.is_rootfs() {
        Path::new("/").join(relative)
    } else {
        browser.root.join(relative)
    };
    let text = path.to_string_lossy();
    if Line::from(text.as_ref()).width() <= usize::from(width) || width == 0 {
        return bounded_cell_text(&text, width);
    }
    let span = Span::raw(text.as_ref());
    let graphemes: Vec<_> = span.styled_graphemes(Style::default()).collect();
    let mut remaining = usize::from(width.saturating_sub(1));
    let mut suffix_start = text.len();
    for grapheme in graphemes.iter().rev() {
        let cells = unicode_width::UnicodeWidthStr::width(grapheme.symbol);
        if cells > remaining {
            break;
        }
        remaining -= cells;
        suffix_start -= grapheme.symbol.len();
    }
    format!("…{}", &text[suffix_start..])
}

pub(super) fn render_file_browser_scrollbar(
    frame: &mut Frame,
    app: &App,
    border: Rect,
    scroll: yoctui_model::BoundedScroll,
) {
    use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};

    if scroll.viewport == 0 || scroll.total <= scroll.viewport || border.width < 2 {
        return;
    }
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    let mut state = ScrollbarState::new(scroll.total - scroll.viewport + 1)
        .position(scroll.offset)
        .viewport_content_length(scroll.viewport);
    frame.render_stateful_widget(
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None)
            .track_symbol(Some(if unicode { "│" } else { "|" }))
            .thumb_symbol(if unicode { "█" } else { "#" })
            .thumb_style(ThemePalette::for_app(app).focus()),
        Rect::new(border.right() - 1, border.y + 1, 1, border.height - 2),
        &mut state,
    );
}
