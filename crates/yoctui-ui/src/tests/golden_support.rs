//! Golden support.
use super::*;

pub(crate) const LITERAL_WIDTH: u16 = LITERAL_REFERENCE_WIDTH;
pub(crate) const LITERAL_HEIGHT: u16 = 48;
pub(crate) const TARGET_GOLDEN_WIDTH: u16 = 160;
pub(crate) const TARGET_GOLDEN_HEIGHT: u16 = 50;
pub(crate) const LITERAL_GOLDEN_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/golden/literal-reference-160x48.cells"
);

pub(crate) fn literal_now() -> SystemTime {
    UNIX_EPOCH + Duration::from_secs(70_107)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LiteralCell {
    pub(crate) symbol: String,
    pub(crate) style: String,
}

pub(crate) fn literal_style(cell: &ratatui::buffer::Cell) -> String {
    format!(
        "fg={:?};bg={:?};ul={:?};mod={:?}",
        cell.fg, cell.bg, cell.underline_color, cell.modifier
    )
}

pub(crate) fn literal_cells(terminal: &Terminal<TestBackend>) -> Vec<LiteralCell> {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| LiteralCell {
            symbol: cell.symbol().into(),
            style: literal_style(cell),
        })
        .collect()
}

pub(crate) fn serialize_literal_golden(cells: &[LiteralCell]) -> String {
    let mut output = format!("YOCTUI_CELL_GOLDEN_V1 {LITERAL_WIDTH} {LITERAL_HEIGHT}\nSYMBOLS\n");
    for row in cells.chunks(usize::from(LITERAL_WIDTH)) {
        output.push_str("S|");
        for cell in row {
            output.push_str(&format!("{}:{}", cell.symbol.len(), cell.symbol));
        }
        output.push('\n');
    }
    output.push_str("STYLES\n");
    let mut start = 0;
    while start < cells.len() {
        let style = &cells[start].style;
        let mut end = start + 1;
        while end < cells.len() && cells[end].style == *style {
            end += 1;
        }
        output.push_str(&format!("T|{}|{style}\n", end - start));
        start = end;
    }
    output
}

pub(crate) fn parse_literal_symbols(line: &str) -> Vec<String> {
    let bytes = line
        .strip_prefix("S|")
        .expect("literal symbol row must start with S|")
        .as_bytes();
    let mut symbols = Vec::with_capacity(usize::from(LITERAL_WIDTH));
    let mut cursor = 0;
    while cursor < bytes.len() {
        let colon = bytes[cursor..]
            .iter()
            .position(|byte| *byte == b':')
            .map(|offset| cursor + offset)
            .expect("literal symbol length must end with colon");
        let length = std::str::from_utf8(&bytes[cursor..colon])
            .expect("literal symbol length must be UTF-8")
            .parse::<usize>()
            .expect("literal symbol length must be numeric");
        cursor = colon + 1;
        let end = cursor + length;
        symbols.push(
            std::str::from_utf8(&bytes[cursor..end])
                .expect("literal symbol must be UTF-8")
                .into(),
        );
        cursor = end;
    }
    symbols
}

pub(crate) fn parse_literal_golden(golden: &str) -> Vec<LiteralCell> {
    let mut lines = golden.lines();
    assert_eq!(
        lines.next(),
        Some("YOCTUI_CELL_GOLDEN_V1 160 48"),
        "literal golden header changed"
    );
    assert_eq!(lines.next(), Some("SYMBOLS"));
    let mut symbols = Vec::with_capacity(usize::from(LITERAL_WIDTH * LITERAL_HEIGHT));
    for _ in 0..LITERAL_HEIGHT {
        symbols.extend(parse_literal_symbols(
            lines
                .next()
                .expect("literal golden is missing a symbol row"),
        ));
    }
    assert_eq!(lines.next(), Some("STYLES"));
    let mut styles = Vec::with_capacity(symbols.len());
    for line in lines.filter(|line| !line.is_empty()) {
        let value = line
            .strip_prefix("T|")
            .expect("literal style run must start with T|");
        let (count, style) = value
            .split_once('|')
            .expect("literal style run must contain a count");
        styles.extend(std::iter::repeat_n(
            style.to_owned(),
            count
                .parse::<usize>()
                .expect("literal style count must be numeric"),
        ));
    }
    assert_eq!(symbols.len(), usize::from(LITERAL_WIDTH * LITERAL_HEIGHT));
    assert_eq!(styles.len(), symbols.len());
    symbols
        .into_iter()
        .zip(styles)
        .map(|(symbol, style)| LiteralCell { symbol, style })
        .collect()
}

