pub const KEYMAP_SCHEMA_VERSION: u16 = 1;
pub const MAX_KEYMAP_OVERRIDES: usize = 256;
pub const MAX_BINDINGS_PER_ACTION: usize = 8;
pub const MAX_KEY_SEQUENCE_STROKES: usize = 3;
pub const MAX_EFFECTIVE_KEYMAP_REPORT_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KeyStroke {
    Char(char),
    Esc,
    Enter,
    Backspace,
    Tab,
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,
    CtrlB,
    CtrlC,
    CtrlP,
    CtrlS,
    CtrlU,
    CtrlV,
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F12,
}

impl KeyStroke {
    pub const fn is_terminal_prefix(self) -> bool {
        matches!(self, Self::CtrlB)
    }
}

impl fmt::Display for KeyStroke {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Char(character) => return write!(formatter, "{character}"),
            Self::Esc => "Esc",
            Self::Enter => "Enter",
            Self::Backspace => "Backspace",
            Self::Tab => "Tab",
            Self::BackTab => "Shift+Tab",
            Self::Up => "Up",
            Self::Down => "Down",
            Self::Left => "Left",
            Self::Right => "Right",
            Self::Home => "Home",
            Self::End => "End",
            Self::PageUp => "PageUp",
            Self::PageDown => "PageDown",
            Self::CtrlB => "Ctrl+B",
            Self::CtrlC => "Ctrl+C",
            Self::CtrlP => "Ctrl+P",
            Self::CtrlS => "Ctrl+S",
            Self::CtrlU => "Ctrl+U",
            Self::CtrlV => "Ctrl+V",
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::F4 => "F4",
            Self::F5 => "F5",
            Self::F6 => "F6",
            Self::F7 => "F7",
            Self::F8 => "F8",
            Self::F9 => "F9",
            Self::F10 => "F10",
            Self::F12 => "F12",
        };
        formatter.write_str(name)
    }
}

impl FromStr for KeyStroke {
    type Err = KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        let named = match value.to_ascii_lowercase().as_str() {
            "esc" | "escape" => Some(Self::Esc),
            "enter" | "return" => Some(Self::Enter),
            "backspace" => Some(Self::Backspace),
            "tab" => Some(Self::Tab),
            "shift+tab" | "backtab" => Some(Self::BackTab),
            "up" => Some(Self::Up),
            "down" => Some(Self::Down),
            "left" => Some(Self::Left),
            "right" => Some(Self::Right),
            "home" => Some(Self::Home),
            "end" => Some(Self::End),
            "pageup" | "page+up" => Some(Self::PageUp),
            "pagedown" | "page+down" => Some(Self::PageDown),
            "ctrl+b" => Some(Self::CtrlB),
            "ctrl+c" => Some(Self::CtrlC),
            "ctrl+p" => Some(Self::CtrlP),
            "ctrl+s" => Some(Self::CtrlS),
            "ctrl+u" => Some(Self::CtrlU),
            "ctrl+v" => Some(Self::CtrlV),
            "f1" => Some(Self::F1),
            "f2" => Some(Self::F2),
            "f3" => Some(Self::F3),
            "f4" => Some(Self::F4),
            "f5" => Some(Self::F5),
            "f6" => Some(Self::F6),
            "f7" => Some(Self::F7),
            "f8" => Some(Self::F8),
            "f9" => Some(Self::F9),
            "f10" => Some(Self::F10),
            "f12" => Some(Self::F12),
            _ => None,
        };
        if let Some(stroke) = named {
            return Ok(stroke);
        }
        let mut characters = value.chars();
        let Some(character) = characters.next() else {
            return Err(KeymapError::InvalidStroke(value.into()));
        };
        if characters.next().is_some() || character.is_control() || character.is_whitespace() {
            return Err(KeymapError::InvalidStroke(value.into()));
        }
        Ok(Self::Char(character))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeySequence(Vec<KeyStroke>);

impl KeySequence {
    pub fn new(strokes: Vec<KeyStroke>) -> Result<Self, KeymapError> {
        if strokes.is_empty() || strokes.len() > MAX_KEY_SEQUENCE_STROKES {
            return Err(KeymapError::InvalidSequenceLength(strokes.len()));
        }
        Ok(Self(strokes))
    }

    pub fn single(stroke: KeyStroke) -> Self {
        Self(vec![stroke])
    }

    pub fn strokes(&self) -> &[KeyStroke] {
        &self.0
    }

    pub fn starts_with(&self, other: &Self) -> bool {
        self.0.starts_with(&other.0)
    }

    pub fn pushed(&self, stroke: KeyStroke) -> Result<Self, KeymapError> {
        let mut strokes = self.0.clone();
        strokes.push(stroke);
        Self::new(strokes)
    }
}

impl fmt::Display for KeySequence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, stroke) in self.0.iter().enumerate() {
            if index > 0 {
                formatter.write_str(" ")?;
            }
            write!(formatter, "{stroke}")?;
        }
        Ok(())
    }
}

impl FromStr for KeySequence {
    type Err = KeymapError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let strokes = value
            .split_ascii_whitespace()
            .map(KeyStroke::from_str)
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(strokes)
    }
}

impl Serialize for KeySequence {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for KeySequence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value.parse().map_err(serde::de::Error::custom)
    }
}
