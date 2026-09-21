#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RawAdditionalArguments {
    arguments: Vec<String>,
}

impl RawAdditionalArguments {
    pub fn parse(input: &str) -> Result<Self, RawArgvError> {
        tokenize_raw_additional_arguments(input).map(|arguments| Self { arguments })
    }

    pub fn as_slice(&self) -> &[String] {
        &self.arguments
    }

    pub fn from_vec(arguments: Vec<String>) -> Result<Self, RawArgvError> {
        let mut validated = Vec::with_capacity(arguments.len());
        for argument in arguments {
            push_raw_argv_argument(&mut validated, argument)?;
        }
        Ok(Self {
            arguments: validated,
        })
    }

    pub fn into_vec(self) -> Vec<String> {
        self.arguments
    }

    fn validate(&self) -> Result<(), RawArgvError> {
        Self::from_vec(self.arguments.clone()).map(|_| ())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawArgvEditor {
    pub editor: PopupEditor,
    pub validated: Option<RawAdditionalArguments>,
    pub validation_error: Option<RawArgvError>,
}

impl RawArgvEditor {
    pub fn new(input: impl Into<String>) -> Result<Self, RawArgvError> {
        let input = input.into();
        validate_raw_argv_input_bound(&input)?;
        Ok(Self {
            editor: PopupEditor::new(input),
            validated: None,
            validation_error: None,
        })
    }

    pub fn replace_input(&mut self, input: impl Into<String>) -> Result<(), RawArgvError> {
        let input = input.into();
        validate_raw_argv_input_bound(&input)?;
        self.editor = PopupEditor::new(input);
        self.validated = None;
        self.validation_error = None;
        Ok(())
    }

    pub fn apply(&mut self, command: PopupEditorCommand) -> Result<(), RawArgvError> {
        let previous = self.editor.clone();
        let invalidates = match command {
            PopupEditorCommand::ToggleInsert => {
                self.editor.toggle_insert();
                false
            }
            PopupEditorCommand::ToggleVisual => {
                let mode = if self.editor.mode() == crate::TextAreaMode::Visual {
                    crate::TextAreaMode::Normal
                } else {
                    crate::TextAreaMode::Visual
                };
                self.editor.set_mode(mode);
                false
            }
            PopupEditorCommand::Insert(character) if self.editor.editing => {
                self.editor.insert(&character.to_string());
                true
            }
            PopupEditorCommand::Insert(_) => false,
            PopupEditorCommand::Newline if self.editor.editing => {
                self.editor.insert("\n");
                true
            }
            PopupEditorCommand::Newline => false,
            PopupEditorCommand::Backspace if self.editor.editing => {
                self.editor.backspace();
                true
            }
            PopupEditorCommand::Backspace => false,
            PopupEditorCommand::Delete => {
                self.editor.delete_forward();
                true
            }
            PopupEditorCommand::Left => {
                self.editor.left();
                false
            }
            PopupEditorCommand::Right => {
                self.editor.right();
                false
            }
            PopupEditorCommand::WordLeft => {
                self.editor.move_cursor(crate::TextAreaMotion::WordLeft);
                false
            }
            PopupEditorCommand::WordRight => {
                self.editor.move_cursor(crate::TextAreaMotion::WordRight);
                false
            }
            PopupEditorCommand::Up => {
                self.editor.up();
                false
            }
            PopupEditorCommand::Down => {
                self.editor.down();
                false
            }
            PopupEditorCommand::Home => {
                self.editor.home();
                false
            }
            PopupEditorCommand::End => {
                self.editor.end();
                false
            }
            PopupEditorCommand::PageUp => {
                self.editor.move_cursor(crate::TextAreaMotion::PageUp);
                false
            }
            PopupEditorCommand::PageDown => {
                self.editor.move_cursor(crate::TextAreaMotion::PageDown);
                false
            }
            PopupEditorCommand::Undo => self.editor.undo(),
            PopupEditorCommand::Redo => self.editor.redo(),
            PopupEditorCommand::SelectPosition {
                line,
                column,
                extend,
            } => {
                self.editor.select_position(line, column, extend);
                false
            }
            PopupEditorCommand::PasteText { text, source } if self.editor.editing => {
                self.editor.paste_text(&text, source).is_ok()
            }
            PopupEditorCommand::PasteText { .. } => false,
            PopupEditorCommand::SelectValue => {
                self.editor.select_range(0, self.editor.text.len());
                false
            }
            PopupEditorCommand::Copy => {
                self.editor.copy_selection_or_line();
                false
            }
            PopupEditorCommand::Paste if self.editor.editing => {
                self.editor.paste();
                true
            }
            PopupEditorCommand::Paste => false,
        };
        if let Err(error) = validate_raw_argv_input_bound(&self.editor.text) {
            self.editor = previous;
            self.validation_error = Some(error.clone());
            return Err(error);
        }
        if invalidates {
            self.validated = None;
            self.validation_error = None;
        }
        Ok(())
    }

    pub fn validate(&mut self) -> Result<&RawAdditionalArguments, RawArgvError> {
        match RawAdditionalArguments::parse(&self.editor.text) {
            Ok(arguments) => {
                self.validated = Some(arguments);
                self.validation_error = None;
                Ok(self.validated.as_ref().expect("just installed"))
            }
            Err(error) => {
                self.validated = None;
                self.validation_error = Some(error.clone());
                Err(error)
            }
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawArgvError {
    #[error("Raw additional-argument input is {bytes} bytes; maximum is {maximum}")]
    InputTooLong { bytes: usize, maximum: usize },
    #[error("Raw additional arguments contain a control character at byte {byte}")]
    ControlCharacter { byte: usize },
    #[error("Raw additional arguments end with an escape opened at byte {byte}")]
    UnterminatedEscape { byte: usize },
    #[error("Raw additional arguments have an unterminated {quote} quote opened at byte {byte}")]
    UnterminatedQuote { quote: char, byte: usize },
    #[error("Raw additional arguments escape a non-ordinary character at byte {byte}")]
    InvalidEscape { byte: usize, character: char },
    #[error("Raw additional argument {argument} contains forbidden operator {operator:?}")]
    ForbiddenOperator { argument: usize, operator: String },
    #[error("Raw additional argument {argument} has an empty option name")]
    EmptyOptionName { argument: usize },
    #[error("Raw additional arguments contain {count} elements; maximum is {maximum}")]
    TooManyArguments { count: usize, maximum: usize },
    #[error("Raw additional argument {argument} is {bytes} bytes; maximum is {maximum}")]
    ArgumentTooLong {
        argument: usize,
        bytes: usize,
        maximum: usize,
    },
    #[error("Raw additional arguments contain {bytes} aggregate bytes; maximum is {maximum}")]
    AggregateTooLong { bytes: usize, maximum: usize },
}

fn tokenize_raw_additional_arguments(input: &str) -> Result<Vec<String>, RawArgvError> {
    validate_raw_argv_input_bound(input)?;
    if let Some((byte, _)) = input
        .char_indices()
        .find(|(_, character)| character.is_control())
    {
        return Err(RawArgvError::ControlCharacter { byte });
    }

    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut token_started = false;
    let mut quote: Option<(char, usize)> = None;
    let mut escape = None;
    for (byte, character) in input.char_indices() {
        if let Some(escape_byte) = escape.take() {
            if !raw_ordinary_escaped_character(character) {
                return Err(RawArgvError::InvalidEscape {
                    byte: escape_byte,
                    character,
                });
            }
            current.push(character);
            token_started = true;
            validate_raw_argv_element_bound(arguments.len(), &current)?;
            continue;
        }
        if character == '\\' {
            escape = Some(byte);
            token_started = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            match quote {
                Some((active, _)) if active == character => quote = None,
                None => quote = Some((character, byte)),
                Some(_) => current.push(character),
            }
            token_started = true;
            continue;
        }
        if character.is_whitespace() && quote.is_none() {
            if token_started {
                push_raw_argv_argument(&mut arguments, std::mem::take(&mut current))?;
                token_started = false;
            }
            continue;
        }
        current.push(character);
        token_started = true;
        validate_raw_argv_element_bound(arguments.len(), &current)?;
    }

    if let Some(byte) = escape {
        return Err(RawArgvError::UnterminatedEscape { byte });
    }
    if let Some((quote, byte)) = quote {
        return Err(RawArgvError::UnterminatedQuote { quote, byte });
    }
    if token_started {
        push_raw_argv_argument(&mut arguments, current)?;
    }
    Ok(arguments)
}

fn validate_raw_argv_input_bound(input: &str) -> Result<(), RawArgvError> {
    if input.len() > MAX_RAW_ADDITIONAL_INPUT_BYTES {
        Err(RawArgvError::InputTooLong {
            bytes: input.len(),
            maximum: MAX_RAW_ADDITIONAL_INPUT_BYTES,
        })
    } else {
        Ok(())
    }
}

fn validate_raw_argv_element_bound(argument: usize, value: &str) -> Result<(), RawArgvError> {
    if value.len() > MAX_RAW_ADDITIONAL_ARGUMENT_BYTES {
        Err(RawArgvError::ArgumentTooLong {
            argument,
            bytes: value.len(),
            maximum: MAX_RAW_ADDITIONAL_ARGUMENT_BYTES,
        })
    } else {
        Ok(())
    }
}

fn push_raw_argv_argument(
    arguments: &mut Vec<String>,
    argument: String,
) -> Result<(), RawArgvError> {
    let index = arguments.len();
    validate_raw_argv_element_bound(index, &argument)?;
    if index == MAX_RAW_ADDITIONAL_ARGUMENTS {
        return Err(RawArgvError::TooManyArguments {
            count: index + 1,
            maximum: MAX_RAW_ADDITIONAL_ARGUMENTS,
        });
    }
    if empty_raw_option_name(&argument) {
        return Err(RawArgvError::EmptyOptionName { argument: index });
    }
    if let Some(operator) = forbidden_raw_argv_operator(&argument) {
        return Err(RawArgvError::ForbiddenOperator {
            argument: index,
            operator: operator.into(),
        });
    }
    let aggregate = arguments
        .iter()
        .map(String::len)
        .sum::<usize>()
        .saturating_add(argument.len());
    if aggregate > MAX_RAW_ADDITIONAL_AGGREGATE_BYTES {
        return Err(RawArgvError::AggregateTooLong {
            bytes: aggregate,
            maximum: MAX_RAW_ADDITIONAL_AGGREGATE_BYTES,
        });
    }
    arguments.push(argument);
    Ok(())
}

fn raw_ordinary_escaped_character(character: char) -> bool {
    !character.is_control() && !matches!(character, '|' | '<' | '>' | ';' | '`')
}

fn empty_raw_option_name(argument: &str) -> bool {
    argument.starts_with("-=") || argument.starts_with("--=")
}

fn forbidden_raw_argv_operator(argument: &str) -> Option<&'static str> {
    ["$(", "&&", "||", ">>", "|", ">", "<", ";", "`"]
        .into_iter()
        .find(|operator| argument.contains(operator))
}

fn valid_raw_identifier(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && !value.starts_with('-')
        && !matches!(value, "." | "..")
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'_' | b'.'))
}

fn valid_raw_target(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RAW_TARGET_BYTES
        && value
            .split('/')
            .all(|segment| valid_raw_identifier(segment, MAX_RAW_TARGET_BYTES))
}

fn valid_raw_file(value: &str) -> bool {
    if value.is_empty()
        || value.len() > MAX_RAW_FILE_BYTES
        || value.trim() != value
        || value.starts_with('-')
        || value.ends_with('/')
        || value.contains("//")
        || value.chars().any(char::is_control)
        || contains_raw_parameter_shell_syntax(value)
    {
        return false;
    }
    let mut normal_components = 0;
    for component in std::path::Path::new(value).components() {
        match component {
            Component::Normal(_) => normal_components += 1,
            Component::RootDir => {}
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => return false,
        }
    }
    normal_components > 0
}

fn valid_raw_text_parameter(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RAW_PARAMETER_TEXT_BYTES
        && value.trim() == value
        && !value.starts_with('-')
        && !value.chars().any(char::is_control)
        && !contains_raw_parameter_shell_syntax(value)
}

fn contains_raw_parameter_shell_syntax(value: &str) -> bool {
    value.chars().any(|character| {
        matches!(
            character,
            '|' | '&'
                | ';'
                | '<'
                | '>'
                | '`'
                | '$'
                | '('
                | ')'
                | '{'
                | '}'
                | '['
                | ']'
                | '*'
                | '?'
                | '~'
                | '!'
                | '#'
                | '\\'
                | '\''
                | '"'
        )
    })
}

fn parameter_placeholder<'a>(
    parameters: &'a [RawParameter],
    id: &RawParameterId,
) -> Option<&'a str> {
    parameters
        .iter()
        .find(|parameter| &parameter.id == id)
        .map(|parameter| parameter.placeholder.as_str())
}

fn contains_shell_syntax(value: &str) -> bool {
    value
        .chars()
        .any(|character| matches!(character, '|' | '&' | ';' | '<' | '>' | '`' | '$'))
}
