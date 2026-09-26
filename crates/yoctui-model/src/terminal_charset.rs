#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum CharacterSet {
    #[default]
    Ascii,
    DecSpecialGraphics,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum StreamState {
    #[default]
    Ground,
    Escape,
    EscapeIntermediate,
    DesignateG0,
    DesignateG1,
    Csi,
    Osc,
    ControlString,
    OscEscape,
    ControlStringEscape,
}

/// Normalizes the VT100 character-set controls that the admitted `vt100`
/// crate deliberately ignores. All other bytes remain in their original
/// order for the maintained parser to interpret.
pub(crate) struct TerminalCharsetNormalizer {
    g0: CharacterSet,
    g1: CharacterSet,
    use_g1: bool,
    state: StreamState,
}

impl Default for TerminalCharsetNormalizer {
    fn default() -> Self {
        Self {
            g0: CharacterSet::Ascii,
            g1: CharacterSet::Ascii,
            use_g1: false,
            state: StreamState::Ground,
        }
    }
}

impl TerminalCharsetNormalizer {
    pub(crate) fn normalize(&mut self, input: &[u8]) -> Vec<u8> {
        let mut output = Vec::with_capacity(input.len());
        for &byte in input {
            self.push(byte, &mut output);
        }
        output
    }

    fn push(&mut self, byte: u8, output: &mut Vec<u8>) {
        output.push(byte);
        match self.state {
            StreamState::Ground => self.ground(byte, output),
            StreamState::Escape => self.escape(byte),
            StreamState::EscapeIntermediate => self.escape_intermediate(byte),
            StreamState::DesignateG0 => self.designate(byte, false),
            StreamState::DesignateG1 => self.designate(byte, true),
            StreamState::Csi => self.csi(byte),
            StreamState::Osc => self.osc(byte),
            StreamState::ControlString => self.control_string(byte),
            StreamState::OscEscape => self.string_escape(byte, true),
            StreamState::ControlStringEscape => self.string_escape(byte, false),
        }
    }

    fn ground(&mut self, byte: u8, output: &mut Vec<u8>) {
        match byte {
            0x0e => self.use_g1 = true,
            0x0f => self.use_g1 = false,
            0x1b => self.state = StreamState::Escape,
            0x20..=0x7e if self.selected() == CharacterSet::DecSpecialGraphics => {
                if let Some(character) = dec_special_graphic(byte) {
                    output.pop();
                    let mut encoded = [0; 4];
                    output.extend_from_slice(character.encode_utf8(&mut encoded).as_bytes());
                }
            }
            _ => {}
        }
    }

    fn escape(&mut self, byte: u8) {
        self.state = match byte {
            b'[' => StreamState::Csi,
            b']' => StreamState::Osc,
            b'P' | b'X' | b'^' | b'_' => StreamState::ControlString,
            b'(' => StreamState::DesignateG0,
            b')' => StreamState::DesignateG1,
            0x20..=0x2f => StreamState::EscapeIntermediate,
            0x30..=0x7e => {
                if byte == b'c' {
                    self.reset();
                }
                StreamState::Ground
            }
            0x1b => StreamState::Escape,
            _ => StreamState::Escape,
        };
    }

    fn escape_intermediate(&mut self, byte: u8) {
        if byte == 0x1b {
            self.state = StreamState::Escape;
        } else if (0x30..=0x7e).contains(&byte) || matches!(byte, 0x18 | 0x1a) {
            self.state = StreamState::Ground;
        }
    }

    fn designate(&mut self, byte: u8, g1: bool) {
        if byte == 0x1b {
            self.state = StreamState::Escape;
            return;
        }
        if matches!(byte, 0x18 | 0x1a) {
            self.state = StreamState::Ground;
            return;
        }
        if !(0x30..=0x7e).contains(&byte) {
            return;
        }
        let charset = if byte == b'0' {
            CharacterSet::DecSpecialGraphics
        } else {
            CharacterSet::Ascii
        };
        if g1 {
            self.g1 = charset;
        } else {
            self.g0 = charset;
        }
        self.state = StreamState::Ground;
    }

    fn csi(&mut self, byte: u8) {
        if byte == 0x1b {
            self.state = StreamState::Escape;
        } else if (0x40..=0x7e).contains(&byte) || matches!(byte, 0x18 | 0x1a) {
            self.state = StreamState::Ground;
        }
    }

    fn osc(&mut self, byte: u8) {
        self.state = match byte {
            0x07 | 0x18 | 0x1a => StreamState::Ground,
            0x1b => StreamState::OscEscape,
            _ => StreamState::Osc,
        };
    }

    fn control_string(&mut self, byte: u8) {
        self.state = match byte {
            0x18 | 0x1a => StreamState::Ground,
            0x1b => StreamState::ControlStringEscape,
            _ => StreamState::ControlString,
        };
    }

    fn string_escape(&mut self, byte: u8, osc: bool) {
        self.state = if byte == b'\\' || matches!(byte, 0x18 | 0x1a) {
            StreamState::Ground
        } else if byte == 0x1b {
            if osc {
                StreamState::OscEscape
            } else {
                StreamState::ControlStringEscape
            }
        } else if osc {
            StreamState::Osc
        } else {
            StreamState::ControlString
        };
    }

    fn selected(&self) -> CharacterSet {
        if self.use_g1 { self.g1 } else { self.g0 }
    }

    fn reset(&mut self) {
        self.g0 = CharacterSet::Ascii;
        self.g1 = CharacterSet::Ascii;
        self.use_g1 = false;
    }
}

fn dec_special_graphic(byte: u8) -> Option<char> {
    Some(match byte {
        b'_' => ' ',
        b'`' => '◆',
        b'a' => '▒',
        b'b' => '␉',
        b'c' => '␌',
        b'd' => '␍',
        b'e' => '␊',
        b'f' => '°',
        b'g' => '±',
        b'h' => '␤',
        b'i' => '␋',
        b'j' => '┘',
        b'k' => '┐',
        b'l' => '┌',
        b'm' => '└',
        b'n' => '┼',
        b'o' => '⎺',
        b'p' => '⎻',
        b'q' => '─',
        b'r' => '⎼',
        b's' => '⎽',
        b't' => '├',
        b'u' => '┤',
        b'v' => '┴',
        b'w' => '┬',
        b'x' => '│',
        b'y' => '≤',
        b'z' => '≥',
        b'{' => 'π',
        b'|' => '≠',
        b'}' => '£',
        b'~' => '·',
        _ => return None,
    })
}
