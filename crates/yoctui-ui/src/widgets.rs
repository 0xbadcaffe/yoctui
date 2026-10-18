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
    for character in value.chars() {
        output.push(character);
        if Line::from(output.as_str()).width() > budget {
            output.pop();
            break;
        }
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
