fn push_bounded(queue: &mut VecDeque<TextAreaSnapshot>, snapshot: TextAreaSnapshot) {
    if queue.len() == TEXTAREA_MAX_HISTORY {
        queue.pop_front();
    }
    queue.push_back(snapshot);
}

fn position_at(text: &str, offset: usize) -> TextAreaPosition {
    let offset = clamp_boundary(text, offset);
    let start = line_start(text, offset);
    TextAreaPosition {
        line: text[..start].bytes().filter(|byte| *byte == b'\n').count(),
        column: text[start..offset].chars().count(),
    }
}

fn offset_for_position(text: &str, target_line: usize, column: usize) -> usize {
    let mut start = 0;
    for _ in 0..target_line {
        start = text[start..]
            .find('\n')
            .map_or(text.len(), |index| start + index + 1);
    }
    let end = line_end(text, start);
    text[start..end]
        .char_indices()
        .nth(column)
        .map_or(end, |(index, _)| start + index)
}

fn line_start(text: &str, cursor: usize) -> usize {
    text[..cursor].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end(text: &str, cursor: usize) -> usize {
    text[cursor..]
        .find('\n')
        .map_or(text.len(), |index| cursor + index)
}

fn previous_boundary(text: &str, cursor: usize) -> usize {
    text[..cursor]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_boundary(text: &str, cursor: usize) -> usize {
    text[cursor..]
        .chars()
        .next()
        .map_or(text.len(), |character| cursor + character.len_utf8())
}

fn clamp_boundary(text: &str, index: usize) -> usize {
    yoctui_utils::utf8_prefix(text, index).len()
}

fn word_left(text: &str, cursor: usize) -> usize {
    let mut at = cursor;
    while at > 0 {
        let previous = previous_boundary(text, at);
        if !text[previous..at]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            break;
        }
        at = previous;
    }
    let class = (at > 0)
        .then(|| {
            let previous = previous_boundary(text, at);
            text[previous..at].chars().next().map(word_class)
        })
        .flatten();
    while at > 0 {
        let previous = previous_boundary(text, at);
        let Some(character) = text[previous..at].chars().next() else {
            break;
        };
        if Some(word_class(character)) != class {
            break;
        }
        at = previous;
    }
    at
}

fn unicode_case_insensitive_matches(text: &str, query: &str) -> Vec<(usize, usize)> {
    let folded_query = query.to_lowercase();
    let mut matches = Vec::new();
    for (start, _) in text.char_indices() {
        let mut folded = String::new();
        for (relative, character) in text[start..].char_indices() {
            folded.extend(character.to_lowercase());
            if folded == folded_query {
                matches.push((start, start + relative + character.len_utf8()));
                break;
            }
            if folded.len() >= folded_query.len() && !folded_query.starts_with(&folded) {
                break;
            }
            if !folded_query.starts_with(&folded) {
                break;
            }
        }
    }
    matches
}

fn word_right(text: &str, cursor: usize) -> usize {
    let mut at = cursor;
    let class = text[at..].chars().next().map(word_class);
    while at < text.len() {
        let next = next_boundary(text, at);
        let Some(character) = text[at..next].chars().next() else {
            break;
        };
        if Some(word_class(character)) != class {
            break;
        }
        at = next;
    }
    while at < text.len() {
        let next = next_boundary(text, at);
        if !text[at..next]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            break;
        }
        at = next;
    }
    at
}

fn word_class(character: char) -> u8 {
    if character.is_alphanumeric() || character == '_' {
        1
    } else if character.is_whitespace() {
        2
    } else {
        3
    }
}

fn line_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if character == '\n' {
            ranges.push((start, index));
            start = index + 1;
        }
    }
    ranges.push((start, text.len()));
    ranges
}

fn wrap_ranges(text: &str, start: usize, end: usize, width: Option<usize>) -> Vec<(usize, usize)> {
    let Some(width) = width else {
        return vec![(start, end)];
    };
    if start == end {
        return vec![(start, end)];
    }
    let mut ranges = Vec::new();
    let mut chunk_start = start;
    let mut chars = 0;
    for (relative, _) in text[start..end].char_indices() {
        if chars == width {
            ranges.push((chunk_start, start + relative));
            chunk_start = start + relative;
            chars = 0;
        }
        chars += 1;
    }
    ranges.push((chunk_start, end));
    ranges
}

fn build_diff(base: &str, current: &str) -> TextAreaDiffPreview {
    let old: Vec<_> = base.lines().collect();
    let new: Vec<_> = current.lines().collect();
    let mut prefix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len().saturating_sub(prefix)
        && suffix < new.len().saturating_sub(prefix)
        && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let mut lines = Vec::new();
    let context_start = prefix.saturating_sub(2);
    for (index, text) in old.iter().enumerate().take(prefix).skip(context_start) {
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Context,
            old_line: Some(index + 1),
            new_line: Some(index + 1),
            text: (*text).to_owned(),
        });
    }
    for (index, text) in old.iter().enumerate().take(old.len() - suffix).skip(prefix) {
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Removed,
            old_line: Some(index + 1),
            new_line: None,
            text: (*text).to_owned(),
        });
    }
    for (index, text) in new.iter().enumerate().take(new.len() - suffix).skip(prefix) {
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Added,
            old_line: None,
            new_line: Some(index + 1),
            text: (*text).to_owned(),
        });
    }
    for offset in (0..suffix)
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let old_index = old.len() - suffix + offset;
        let new_index = new.len() - suffix + offset;
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Context,
            old_line: Some(old_index + 1),
            new_line: Some(new_index + 1),
            text: old[old_index].to_owned(),
        });
    }
    let truncated = lines.len() > TEXTAREA_MAX_DIFF_LINES;
    lines.truncate(TEXTAREA_MAX_DIFF_LINES);
    TextAreaDiffPreview {
        base: TextAreaRevision::of(base),
        current: TextAreaRevision::of(current),
        lines,
        truncated,
    }
}

fn atomic_temporary_path(target: &Path, revision: &TextAreaRevision) -> PathBuf {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("yoctui-save");
    let suffix = u64::from_be_bytes(revision.sha256[..8].try_into().expect("fixed digest"));
    target.with_file_name(format!(".{name}.yoctui-{suffix:016x}.tmp"))
}

use yoctui_utils::truncate_utf8;

fn bounded_utf8(mut value: String, limit: usize) -> String {
    truncate_utf8(&mut value, limit);
    value
}

