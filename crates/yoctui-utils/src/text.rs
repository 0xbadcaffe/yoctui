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
mod tests {
    use super::*;

    #[test]
    fn ansi_controls_do_not_swallow_text_after_punctuation_finals() {
        assert_eq!(strip_ansi("a\x1b[1~after\x1b[2@done"), "aafterdone");
        assert_eq!(strip_ansi("\x1b[31mλ warning\x1b[0m\n\t"), "λ warning\n\t");
        assert_eq!(
            strip_ansi("\x1b]8;;https://example.invalid\x1b\\link\x1b]8;;\x1b\\"),
            "link"
        );
        assert_eq!(strip_ansi("a\x1b]title\x07b\x1bPpayload\x1b\\c"), "abc");
        assert_eq!(strip_ansi("a\x1b[31"), "a");
        assert_eq!(strip_ansi("a\x1b]unfinished"), "a");
        assert_eq!(strip_ansi("a\x1b"), "a");
        assert_eq!(strip_ansi("\x1b(Btext"), "text");
    }

    #[test]
    fn truncation_covers_every_multibyte_boundary() {
        for (limit, expected) in [
            (0, ""),
            (1, "a"),
            (2, "a"),
            (3, "aλ"),
            (4, "aλ"),
            (7, "aλ🙂"),
            (99, "aλ🙂z"),
        ] {
            let mut value = "aλ🙂z".to_owned();
            truncate_utf8(&mut value, limit);
            assert_eq!(value, expected);
        }
    }
}

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
mod bounded_tests {
    use super::*;
    #[test]
    fn bounded_append_and_marker_share_the_exact_byte_budget() {
        let mut output = "a".to_owned();
        assert!(!push_bounded(&mut output, "λ🙂", 4));
        assert_eq!(output, "aλ");
        append_truncation_marker(&mut output, "!", 3);
        assert_eq!(output, "a!");
        append_truncation_marker(&mut output, "too long", 3);
        assert_eq!(output, "a!");
        assert!(push_bounded(&mut output, "b", 3));
        assert_eq!(output, "a!b");
        assert_eq!(utf8_prefix("λ🙂", usize::MAX), "λ🙂");
    }
}
