/// ECMA-48 CSI final bytes include punctuation, not only letters.
pub fn is_csi_final_byte(byte: u8) -> bool {
    (0x40..=0x7e).contains(&byte)
}

/// Strip terminal escape sequences while preserving ordinary text and whitespace.
/// Unterminated control strings are discarded through the end of the input.
pub fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(character) = chars.next() {
        if character != '\x1b' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('[') => {
                for next in chars.by_ref() {
                    if next.is_ascii() && is_csi_final_byte(next as u8) {
                        break;
                    }
                }
            }
            Some(kind @ (']' | 'P' | 'X' | '^' | '_')) => {
                while let Some(next) = chars.next() {
                    if kind == ']' && next == '\x07' {
                        break;
                    }
                    if next == '\x1b' && chars.peek() == Some(&'\\') {
                        chars.next();
                        break;
                    }
                }
            }
            Some(' '..='/') => {
                for next in chars.by_ref() {
                    if ('0'..='~').contains(&next) {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

/// Truncate to a byte budget without splitting a UTF-8 character.
pub fn truncate_utf8(value: &mut String, limit: usize) {
    if value.len() <= limit {
        return;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
}

#[cfg(test)]
#[path = "tests/text/mod.rs"]
mod tests;

/// Append bounded UTF-8 text without splitting characters.
pub fn push_bounded(output: &mut String, value: &str, maximum_bytes: usize) -> bool {
    let available = maximum_bytes.saturating_sub(output.len());
    if value.len() <= available {
        output.push_str(value);
        return true;
    }
    let mut keep = available.min(value.len());
    while keep > 0 && !value.is_char_boundary(keep) {
        keep -= 1;
    }
    output.push_str(&value[..keep]);
    false
}

/// Append bounded UTF-8 text without splitting characters.
pub fn append_truncation_marker(output: &mut String, marker: &str, maximum_bytes: usize) {
    if marker.len() > maximum_bytes {
        return;
    }
    if output.len().saturating_add(marker.len()) > maximum_bytes {
        let mut keep = maximum_bytes - marker.len();
        while keep > 0 && !output.is_char_boundary(keep) {
            keep -= 1;
        }
        output.truncate(keep);
    }
    output.push_str(marker);
}

/// Return the longest prefix within the byte budget.
pub fn utf8_prefix(value: &str, limit: usize) -> &str {
    let mut end = limit.min(value.len());
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    &value[..end]
}

#[cfg(test)]
#[path = "tests/text_bounded/mod.rs"]
mod bounded_tests;
