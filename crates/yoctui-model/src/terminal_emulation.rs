use crate::{PtyDimensions, PtySessionError};
use thiserror::Error;

pub const MAX_TERMINAL_SCROLLBACK_LINES: usize = 100_000;
pub const MAX_TERMINAL_FEED_BYTES: usize = 64 * 1024;
pub const MAX_TERMINAL_SNAPSHOT_CELLS: usize = 250_000;
pub const MAX_TERMINAL_CELL_BYTES: usize = 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalColor {
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalCell {
    pub contents: String,
    pub foreground: TerminalColor,
    pub background: TerminalColor,
    pub bold: bool,
    pub dim: bool,
    pub italic: bool,
    pub underline: bool,
    pub inverse: bool,
    pub wide: bool,
    pub wide_continuation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMouseMode {
    Disabled,
    Press,
    PressRelease,
    ButtonMotion,
    AnyMotion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalMouseEncoding {
    Default,
    Utf8,
    Sgr,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalModes {
    pub alternate_screen: bool,
    pub cursor_hidden: bool,
    pub application_cursor: bool,
    pub application_keypad: bool,
    pub bracketed_paste: bool,
    pub mouse: TerminalMouseMode,
    pub mouse_encoding: TerminalMouseEncoding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub dimensions: PtyDimensions,
    pub scrollback_offset: usize,
    pub max_scrollback_offset: usize,
    pub cursor: (u16, u16),
    pub modes: TerminalModes,
    pub cells: Vec<TerminalCell>,
    pub plain_text: String,
}

pub struct TerminalEmulator {
    parser: vt100::Parser,
}

impl TerminalEmulator {
    pub fn new(
        dimensions: PtyDimensions,
        scrollback_lines: usize,
    ) -> Result<Self, TerminalEmulationError> {
        validate_size(dimensions)?;
        if scrollback_lines > MAX_TERMINAL_SCROLLBACK_LINES {
            return Err(TerminalEmulationError::ScrollbackTooLarge {
                requested: scrollback_lines,
                maximum: MAX_TERMINAL_SCROLLBACK_LINES,
            });
        }
        Ok(Self {
            parser: vt100::Parser::new(dimensions.rows, dimensions.columns, scrollback_lines),
        })
    }

    pub fn process(&mut self, bytes: &[u8]) -> Result<(), TerminalEmulationError> {
        if bytes.len() > MAX_TERMINAL_FEED_BYTES {
            return Err(TerminalEmulationError::FeedTooLarge(bytes.len()));
        }
        self.parser.process(bytes);
        Ok(())
    }

    pub fn resize(&mut self, dimensions: PtyDimensions) -> Result<(), TerminalEmulationError> {
        validate_size(dimensions)?;
        self.parser
            .screen_mut()
            .set_size(dimensions.rows, dimensions.columns);
        Ok(())
    }

    pub fn modes(&self) -> TerminalModes {
        modes(self.parser.screen())
    }

    pub fn snapshot(
        &mut self,
        scrollback_offset: usize,
    ) -> Result<TerminalSnapshot, TerminalEmulationError> {
        let prior = self.parser.screen().scrollback();
        self.parser.screen_mut().set_scrollback(usize::MAX);
        let maximum = self.parser.screen().scrollback();
        self.parser
            .screen_mut()
            .set_scrollback(scrollback_offset.min(maximum));
        let snapshot = snapshot_screen(self.parser.screen(), maximum)?;
        self.parser.screen_mut().set_scrollback(prior.min(maximum));
        Ok(snapshot)
    }
}

fn snapshot_screen(
    screen: &vt100::Screen,
    max_scrollback_offset: usize,
) -> Result<TerminalSnapshot, TerminalEmulationError> {
    let (rows, columns) = screen.size();
    let dimensions = PtyDimensions { columns, rows };
    validate_size(dimensions)?;
    let capacity = usize::from(rows) * usize::from(columns);
    let mut cells = Vec::with_capacity(capacity);
    for row in 0..rows {
        for column in 0..columns {
            let cell = screen
                .cell(row, column)
                .ok_or(TerminalEmulationError::MissingCell { row, column })?;
            if cell.contents().len() > MAX_TERMINAL_CELL_BYTES {
                return Err(TerminalEmulationError::CellTooLarge {
                    row,
                    column,
                    bytes: cell.contents().len(),
                    maximum: MAX_TERMINAL_CELL_BYTES,
                });
            }
            cells.push(TerminalCell {
                contents: cell.contents().to_owned(),
                foreground: color(cell.fgcolor()),
                background: color(cell.bgcolor()),
                bold: cell.bold(),
                dim: cell.dim(),
                italic: cell.italic(),
                underline: cell.underline(),
                inverse: cell.inverse(),
                wide: cell.is_wide(),
                wide_continuation: cell.is_wide_continuation(),
            });
        }
    }
    Ok(TerminalSnapshot {
        dimensions,
        scrollback_offset: screen.scrollback(),
        max_scrollback_offset,
        cursor: screen.cursor_position(),
        modes: modes(screen),
        cells,
        plain_text: screen.contents(),
    })
}

fn validate_size(dimensions: PtyDimensions) -> Result<(), TerminalEmulationError> {
    dimensions.validate()?;
    let cells = usize::from(dimensions.rows) * usize::from(dimensions.columns);
    if cells > MAX_TERMINAL_SNAPSHOT_CELLS {
        return Err(TerminalEmulationError::ScreenTooLarge {
            cells,
            maximum: MAX_TERMINAL_SNAPSHOT_CELLS,
        });
    }
    Ok(())
}

fn modes(screen: &vt100::Screen) -> TerminalModes {
    TerminalModes {
        alternate_screen: screen.alternate_screen(),
        cursor_hidden: screen.hide_cursor(),
        application_cursor: screen.application_cursor(),
        application_keypad: screen.application_keypad(),
        bracketed_paste: screen.bracketed_paste(),
        mouse: match screen.mouse_protocol_mode() {
            vt100::MouseProtocolMode::None => TerminalMouseMode::Disabled,
            vt100::MouseProtocolMode::Press => TerminalMouseMode::Press,
            vt100::MouseProtocolMode::PressRelease => TerminalMouseMode::PressRelease,
            vt100::MouseProtocolMode::ButtonMotion => TerminalMouseMode::ButtonMotion,
            vt100::MouseProtocolMode::AnyMotion => TerminalMouseMode::AnyMotion,
        },
        mouse_encoding: match screen.mouse_protocol_encoding() {
            vt100::MouseProtocolEncoding::Default => TerminalMouseEncoding::Default,
            vt100::MouseProtocolEncoding::Utf8 => TerminalMouseEncoding::Utf8,
            vt100::MouseProtocolEncoding::Sgr => TerminalMouseEncoding::Sgr,
        },
    }
}

fn color(color: vt100::Color) -> TerminalColor {
    match color {
        vt100::Color::Default => TerminalColor::Default,
        vt100::Color::Idx(index) => TerminalColor::Indexed(index),
        vt100::Color::Rgb(red, green, blue) => TerminalColor::Rgb(red, green, blue),
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TerminalEmulationError {
    #[error(transparent)]
    InvalidDimensions(#[from] PtySessionError),
    #[error("terminal scrollback {requested} exceeds {maximum} lines")]
    ScrollbackTooLarge { requested: usize, maximum: usize },
    #[error("terminal feed of {0} bytes exceeds the per-event bound")]
    FeedTooLarge(usize),
    #[error("terminal screen has {cells} cells, exceeding {maximum}")]
    ScreenTooLarge { cells: usize, maximum: usize },
    #[error("maintained terminal emulator omitted cell ({row}, {column})")]
    MissingCell { row: u16, column: u16 },
    #[error("terminal cell ({row}, {column}) has {bytes} bytes, exceeding {maximum}")]
    CellTooLarge {
        row: u16,
        column: u16,
        bytes: usize,
        maximum: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emulator(rows: u16, columns: u16, scrollback: usize) -> TerminalEmulator {
        TerminalEmulator::new(PtyDimensions { columns, rows }, scrollback).unwrap()
    }

    #[test]
    fn terminal_emulation_tracks_cursor_styles_unicode_and_modes() {
        let mut terminal = emulator(6, 30, 20);
        terminal
            .process(
                b"plain \x1b[1;3;4;38;2;1;2;3;48;5;17mstyled\x1b[0m\r\n\
                  \x1b[?1049h\x1b[2J\x1b[3;5Hmenu \xe2\x94\x82\x1b[?25l\
                  \x1b[?1h\x1b=\x1b[?2004h\x1b[?1002h\x1b[?1006h",
            )
            .unwrap();
        let snapshot = terminal.snapshot(0).unwrap();
        assert!(snapshot.modes.alternate_screen);
        assert!(snapshot.modes.cursor_hidden);
        assert!(snapshot.modes.application_cursor);
        assert!(snapshot.modes.application_keypad);
        assert!(snapshot.modes.bracketed_paste);
        assert_eq!(snapshot.modes.mouse, TerminalMouseMode::ButtonMotion);
        assert_eq!(snapshot.modes.mouse_encoding, TerminalMouseEncoding::Sgr);
        assert!(snapshot.plain_text.contains("menu │"));
        assert_eq!(snapshot.cursor, (2, 10));

        terminal.process(b"\x1b[?1049l").unwrap();
        let snapshot = terminal.snapshot(0).unwrap();
        let styled = &snapshot.cells[6];
        assert_eq!(styled.contents, "s");
        assert!(styled.bold && styled.italic && styled.underline);
        assert_eq!(styled.foreground, TerminalColor::Rgb(1, 2, 3));
        assert_eq!(styled.background, TerminalColor::Indexed(17));
    }

    #[test]
    fn ux_terminal_adapter_snapshot_is_complete_bounded_and_parser_owned() {
        let mut terminal = emulator(3, 12, 4);
        terminal
            .process(
                b"\x1b[1;1Hplain \x1b[1;3;4;38;2;4;5;6;48;5;17mstyled\x1b[0m\r\n\
                  wide:\xe7\x95\x8c\r\nthird\r\nfourth\x1b[?25l",
            )
            .unwrap();
        let snapshot = terminal.snapshot(1).unwrap();
        assert_eq!(
            snapshot.cells.len(),
            usize::from(snapshot.dimensions.rows) * usize::from(snapshot.dimensions.columns)
        );
        assert!(snapshot.cells.iter().any(|cell| {
            cell.contents == "s"
                && cell.bold
                && cell.italic
                && cell.underline
                && cell.foreground == TerminalColor::Rgb(4, 5, 6)
                && cell.background == TerminalColor::Indexed(17)
        }));
        assert!(snapshot.cells.iter().any(|cell| cell.wide));
        assert!(snapshot.cells.iter().any(|cell| cell.wide_continuation));
        assert!(snapshot.modes.cursor_hidden);
        assert_eq!(snapshot.scrollback_offset, 1);
        assert!(snapshot.max_scrollback_offset >= snapshot.scrollback_offset);
        assert!(
            snapshot
                .cells
                .iter()
                .all(|cell| cell.contents.len() <= MAX_TERMINAL_CELL_BYTES)
        );
    }

    #[test]
    fn terminal_emulation_bounds_and_restores_scrollback_snapshots() {
        let mut terminal = emulator(3, 12, 4);
        terminal
            .process(b"one\r\ntwo\r\nthree\r\nfour\r\nfive")
            .unwrap();
        let current = terminal.snapshot(0).unwrap();
        assert!(current.plain_text.contains("five"));
        assert!(current.max_scrollback_offset > 0);
        let history = terminal.snapshot(usize::MAX).unwrap();
        assert_eq!(history.scrollback_offset, history.max_scrollback_offset);
        assert!(history.plain_text.contains("one") || history.plain_text.contains("two"));
        let current_again = terminal.snapshot(0).unwrap();
        assert_eq!(current_again.plain_text, current.plain_text);

        terminal
            .resize(PtyDimensions {
                columns: 20,
                rows: 5,
            })
            .unwrap();
        assert_eq!(
            terminal.snapshot(0).unwrap().dimensions,
            PtyDimensions {
                columns: 20,
                rows: 5
            }
        );
        assert!(matches!(
            TerminalEmulator::new(
                PtyDimensions {
                    columns: 1_000,
                    rows: 1_000
                },
                0
            ),
            Err(TerminalEmulationError::ScreenTooLarge { .. })
        ));
        assert_eq!(
            terminal.process(&vec![0; MAX_TERMINAL_FEED_BYTES + 1]),
            Err(TerminalEmulationError::FeedTooLarge(
                MAX_TERMINAL_FEED_BYTES + 1
            ))
        );
    }
}
