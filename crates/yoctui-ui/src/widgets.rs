//! Shared bounded text and metadata presentation helpers.

use super::*;

pub(super) fn matches_metadata(query: &str, values: &[&str]) -> bool {
    let query = query.to_lowercase();
    query.is_empty()
        || values
            .iter()
            .any(|value| value.to_lowercase().contains(query.as_str()))
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