pub(crate) fn assert_literal_cells(expected: &[LiteralCell], actual: &[LiteralCell]) {
    assert_eq!(expected.len(), actual.len());
    if let Some((index, (expected, actual))) = expected
        .iter()
        .zip(actual)
        .enumerate()
        .find(|(_, (expected, actual))| expected != actual)
    {
        let x = index % usize::from(LITERAL_WIDTH);
        let y = index / usize::from(LITERAL_WIDTH);
        panic!(
            "literal reference mismatch at ({x},{y})\nexpected: {expected:?}\n  actual: {actual:?}\nRun scripts/update-literal-ui-golden.sh only after reviewing the intentional UI change."
        );
    }
}

pub(crate) fn serialize_target_golden(cells: &[LiteralCell]) -> String {
    let mut output =
        format!("YOCTUI_CELL_GOLDEN_V1 {TARGET_GOLDEN_WIDTH} {TARGET_GOLDEN_HEIGHT}\nSYMBOLS\n");
    for row in cells.chunks(usize::from(TARGET_GOLDEN_WIDTH)) {
        output.push_str("S|");
        for cell in row {
            output.push_str(&format!("{}:{}", cell.symbol.len(), cell.symbol));
        }
        output.push('\n');
    }
    output.push_str("STYLES\n");
    let mut start = 0;
    while start < cells.len() {
        let style = &cells[start].style;
        let mut end = start + 1;
        while end < cells.len() && cells[end].style == *style {
            end += 1;
        }
        output.push_str(&format!("T|{}|{style}\n", end - start));
        start = end;
    }
    output
}

pub(crate) fn parse_target_golden(golden: &str) -> Vec<LiteralCell> {
    let mut lines = golden.lines();
    assert_eq!(
        lines.next(),
        Some("YOCTUI_CELL_GOLDEN_V1 160 50"),
        "target-design golden header changed"
    );
    assert_eq!(lines.next(), Some("SYMBOLS"));
    let mut symbols = Vec::with_capacity(usize::from(TARGET_GOLDEN_WIDTH * TARGET_GOLDEN_HEIGHT));
    for _ in 0..TARGET_GOLDEN_HEIGHT {
        symbols.extend(parse_literal_symbols(
            lines
                .next()
                .expect("target-design golden is missing a symbol row"),
        ));
    }
    assert_eq!(lines.next(), Some("STYLES"));
    let mut styles = Vec::with_capacity(symbols.len());
    for line in lines.filter(|line| !line.is_empty()) {
        let value = line
            .strip_prefix("T|")
            .expect("target-design style run must start with T|");
        let (count, style) = value
            .split_once('|')
            .expect("target-design style run must contain a count");
        styles.extend(std::iter::repeat_n(
            style.to_owned(),
            count
                .parse::<usize>()
                .expect("target-design style count must be numeric"),
        ));
    }
    assert_eq!(
        symbols.len(),
        usize::from(TARGET_GOLDEN_WIDTH * TARGET_GOLDEN_HEIGHT)
    );
    assert_eq!(styles.len(), symbols.len());
    symbols
        .into_iter()
        .zip(styles)
        .map(|(symbol, style)| LiteralCell { symbol, style })
        .collect()
}

pub(crate) fn assert_target_golden(name: &str, expected: &[LiteralCell], actual: &[LiteralCell]) {
    assert_eq!(expected.len(), actual.len());
    if let Some((index, (expected, actual))) = expected
        .iter()
        .zip(actual)
        .enumerate()
        .find(|(_, (expected, actual))| expected != actual)
    {
        let x = index % usize::from(TARGET_GOLDEN_WIDTH);
        let y = index / usize::from(TARGET_GOLDEN_WIDTH);
        panic!(
            "target-design golden {name} mismatch at ({x},{y})\nexpected: {expected:?}\n  actual: {actual:?}\nRun scripts/update-target-design-goldens.sh only after reviewing the intentional UI change."
        );
    }
}
