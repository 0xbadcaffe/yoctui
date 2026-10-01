use super::*;

pub(super) fn bounded_lines(text: &str) -> Vec<String> {
    let mut bytes = 0usize;
    text.lines()
        .take_while(|line| {
            bytes = bytes.saturating_add(line.len() + 1);
            bytes <= MAX_HARDWARE_TEXT_BYTES
        })
        .map(|line| {
            line.chars()
                .filter(|character| *character == '\t' || !character.is_control())
                .collect()
        })
        .collect()
}

pub(super) fn readable_pdf_lines(text: &str) -> Vec<String> {
    let cleaned = text
        .chars()
        .filter(|character| {
            matches!(character, '\n' | '\t')
                || (!character.is_control()
                    && !is_private_use(*character)
                    && *character != '\u{fffd}')
        })
        .collect::<String>();
    if !cleaned.chars().any(char::is_alphanumeric) {
        return Vec::new();
    }
    bounded_lines(&cleaned)
}

fn is_private_use(character: char) -> bool {
    matches!(character as u32, 0xe000..=0xf8ff | 0xf0000..=0xffffd | 0x100000..=0x10fffd)
}
