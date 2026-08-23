use crate::{
    CapabilityId, CapabilityState, DaemonCompatibilitySnapshot, PopupEditor, PopupEditorCommand,
    Recipe, RecipeMetadata,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    path::{Component, Path, PathBuf},
    sync::OnceLock,
};
use thiserror::Error;

pub const RAW_CATALOG_VERSION: u16 = 1;
pub const MAX_RAW_CATEGORIES: usize = 64;
pub const MAX_RAW_COMMANDS: usize = 512;
pub const MAX_RAW_PARAMETERS: usize = 32;
pub const MAX_RAW_CAPABILITY_REQUIREMENTS: usize = 32;
pub const MAX_RAW_ID_BYTES: usize = 96;
pub const MAX_RAW_LABEL_BYTES: usize = 160;
pub const MAX_RAW_TEXT_BYTES: usize = 4_096;
pub const MAX_RAW_REFERENCE_COMMAND_BYTES: usize = 1_024;
pub const MAX_RAW_ARG_BYTES: usize = 512;
pub const MAX_RAW_RECIPE_BYTES: usize = 256;
pub const MAX_RAW_IMAGE_BYTES: usize = 256;
pub const MAX_RAW_TARGET_BYTES: usize = 256;
pub const MAX_RAW_TASK_BYTES: usize = 256;
pub const MAX_RAW_UI_BYTES: usize = 128;
pub const MAX_RAW_FILE_BYTES: usize = 4_096;
pub const MAX_RAW_INTEGER_INPUT_BYTES: usize = 10;
pub const MAX_RAW_PARAMETER_TEXT_BYTES: usize = 512;
pub const MAX_RAW_MULTICONFIG_BYTES: usize = 128;
pub const MAX_RAW_INTEGER: u32 = u32::MAX;
pub const MAX_RAW_ADDITIONAL_INPUT_BYTES: usize = 12_288;
pub const MAX_RAW_ADDITIONAL_ARGUMENTS: usize = 64;
pub const MAX_RAW_ADDITIONAL_ARGUMENT_BYTES: usize = 512;
pub const MAX_RAW_ADDITIONAL_AGGREGATE_BYTES: usize = 8_192;
pub const MAX_RAW_PREVIEW_ARGUMENTS: usize = 128;
pub const MAX_RAW_PREVIEW_ARGUMENT_BYTES: usize = 8_192;
pub const MAX_RAW_SEARCH_BYTES: usize = 512;
pub const MAX_RAW_FAVORITES: usize = 256;
pub const MAX_RAW_HISTORY_STUBS: usize = 256;
pub const MAX_RAW_VIEW_DEPTH: usize = 8;
pub const MAX_RAW_EXECUTION_ID_BYTES: usize = 96;
pub const MAX_RAW_EXECUTION_REQUESTS: usize = 64;
pub const MAX_RAW_EXECUTION_MESSAGE_BYTES: usize = 4_096;
pub const MAX_RAW_OUTPUT_CHUNK_BYTES: usize = 64 * 1_024;
pub const MAX_RAW_OUTPUT_RETAINED_BYTES: usize = 1_024 * 1_024;
pub const MAX_RAW_OUTPUT_RETAINED_LINES: usize = 10_000;

pub fn builtin_raw_catalog() -> &'static RawCatalog {
    static CATALOG: OnceLock<RawCatalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        RawCatalog::builtin()
            .normalize()
            .expect("the generated Raw catalog is validated by traceability tests")
    })
}

macro_rules! raw_id {
    ($name:ident, $field:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, RawCatalogError> {
                let value = value.into();
                if !valid_id(&value) {
                    return Err(RawCatalogError::InvalidIdentity {
                        field: $field,
                        value,
                    });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = RawCatalogError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

raw_id!(RawCategoryId, "category");
raw_id!(RawCommandId, "command");
raw_id!(RawReferenceId, "reference");
raw_id!(RawParameterId, "parameter");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawCategoryKind {
    Executable,
    ReferenceOnly,
    Conceptual,
    CompanionTools,
    Favorites,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCategory {
    pub id: RawCategoryId,
    pub label: String,
    pub reference_heading: String,
    pub kind: RawCategoryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawParameterKind {
    Recipe,
    Image,
    Target,
    Task,
    UserInterface,
    File,
    Integer,
    Text,
    Multiconfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawParameterPresence {
    Required,
    Optional,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawParameter {
    pub id: RawParameterId,
    pub label: String,
    /// Exact token shown in the immutable reference, such as `<recipe>`.
    pub placeholder: String,
    pub kind: RawParameterKind,
    pub presence: RawParameterPresence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum RawParameterValue {
    Recipe(String),
    Image(String),
    Target(String),
    Task(String),
    UserInterface(String),
    File(String),
    Integer(u32),
    Text(String),
    Multiconfig(String),
}

impl RawParameterValue {
    pub const fn kind(&self) -> RawParameterKind {
        match self {
            Self::Recipe(_) => RawParameterKind::Recipe,
            Self::Image(_) => RawParameterKind::Image,
            Self::Target(_) => RawParameterKind::Target,
            Self::Task(_) => RawParameterKind::Task,
            Self::UserInterface(_) => RawParameterKind::UserInterface,
            Self::File(_) => RawParameterKind::File,
            Self::Integer(_) => RawParameterKind::Integer,
            Self::Text(_) => RawParameterKind::Text,
            Self::Multiconfig(_) => RawParameterKind::Multiconfig,
        }
    }

    pub fn argument(&self) -> String {
        match self {
            Self::Recipe(value)
            | Self::Image(value)
            | Self::Target(value)
            | Self::Task(value)
            | Self::UserInterface(value)
            | Self::File(value)
            | Self::Text(value)
            | Self::Multiconfig(value) => value.clone(),
            Self::Integer(value) => value.to_string(),
        }
    }
}

impl RawParameter {
    pub fn parse_value(&self, input: &str) -> Result<Option<RawParameterValue>, RawParameterError> {
        if input.is_empty() {
            return match self.presence {
                RawParameterPresence::Optional => Ok(None),
                RawParameterPresence::Required => Err(RawParameterError::Required {
                    parameter: self.id.clone(),
                }),
            };
        }

        let value = match self.kind {
            RawParameterKind::Recipe => RawParameterValue::Recipe(input.to_owned()),
            RawParameterKind::Image => RawParameterValue::Image(input.to_owned()),
            RawParameterKind::Target => RawParameterValue::Target(input.to_owned()),
            RawParameterKind::Task => RawParameterValue::Task(input.to_owned()),
            RawParameterKind::UserInterface => RawParameterValue::UserInterface(input.to_owned()),
            RawParameterKind::File => RawParameterValue::File(input.to_owned()),
            RawParameterKind::Integer => {
                if input.len() > MAX_RAW_INTEGER_INPUT_BYTES
                    || !input.bytes().all(|byte| byte.is_ascii_digit())
                {
                    return Err(self.invalid(RawParameterInvalidReason::InvalidInteger));
                }
                let value = input
                    .parse::<u64>()
                    .map_err(|_| self.invalid(RawParameterInvalidReason::InvalidInteger))?;
                if value > u64::from(MAX_RAW_INTEGER) {
                    return Err(self.invalid(RawParameterInvalidReason::IntegerOutOfRange));
                }
                RawParameterValue::Integer(value as u32)
            }
            RawParameterKind::Text => RawParameterValue::Text(input.to_owned()),
            RawParameterKind::Multiconfig => RawParameterValue::Multiconfig(input.to_owned()),
        };
        self.validate_value(&value)?;
        Ok(Some(value))
    }

    pub fn validate_value(&self, value: &RawParameterValue) -> Result<(), RawParameterError> {
        if self.kind != value.kind() {
            return Err(RawParameterError::KindMismatch {
                parameter: self.id.clone(),
                expected: self.kind,
                actual: value.kind(),
            });
        }

        let valid = match value {
            RawParameterValue::Recipe(value) => valid_raw_identifier(value, MAX_RAW_RECIPE_BYTES),
            RawParameterValue::Image(value) => valid_raw_identifier(value, MAX_RAW_IMAGE_BYTES),
            RawParameterValue::Target(value) => valid_raw_target(value),
            RawParameterValue::Task(value) => valid_raw_identifier(value, MAX_RAW_TASK_BYTES),
            RawParameterValue::UserInterface(value) => {
                valid_raw_identifier(value, MAX_RAW_UI_BYTES)
            }
            RawParameterValue::File(value) => valid_raw_file(value),
            RawParameterValue::Integer(_) => true,
            RawParameterValue::Text(value) => valid_raw_text_parameter(value),
            RawParameterValue::Multiconfig(value) => {
                valid_raw_identifier(value, MAX_RAW_MULTICONFIG_BYTES)
            }
        };
        if valid {
            Ok(())
        } else {
            Err(self.invalid(RawParameterInvalidReason::InvalidValue))
        }
    }

    fn invalid(&self, reason: RawParameterInvalidReason) -> RawParameterError {
        RawParameterError::InvalidValue {
            parameter: self.id.clone(),
            kind: self.kind,
            reason,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawParameterInvalidReason {
    InvalidValue,
    InvalidInteger,
    IntegerOutOfRange,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawParameterError {
    #[error("Raw parameter {parameter} is required")]
    Required { parameter: RawParameterId },
    #[error("Raw parameter {parameter} expects {expected:?}, but received {actual:?}")]
    KindMismatch {
        parameter: RawParameterId,
        expected: RawParameterKind,
        actual: RawParameterKind,
    },
    #[error("Raw parameter {parameter} has invalid {kind:?} input: {reason:?}")]
    InvalidValue {
        parameter: RawParameterId,
        kind: RawParameterKind,
        reason: RawParameterInvalidReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RawSelectorIdentity {
    Recipe { name: String, file: Option<PathBuf> },
    Image(String),
    Target(String),
    Task { recipe: String, task: String },
    Multiconfig(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSelectorChoice {
    pub identity: RawSelectorIdentity,
    pub value: RawParameterValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawSelectorInventory {
    Unavailable { reason: String },
    Available { choices: Vec<RawSelectorChoice> },
}

impl RawSelectorInventory {
    pub fn choices(&self) -> Option<&[RawSelectorChoice]> {
        match self {
            Self::Unavailable { .. } => None,
            Self::Available { choices } => Some(choices),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawSelectorAuthority {
    pub recipes: RawSelectorInventory,
    pub images: RawSelectorInventory,
    pub targets: RawSelectorInventory,
    pub tasks: RawSelectorInventory,
    pub multiconfigs: RawSelectorInventory,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RawSelectorSources<'a> {
    pub recipes: Option<&'a [Recipe]>,
    pub images: Option<&'a [String]>,
    pub current_target: Option<&'a str>,
    pub recent_targets: Option<&'a [String]>,
    pub selected_recipe: Option<&'a str>,
    pub recipe_metadata: Option<&'a RecipeMetadata>,
    pub recipe_metadata_pending: bool,
    pub recipe_metadata_error: Option<&'a str>,
    /// The effective `BBMULTICONFIG` value. `None` means that no current
    /// workspace authority exists; `Some("")` is a known empty inventory.
    pub multiconfig: Option<&'a str>,
}

impl RawSelectorAuthority {
    pub fn project(sources: RawSelectorSources<'_>) -> Self {
        Self {
            recipes: project_recipe_choices(sources.recipes),
            images: project_image_choices(sources.images),
            targets: project_target_choices(sources.current_target, sources.recent_targets),
            tasks: project_task_choices(
                sources.selected_recipe,
                sources.recipe_metadata,
                sources.recipe_metadata_pending,
                sources.recipe_metadata_error,
            ),
            multiconfigs: project_multiconfig_choices(sources.multiconfig),
        }
    }

    fn inventory(&self, kind: RawParameterKind) -> Option<&RawSelectorInventory> {
        match kind {
            RawParameterKind::Recipe => Some(&self.recipes),
            RawParameterKind::Image => Some(&self.images),
            RawParameterKind::Target => Some(&self.targets),
            RawParameterKind::Task => Some(&self.tasks),
            RawParameterKind::Multiconfig => Some(&self.multiconfigs),
            RawParameterKind::UserInterface
            | RawParameterKind::File
            | RawParameterKind::Integer
            | RawParameterKind::Text => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawParameterSelector {
    pub parameter: RawParameter,
    pub inventory: RawSelectorInventory,
    pub manual_entry: bool,
}

impl RawParameterSelector {
    pub fn parse_manual(
        &self,
        input: &str,
    ) -> Result<Option<RawParameterValue>, RawParameterError> {
        self.parameter.parse_value(input)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawSelectorError {
    #[error("Raw command {command} has no parameter {parameter}")]
    UnknownParameter {
        command: RawCommandId,
        parameter: RawParameterId,
    },
    #[error("Raw command {0} is reference-only and has no executable selectors")]
    ReferenceOnly(RawCommandId),
    #[error("Raw parameter {parameter} on command {command} is not inventory-backed")]
    NotInventoryBacked {
        command: RawCommandId,
        parameter: RawParameterId,
    },
}

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
                self.editor.editing = !self.editor.editing;
                false
            }
            PopupEditorCommand::Insert(character) if self.editor.editing => {
                self.editor.insert(&character.to_string());
                true
            }
            PopupEditorCommand::Insert(_) => false,
            PopupEditorCommand::Backspace if self.editor.editing => {
                self.editor.backspace();
                true
            }
            PopupEditorCommand::Backspace => false,
            PopupEditorCommand::Left => {
                self.editor.left();
                false
            }
            PopupEditorCommand::Right => {
                self.editor.right();
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPreviewRequest {
    pub catalog_version: u16,
    pub command: RawCommandId,
    pub parameters: BTreeMap<RawParameterId, RawParameterValue>,
    pub additional_arguments: RawAdditionalArguments,
    pub capability_generation: u64,
    pub build_directory: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum RawPreviewArgumentSource {
    Executable,
    Template { index: usize },
    Additional { index: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawPreviewArgument {
    pub index: usize,
    pub value: String,
    pub source: RawPreviewArgumentSource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionPreview {
    pub catalog_version: u16,
    pub command: RawCommandId,
    pub executable: RawExecutable,
    pub arguments: Vec<String>,
    pub indexed_arguments: Vec<RawPreviewArgument>,
    pub capability_generation: u64,
    pub environment: crate::YoctoEnvironmentIdentity,
    pub build_directory: PathBuf,
    pub implementations: Vec<(CapabilityId, String)>,
    pub capability_issues: Vec<RawCapabilityIssue>,
    pub interaction: RawInteractionMode,
    pub safety: RawSafetyClass,
    pub limitations: Vec<String>,
}

macro_rules! raw_execution_id {
    ($name:ident, $prefix:literal, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, RawExecutionError> {
                let value = value.into();
                let token = value.strip_prefix($prefix).ok_or_else(|| {
                    RawExecutionError::InvalidIdentity {
                        kind: $kind,
                        value: value.clone(),
                    }
                })?;
                if value.len() > MAX_RAW_EXECUTION_ID_BYTES
                    || token.is_empty()
                    || !token.bytes().all(|byte| {
                        byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-')
                    })
                {
                    return Err(RawExecutionError::InvalidIdentity { kind: $kind, value });
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl TryFrom<&str> for $name {
            type Error = RawExecutionError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }
    };
}

raw_execution_id!(RawRequestId, "raw-request:", "request");
raw_execution_id!(RawJobId, "raw-job:", "job");
raw_execution_id!(RawSessionId, "raw-session:", "session");
raw_execution_id!(RawStreamId, "raw-stream:", "stream");
raw_execution_id!(RawDurableReferenceId, "raw-durable:", "durable reference");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawPreviewDigest(pub [u8; 32]);

impl RawPreviewDigest {
    pub fn from_preview(preview: &RawExecutionPreview) -> Self {
        let mut digest = Sha256::new();
        raw_digest_field(&mut digest, &preview.catalog_version.to_be_bytes());
        raw_digest_field(&mut digest, preview.command.as_str().as_bytes());
        raw_digest_field(&mut digest, preview.executable.as_str().as_bytes());
        raw_digest_field(&mut digest, &preview.capability_generation.to_be_bytes());
        raw_digest_field(
            &mut digest,
            preview.build_directory.as_os_str().as_encoded_bytes(),
        );
        raw_digest_field(
            &mut digest,
            raw_interaction_name(preview.interaction).as_bytes(),
        );
        raw_digest_field(&mut digest, raw_safety_name(preview.safety).as_bytes());
        for argument in &preview.indexed_arguments {
            raw_digest_field(&mut digest, &argument.index.to_be_bytes());
            raw_digest_field(&mut digest, argument.value.as_bytes());
            match argument.source {
                RawPreviewArgumentSource::Executable => {
                    raw_digest_field(&mut digest, b"executable")
                }
                RawPreviewArgumentSource::Template { index } => {
                    raw_digest_field(&mut digest, b"template");
                    raw_digest_field(&mut digest, &index.to_be_bytes());
                }
                RawPreviewArgumentSource::Additional { index } => {
                    raw_digest_field(&mut digest, b"additional");
                    raw_digest_field(&mut digest, &index.to_be_bytes());
                }
            }
        }
        Self(digest.finalize().into())
    }

    pub fn to_hex(self) -> String {
        self.0.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn from_hex(value: &str) -> Result<Self, RawExecutionError> {
        if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(RawExecutionError::InvalidPreviewDigest);
        }
        let mut bytes = [0; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
                .map_err(|_| RawExecutionError::InvalidPreviewDigest)?;
        }
        Ok(Self(bytes))
    }
}

fn raw_digest_field(digest: &mut Sha256, value: &[u8]) {
    digest.update((value.len() as u64).to_be_bytes());
    digest.update(value);
}

const fn raw_interaction_name(interaction: RawInteractionMode) -> &'static str {
    match interaction {
        RawInteractionMode::NoninteractiveJob => "noninteractive_job",
        RawInteractionMode::InteractivePty => "interactive_pty",
    }
}

const fn raw_safety_name(safety: RawSafetyClass) -> &'static str {
    match safety {
        RawSafetyClass::Inspection => "inspection",
        RawSafetyClass::Build => "build",
        RawSafetyClass::MetadataMutation => "metadata_mutation",
        RawSafetyClass::Destructive => "destructive",
        RawSafetyClass::ServerLifecycle => "server_lifecycle",
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawConfirmedExecutionRequest {
    pub id: RawRequestId,
    pub catalog_version: u16,
    pub command: RawCommandId,
    pub parameters: BTreeMap<RawParameterId, RawParameterValue>,
    pub additional_arguments: Vec<String>,
    pub interaction: RawInteractionMode,
    pub safety: RawSafetyClass,
    pub capability_generation: u64,
    pub build_directory: PathBuf,
    pub preview_digest: RawPreviewDigest,
}

impl RawConfirmedExecutionRequest {
    pub fn from_reviewed_preview(
        id: RawRequestId,
        catalog: &RawCatalog,
        request: &RawPreviewRequest,
        preview: &RawExecutionPreview,
    ) -> Result<Self, RawExecutionError> {
        if request.catalog_version != preview.catalog_version
            || request.command != preview.command
            || request.capability_generation != preview.capability_generation
            || request.build_directory != preview.build_directory
        {
            return Err(RawExecutionError::PreviewRequestMismatch);
        }
        catalog
            .validate()
            .map_err(|error| RawExecutionError::InvalidReviewedPreview(error.to_string()))?;
        if catalog.version != request.catalog_version {
            return Err(RawExecutionError::PreviewRequestMismatch);
        }
        let command = catalog
            .command(&request.command)
            .ok_or(RawExecutionError::InvalidCommand)?;
        let RawExecutionPolicy::Executable { template } = &command.execution else {
            return Err(RawExecutionError::InvalidCommand);
        };
        validate_raw_preview_parameters(command, &request.parameters)
            .map_err(|error| RawExecutionError::InvalidReviewedPreview(error.to_string()))?;
        if request.parameters.len() > MAX_RAW_PARAMETERS {
            return Err(RawExecutionError::TooManyParameters);
        }
        request.additional_arguments.validate()?;
        validate_raw_execution_build_directory(&request.build_directory)?;
        let mut arguments = Vec::new();
        let mut indexed_arguments = vec![RawPreviewArgument {
            index: 0,
            value: template.executable.as_str().into(),
            source: RawPreviewArgumentSource::Executable,
        }];
        for (template_index, argument) in template.arguments.iter().enumerate() {
            if let Some(value) = render_raw_template_argument(argument, &request.parameters) {
                push_raw_preview_argument(
                    &mut arguments,
                    &mut indexed_arguments,
                    value,
                    RawPreviewArgumentSource::Template {
                        index: template_index,
                    },
                )
                .map_err(|error| RawExecutionError::InvalidReviewedPreview(error.to_string()))?;
            }
        }
        for (additional_index, value) in request.additional_arguments.as_slice().iter().enumerate()
        {
            push_raw_preview_argument(
                &mut arguments,
                &mut indexed_arguments,
                value.clone(),
                RawPreviewArgumentSource::Additional {
                    index: additional_index,
                },
            )
            .map_err(|error| RawExecutionError::InvalidReviewedPreview(error.to_string()))?;
        }
        if preview.executable != template.executable
            || preview.interaction != template.interaction
            || preview.safety != template.safety
            || preview.arguments != arguments
            || preview.indexed_arguments != indexed_arguments
        {
            return Err(RawExecutionError::PreviewRequestMismatch);
        }
        Ok(Self {
            id,
            catalog_version: request.catalog_version,
            command: request.command.clone(),
            parameters: request.parameters.clone(),
            additional_arguments: request.additional_arguments.as_slice().to_vec(),
            interaction: preview.interaction,
            safety: preview.safety,
            capability_generation: request.capability_generation,
            build_directory: request.build_directory.clone(),
            preview_digest: RawPreviewDigest::from_preview(preview),
        })
    }

    pub fn validate(&self) -> Result<(), RawExecutionError> {
        RawRequestId::new(self.id.as_str())?;
        RawCommandId::new(self.command.as_str()).map_err(|_| RawExecutionError::InvalidCommand)?;
        if self.catalog_version == 0 || self.capability_generation == 0 {
            return Err(RawExecutionError::InvalidAuthority);
        }
        if self.parameters.len() > MAX_RAW_PARAMETERS {
            return Err(RawExecutionError::TooManyParameters);
        }
        for (parameter, value) in &self.parameters {
            RawParameterId::new(parameter.as_str())
                .map_err(|_| RawExecutionError::InvalidParameterValue)?;
            validate_raw_execution_parameter_value(value)?;
        }
        RawAdditionalArguments::from_vec(self.additional_arguments.clone())?;
        validate_raw_execution_build_directory(&self.build_directory)?;
        Ok(())
    }
}

fn validate_raw_execution_parameter_value(
    value: &RawParameterValue,
) -> Result<(), RawExecutionError> {
    let valid = match value {
        RawParameterValue::Recipe(value) => valid_raw_identifier(value, MAX_RAW_RECIPE_BYTES),
        RawParameterValue::Image(value) => valid_raw_identifier(value, MAX_RAW_IMAGE_BYTES),
        RawParameterValue::Target(value) => valid_raw_target(value),
        RawParameterValue::Task(value) => valid_raw_identifier(value, MAX_RAW_TASK_BYTES),
        RawParameterValue::UserInterface(value) => valid_raw_identifier(value, MAX_RAW_UI_BYTES),
        RawParameterValue::File(value) => valid_raw_file(value),
        RawParameterValue::Integer(_) => true,
        RawParameterValue::Text(value) => valid_raw_text_parameter(value),
        RawParameterValue::Multiconfig(value) => {
            valid_raw_identifier(value, MAX_RAW_MULTICONFIG_BYTES)
        }
    };
    if !valid {
        return Err(RawExecutionError::InvalidParameterValue);
    }
    Ok(())
}

fn validate_raw_execution_build_directory(path: &Path) -> Result<(), RawExecutionError> {
    if !path.is_absolute()
        || path.as_os_str().as_encoded_bytes().len() > MAX_RAW_FILE_BYTES
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(RawExecutionError::InvalidBuildDirectory);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawOutputChunk {
    pub stream_id: RawStreamId,
    pub stream: RawOutputStream,
    pub sequence: u64,
    pub text: String,
    pub truncated_bytes: u64,
    pub dropped_lines: u64,
}

impl RawOutputChunk {
    pub fn validate(&self) -> Result<(), RawExecutionError> {
        RawStreamId::new(self.stream_id.as_str())?;
        if self.sequence == 0 || self.text.len() > MAX_RAW_OUTPUT_CHUNK_BYTES {
            return Err(RawExecutionError::InvalidOutputChunk);
        }
        Ok(())
    }

    fn line_count(&self) -> usize {
        raw_output_line_count(&self.text)
    }
}

fn raw_output_line_count(text: &str) -> usize {
    if text.is_empty() {
        1
    } else {
        text.bytes().filter(|byte| *byte == b'\n').count() + usize::from(!text.ends_with('\n'))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawRetainedOutput {
    pub stream_id: RawStreamId,
    pub stream: RawOutputStream,
    pub chunks: VecDeque<RawOutputChunk>,
    pub next_sequence: u64,
    pub retained_bytes: usize,
    pub retained_lines: usize,
    pub dropped_bytes: u64,
    pub dropped_lines: u64,
    pub truncated_chunks: u64,
}

impl RawRetainedOutput {
    pub fn new(stream_id: RawStreamId, stream: RawOutputStream) -> Self {
        Self {
            stream_id,
            stream,
            chunks: VecDeque::new(),
            next_sequence: 1,
            retained_bytes: 0,
            retained_lines: 0,
            dropped_bytes: 0,
            dropped_lines: 0,
            truncated_chunks: 0,
        }
    }

    fn append(&mut self, chunk: RawOutputChunk) -> Result<bool, RawExecutionError> {
        chunk.validate()?;
        if chunk.stream_id != self.stream_id || chunk.stream != self.stream {
            return Err(RawExecutionError::WrongOutputStream);
        }
        if chunk.sequence < self.next_sequence {
            return Ok(false);
        }
        if chunk.sequence != self.next_sequence {
            return Err(RawExecutionError::OutputGap {
                expected: self.next_sequence,
                actual: chunk.sequence,
            });
        }
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(RawExecutionError::SequenceExhausted)?;
        self.retained_bytes = self.retained_bytes.saturating_add(chunk.text.len());
        self.retained_lines = self.retained_lines.saturating_add(chunk.line_count());
        self.dropped_bytes = self.dropped_bytes.saturating_add(chunk.truncated_bytes);
        self.dropped_lines = self.dropped_lines.saturating_add(chunk.dropped_lines);
        self.truncated_chunks = self
            .truncated_chunks
            .saturating_add(u64::from(chunk.truncated_bytes > 0));
        self.chunks.push_back(chunk);
        while self.retained_bytes > MAX_RAW_OUTPUT_RETAINED_BYTES
            || self.retained_lines > MAX_RAW_OUTPUT_RETAINED_LINES
            || self.chunks.len() > MAX_RAW_OUTPUT_RETAINED_LINES
        {
            let Some(removed) = self.chunks.pop_front() else {
                break;
            };
            let removed_bytes = removed.text.len();
            let removed_lines = removed.line_count();
            self.retained_bytes = self.retained_bytes.saturating_sub(removed_bytes);
            self.retained_lines = self.retained_lines.saturating_sub(removed_lines);
            self.dropped_bytes = self.dropped_bytes.saturating_add(removed_bytes as u64);
            self.dropped_lines = self.dropped_lines.saturating_add(removed_lines as u64);
        }
        Ok(true)
    }

    pub fn validate(&self) -> Result<(), RawExecutionError> {
        RawStreamId::new(self.stream_id.as_str())?;
        if self.next_sequence == 0
            || self.retained_bytes > MAX_RAW_OUTPUT_RETAINED_BYTES
            || self.retained_lines > MAX_RAW_OUTPUT_RETAINED_LINES
            || self.chunks.len() > MAX_RAW_OUTPUT_RETAINED_LINES
            || self.retained_bytes != self.chunks.iter().map(|chunk| chunk.text.len()).sum()
            || self.retained_lines != self.chunks.iter().map(RawOutputChunk::line_count).sum()
        {
            return Err(RawExecutionError::InvalidOutputSnapshot);
        }
        let mut expected = self
            .chunks
            .front()
            .map(|chunk| chunk.sequence)
            .unwrap_or(self.next_sequence);
        for chunk in &self.chunks {
            chunk.validate()?;
            if chunk.stream_id != self.stream_id
                || chunk.stream != self.stream
                || chunk.sequence != expected
            {
                return Err(RawExecutionError::InvalidOutputSnapshot);
            }
            expected = expected
                .checked_add(1)
                .ok_or(RawExecutionError::SequenceExhausted)?;
        }
        if expected != self.next_sequence {
            return Err(RawExecutionError::InvalidOutputSnapshot);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum RawExecutionOwner {
    Job(RawJobId),
    Pty(RawSessionId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Lost,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", content = "outcome", rename_all = "snake_case")]
pub enum RawExecutionPhase {
    Queued,
    Starting,
    Running,
    Cancelling,
    Terminal(RawExecutionOutcome),
}

impl RawExecutionPhase {
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal(_))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawAttachmentState {
    Attached,
    Detached,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct RawEventCursor {
    pub sequence: u64,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionResult {
    pub outcome: RawExecutionOutcome,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub elapsed_ms: u64,
    pub durable_reference: Option<RawDurableReferenceId>,
}

impl RawExecutionResult {
    pub fn validate(&self) -> Result<(), RawExecutionError> {
        if self
            .message
            .as_ref()
            .is_some_and(|message| message.len() > MAX_RAW_EXECUTION_MESSAGE_BYTES)
        {
            return Err(RawExecutionError::ResultMessageTooLong);
        }
        if let Some(reference) = &self.durable_reference {
            RawDurableReferenceId::new(reference.as_str())?;
        }
        match (self.outcome, self.exit_code) {
            (RawExecutionOutcome::Succeeded, Some(0))
            | (RawExecutionOutcome::Failed, Some(_))
            | (RawExecutionOutcome::Failed, None)
            | (RawExecutionOutcome::Cancelled, Some(_))
            | (RawExecutionOutcome::Cancelled, None)
            | (RawExecutionOutcome::Lost, None) => Ok(()),
            (RawExecutionOutcome::Succeeded, Some(_))
            | (RawExecutionOutcome::Succeeded, None)
            | (RawExecutionOutcome::Lost, Some(_)) => Err(RawExecutionError::InvalidResult),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionState {
    pub request: RawConfirmedExecutionRequest,
    pub phase: RawExecutionPhase,
    pub attachment: RawAttachmentState,
    pub owner: Option<RawExecutionOwner>,
    pub cancellation_requested: bool,
    pub queued_unix_ms: u64,
    pub started_unix_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub result: Option<RawExecutionResult>,
    pub stdout: RawRetainedOutput,
    pub stderr: RawRetainedOutput,
    pub cursor: RawEventCursor,
}

impl RawExecutionState {
    pub fn queued(
        request: RawConfirmedExecutionRequest,
        stdout: RawStreamId,
        stderr: RawStreamId,
        queued_unix_ms: u64,
        cursor: RawEventCursor,
    ) -> Result<Self, RawExecutionError> {
        request.validate()?;
        if stdout == stderr {
            return Err(RawExecutionError::DuplicateStreamIdentity);
        }
        let state = Self {
            request,
            phase: RawExecutionPhase::Queued,
            attachment: RawAttachmentState::Attached,
            owner: None,
            cancellation_requested: false,
            queued_unix_ms,
            started_unix_ms: None,
            elapsed_ms: 0,
            result: None,
            stdout: RawRetainedOutput::new(stdout, RawOutputStream::Stdout),
            stderr: RawRetainedOutput::new(stderr, RawOutputStream::Stderr),
            cursor,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn validate(&self) -> Result<(), RawExecutionError> {
        self.request.validate()?;
        self.stdout.validate()?;
        self.stderr.validate()?;
        if self.stdout.stream_id == self.stderr.stream_id
            || self.stdout.stream != RawOutputStream::Stdout
            || self.stderr.stream != RawOutputStream::Stderr
        {
            return Err(RawExecutionError::DuplicateStreamIdentity);
        }
        if let Some(owner) = &self.owner {
            match owner {
                RawExecutionOwner::Job(id) => {
                    RawJobId::new(id.as_str())?;
                    if self.request.interaction != RawInteractionMode::NoninteractiveJob {
                        return Err(RawExecutionError::WrongOwnerKind);
                    }
                }
                RawExecutionOwner::Pty(id) => {
                    RawSessionId::new(id.as_str())?;
                    if self.request.interaction != RawInteractionMode::InteractivePty {
                        return Err(RawExecutionError::WrongOwnerKind);
                    }
                }
            }
        }
        match (self.phase, &self.result) {
            (RawExecutionPhase::Terminal(outcome), Some(result)) if outcome == result.outcome => {
                result.validate()?;
                if self.elapsed_ms != result.elapsed_ms {
                    return Err(RawExecutionError::InvalidResult);
                }
            }
            (RawExecutionPhase::Terminal(_), _) | (_, Some(_)) => {
                return Err(RawExecutionError::InvalidResult);
            }
            _ => {}
        }
        match self.phase {
            RawExecutionPhase::Queued if self.owner.is_some() || self.started_unix_ms.is_some() => {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Starting
                if self.owner.is_none() || self.started_unix_ms.is_some() =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Running
                if self.owner.is_none() || self.started_unix_ms.is_none() =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Cancelling
                if !self.cancellation_requested
                    || self.started_unix_ms.is_some() && self.owner.is_none() =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
                if !self.cancellation_requested =>
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            _ => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutionEvent {
    pub request_id: RawRequestId,
    pub sequence: u64,
    pub generation: u64,
    pub kind: RawExecutionEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RawExecutionEventKind {
    Starting { owner: RawExecutionOwner },
    Running { started_unix_ms: u64 },
    CancellationRequested,
    Cancelling,
    AttachmentChanged { attachment: RawAttachmentState },
    Elapsed { elapsed_ms: u64 },
    Output { chunk: RawOutputChunk },
    Finished { result: RawExecutionResult },
}

pub fn reduce_raw_execution(
    state: &mut RawExecutionState,
    event: RawExecutionEvent,
) -> Result<bool, RawExecutionError> {
    if event.request_id != state.request.id {
        return Err(RawExecutionError::WrongRequest);
    }
    if event.sequence <= state.cursor.sequence || event.generation <= state.cursor.generation {
        return Ok(false);
    }
    let expected_sequence = state
        .cursor
        .sequence
        .checked_add(1)
        .ok_or(RawExecutionError::SequenceExhausted)?;
    let expected_generation = state
        .cursor
        .generation
        .checked_add(1)
        .ok_or(RawExecutionError::GenerationExhausted)?;
    if event.sequence != expected_sequence || event.generation != expected_generation {
        return Err(RawExecutionError::EventGap {
            expected_sequence,
            actual_sequence: event.sequence,
            expected_generation,
            actual_generation: event.generation,
        });
    }
    let mut next = state.clone();
    match event.kind {
        RawExecutionEventKind::Starting { owner } => {
            if next.phase != RawExecutionPhase::Queued {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.owner = Some(owner);
            next.phase = RawExecutionPhase::Starting;
        }
        RawExecutionEventKind::Running { started_unix_ms } => {
            if next.phase != RawExecutionPhase::Starting || next.owner.is_none() {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.started_unix_ms = Some(started_unix_ms);
            next.phase = RawExecutionPhase::Running;
        }
        RawExecutionEventKind::CancellationRequested => {
            if next.phase.is_terminal() {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.cancellation_requested = true;
        }
        RawExecutionEventKind::Cancelling => {
            if !next.cancellation_requested
                || !matches!(
                    next.phase,
                    RawExecutionPhase::Queued
                        | RawExecutionPhase::Starting
                        | RawExecutionPhase::Running
                )
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.phase = RawExecutionPhase::Cancelling;
        }
        RawExecutionEventKind::AttachmentChanged { attachment } => {
            next.attachment = attachment;
        }
        RawExecutionEventKind::Elapsed { elapsed_ms } => {
            if next.phase.is_terminal() || elapsed_ms < next.elapsed_ms {
                return Err(RawExecutionError::InvalidElapsed);
            }
            next.elapsed_ms = elapsed_ms;
        }
        RawExecutionEventKind::Output { chunk } => {
            if !matches!(
                next.phase,
                RawExecutionPhase::Starting
                    | RawExecutionPhase::Running
                    | RawExecutionPhase::Cancelling
            ) {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            match chunk.stream {
                RawOutputStream::Stdout => {
                    if !next.stdout.append(chunk)? {
                        return Ok(false);
                    }
                }
                RawOutputStream::Stderr => {
                    if !next.stderr.append(chunk)? {
                        return Ok(false);
                    }
                }
            }
        }
        RawExecutionEventKind::Finished { result } => {
            result.validate()?;
            if next.phase.is_terminal()
                || (result.outcome == RawExecutionOutcome::Cancelled
                    && !next.cancellation_requested)
                || (result.outcome == RawExecutionOutcome::Succeeded
                    && next.phase != RawExecutionPhase::Running)
            {
                return Err(RawExecutionError::InvalidLifecycle);
            }
            next.elapsed_ms = result.elapsed_ms;
            next.phase = RawExecutionPhase::Terminal(result.outcome);
            next.result = Some(result);
        }
    }
    next.cursor = RawEventCursor {
        sequence: event.sequence,
        generation: event.generation,
    };
    next.validate()?;
    *state = next;
    Ok(true)
}

pub fn replace_raw_execution_snapshot(
    state: &mut Option<RawExecutionState>,
    replacement: RawExecutionState,
) -> Result<bool, RawExecutionError> {
    replacement.validate()?;
    if let Some(current) = state
        && current.request.id == replacement.request.id
        && (replacement.cursor.sequence <= current.cursor.sequence
            || replacement.cursor.generation <= current.cursor.generation)
    {
        return Ok(false);
    }
    *state = Some(replacement);
    Ok(true)
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawExecutionError {
    #[error("invalid Raw {kind} identity: {value:?}")]
    InvalidIdentity { kind: &'static str, value: String },
    #[error("invalid Raw command identity")]
    InvalidCommand,
    #[error("Raw execution authority must use nonzero catalog and capability generations")]
    InvalidAuthority,
    #[error("Raw preview and preview request do not describe the same reviewed work")]
    PreviewRequestMismatch,
    #[error("Raw reviewed preview is invalid: {0}")]
    InvalidReviewedPreview(String),
    #[error("Raw preview digest is not a 32-byte hexadecimal SHA-256 digest")]
    InvalidPreviewDigest,
    #[error("Raw execution contains too many typed parameters")]
    TooManyParameters,
    #[error("Raw execution contains an invalid typed parameter value")]
    InvalidParameterValue,
    #[error("Raw execution build directory is not a bounded absolute normalized identity")]
    InvalidBuildDirectory,
    #[error(transparent)]
    InvalidAdditionalArguments(#[from] RawArgvError),
    #[error("Raw stdout and stderr must use distinct stream identities")]
    DuplicateStreamIdentity,
    #[error("Raw execution owner does not match its interaction class")]
    WrongOwnerKind,
    #[error("Raw output chunk is empty-sequence or exceeds its byte bound")]
    InvalidOutputChunk,
    #[error("Raw output chunk names the wrong typed stream")]
    WrongOutputStream,
    #[error("Raw output sequence gap: expected {expected}, got {actual}")]
    OutputGap { expected: u64, actual: u64 },
    #[error("Raw output snapshot is inconsistent or exceeds retained bounds")]
    InvalidOutputSnapshot,
    #[error("Raw execution event belongs to a different request")]
    WrongRequest,
    #[error(
        "Raw execution event gap: expected sequence/generation {expected_sequence}/{expected_generation}, got {actual_sequence}/{actual_generation}"
    )]
    EventGap {
        expected_sequence: u64,
        actual_sequence: u64,
        expected_generation: u64,
        actual_generation: u64,
    },
    #[error("Raw execution event sequence is exhausted")]
    SequenceExhausted,
    #[error("Raw execution generation is exhausted")]
    GenerationExhausted,
    #[error("invalid Raw execution lifecycle transition")]
    InvalidLifecycle,
    #[error("Raw execution elapsed time moved backwards or changed after termination")]
    InvalidElapsed,
    #[error("Raw execution result is inconsistent with its outcome")]
    InvalidResult,
    #[error("Raw execution result message exceeds its byte bound")]
    ResultMessageTooLong,
}

#[cfg(test)]
mod raw_execution_tests {
    use super::*;

    fn request_and_preview(
        interaction: RawInteractionMode,
    ) -> (RawCatalog, RawPreviewRequest, RawExecutionPreview) {
        let mut catalog = super::raw_preview_tests::catalog();
        let RawExecutionPolicy::Executable { template } =
            &mut catalog.commands.first_mut().unwrap().execution
        else {
            unreachable!();
        };
        template.interaction = interaction;
        let mut request = super::raw_preview_tests::request();
        request.additional_arguments =
            RawAdditionalArguments::from_vec(vec!["--dry-run".into()]).unwrap();
        let preview = catalog
            .preview(
                &request,
                Some(&super::raw_preview_tests::authority(
                    request.capability_generation,
                    true,
                )),
            )
            .unwrap();
        (catalog, request, preview)
    }

    fn confirmed(interaction: RawInteractionMode) -> RawConfirmedExecutionRequest {
        let (catalog, request, preview) = request_and_preview(interaction);
        RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new("raw-request:test-1").unwrap(),
            &catalog,
            &request,
            &preview,
        )
        .unwrap()
    }

    fn queued(interaction: RawInteractionMode) -> RawExecutionState {
        RawExecutionState::queued(
            confirmed(interaction),
            RawStreamId::new("raw-stream:stdout-1").unwrap(),
            RawStreamId::new("raw-stream:stderr-1").unwrap(),
            100,
            RawEventCursor::default(),
        )
        .unwrap()
    }

    fn apply(state: &mut RawExecutionState, kind: RawExecutionEventKind) {
        reduce_raw_execution(
            state,
            RawExecutionEvent {
                request_id: state.request.id.clone(),
                sequence: state.cursor.sequence + 1,
                generation: state.cursor.generation + 1,
                kind,
            },
        )
        .unwrap();
    }

    #[test]
    fn raw_execution_identities_are_bounded_disjoint_and_digest_is_deterministic() {
        assert!(RawRequestId::new("raw-request:one").is_ok());
        assert!(RawRequestId::new("raw-job:one").is_err());
        assert!(RawJobId::new("raw-request:one").is_err());
        assert!(RawSessionId::new("raw-session:").is_err());
        assert!(RawStreamId::new("raw-stream:bad/token").is_err());
        assert!(
            RawDurableReferenceId::new(format!(
                "raw-durable:{}",
                "x".repeat(MAX_RAW_EXECUTION_ID_BYTES)
            ))
            .is_err()
        );

        let (catalog, request, preview) =
            request_and_preview(RawInteractionMode::NoninteractiveJob);
        let first = RawConfirmedExecutionRequest::from_reviewed_preview(
            RawRequestId::new("raw-request:one").unwrap(),
            &catalog,
            &request,
            &preview,
        )
        .unwrap();
        let second = RawPreviewDigest::from_preview(&preview);
        assert_eq!(first.preview_digest, second);
        assert_eq!(
            RawPreviewDigest::from_hex(&second.to_hex()).unwrap(),
            second
        );
        assert_eq!(second.to_hex().len(), 64);
        assert!(!second.to_hex().contains("bitbake"));

        let mut forged = preview.clone();
        forged.arguments[0] = "forged".into();
        assert_eq!(
            RawConfirmedExecutionRequest::from_reviewed_preview(
                RawRequestId::new("raw-request:forged").unwrap(),
                &catalog,
                &request,
                &forged,
            ),
            Err(RawExecutionError::PreviewRequestMismatch)
        );

        let mut mismatched = preview;
        mismatched.capability_generation += 1;
        assert_eq!(
            RawConfirmedExecutionRequest::from_reviewed_preview(
                RawRequestId::new("raw-request:two").unwrap(),
                &catalog,
                &request,
                &mismatched,
            ),
            Err(RawExecutionError::PreviewRequestMismatch)
        );
    }

    #[test]
    fn raw_execution_reducer_covers_job_lifecycle_output_detach_and_success() {
        let mut state = queued(RawInteractionMode::NoninteractiveJob);
        apply(
            &mut state,
            RawExecutionEventKind::Starting {
                owner: RawExecutionOwner::Job(RawJobId::new("raw-job:one").unwrap()),
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::Running {
                started_unix_ms: 125,
            },
        );
        let stdout = state.stdout.stream_id.clone();
        apply(
            &mut state,
            RawExecutionEventKind::Output {
                chunk: RawOutputChunk {
                    stream_id: stdout,
                    stream: RawOutputStream::Stdout,
                    sequence: 1,
                    text: "héllo\n".into(),
                    truncated_bytes: 3,
                    dropped_lines: 1,
                },
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::AttachmentChanged {
                attachment: RawAttachmentState::Detached,
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::AttachmentChanged {
                attachment: RawAttachmentState::Attached,
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::AttachmentChanged {
                attachment: RawAttachmentState::Detached,
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::Elapsed { elapsed_ms: 80 },
        );
        apply(
            &mut state,
            RawExecutionEventKind::Finished {
                result: RawExecutionResult {
                    outcome: RawExecutionOutcome::Succeeded,
                    exit_code: Some(0),
                    message: Some("complete".into()),
                    elapsed_ms: 90,
                    durable_reference: Some(
                        RawDurableReferenceId::new("raw-durable:history-1").unwrap(),
                    ),
                },
            },
        );
        assert_eq!(
            state.phase,
            RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded)
        );
        assert_eq!(state.attachment, RawAttachmentState::Detached);
        assert_eq!(state.stdout.retained_bytes, "héllo\n".len());
        assert_eq!(state.stdout.dropped_bytes, 3);
        assert_eq!(state.stdout.dropped_lines, 1);
        assert_eq!(state.elapsed_ms, 90);
        assert!(state.validate().is_ok());
    }

    #[test]
    fn raw_pty_execution_cancellation_and_fail_closed_correlation_are_exact() {
        let mut state = queued(RawInteractionMode::InteractivePty);
        apply(
            &mut state,
            RawExecutionEventKind::Starting {
                owner: RawExecutionOwner::Pty(
                    RawSessionId::new("raw-session:interactive-1").unwrap(),
                ),
            },
        );
        apply(
            &mut state,
            RawExecutionEventKind::Running {
                started_unix_ms: 101,
            },
        );
        apply(&mut state, RawExecutionEventKind::CancellationRequested);
        apply(&mut state, RawExecutionEventKind::Cancelling);
        let before = state.clone();
        let stale = RawExecutionEvent {
            request_id: state.request.id.clone(),
            sequence: state.cursor.sequence,
            generation: state.cursor.generation,
            kind: RawExecutionEventKind::Elapsed { elapsed_ms: 999 },
        };
        assert!(!reduce_raw_execution(&mut state, stale).unwrap());
        assert_eq!(state, before);

        let gap = RawExecutionEvent {
            request_id: state.request.id.clone(),
            sequence: state.cursor.sequence + 2,
            generation: state.cursor.generation + 2,
            kind: RawExecutionEventKind::Elapsed { elapsed_ms: 999 },
        };
        assert!(matches!(
            reduce_raw_execution(&mut state, gap),
            Err(RawExecutionError::EventGap { .. })
        ));
        assert_eq!(state, before);

        apply(
            &mut state,
            RawExecutionEventKind::Finished {
                result: RawExecutionResult {
                    outcome: RawExecutionOutcome::Cancelled,
                    exit_code: None,
                    message: None,
                    elapsed_ms: 50,
                    durable_reference: None,
                },
            },
        );
        assert_eq!(
            state.phase,
            RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
        );
    }

    #[test]
    fn raw_execution_streams_are_independently_bounded_and_snapshots_reject_corruption() {
        let mut output = RawRetainedOutput::new(
            RawStreamId::new("raw-stream:bounded").unwrap(),
            RawOutputStream::Stdout,
        );
        for sequence in 1..=20 {
            output
                .append(RawOutputChunk {
                    stream_id: output.stream_id.clone(),
                    stream: RawOutputStream::Stdout,
                    sequence,
                    text: "界".repeat(MAX_RAW_OUTPUT_CHUNK_BYTES / 3),
                    truncated_bytes: 0,
                    dropped_lines: 0,
                })
                .unwrap();
        }
        assert!(output.retained_bytes <= MAX_RAW_OUTPUT_RETAINED_BYTES);
        assert!(output.retained_lines <= MAX_RAW_OUTPUT_RETAINED_LINES);
        assert!(output.dropped_bytes > 0);
        assert!(output.validate().is_ok());

        let mut corrupt = queued(RawInteractionMode::NoninteractiveJob);
        corrupt.stdout.retained_bytes = 1;
        let mut installed = Some(queued(RawInteractionMode::NoninteractiveJob));
        let before = installed.clone();
        assert_eq!(
            replace_raw_execution_snapshot(&mut installed, corrupt),
            Err(RawExecutionError::InvalidOutputSnapshot)
        );
        assert_eq!(installed, before);

        let oversized = RawOutputChunk {
            stream_id: RawStreamId::new("raw-stream:oversized").unwrap(),
            stream: RawOutputStream::Stderr,
            sequence: 1,
            text: "é".repeat(MAX_RAW_OUTPUT_CHUNK_BYTES / 2 + 1),
            truncated_bytes: 0,
            dropped_lines: 0,
        };
        assert_eq!(
            oversized.validate(),
            Err(RawExecutionError::InvalidOutputChunk)
        );
    }

    #[test]
    fn raw_execution_terminal_failure_and_loss_paths_validate() {
        for (outcome, exit_code) in [
            (RawExecutionOutcome::Failed, Some(2)),
            (RawExecutionOutcome::Lost, None),
        ] {
            let mut state = queued(RawInteractionMode::NoninteractiveJob);
            apply(
                &mut state,
                RawExecutionEventKind::Finished {
                    result: RawExecutionResult {
                        outcome,
                        exit_code,
                        message: Some("terminal".into()),
                        elapsed_ms: 1,
                        durable_reference: None,
                    },
                },
            );
            assert_eq!(state.phase, RawExecutionPhase::Terminal(outcome));
        }
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawPreviewError {
    #[error("Raw preview catalog is invalid: {0}")]
    InvalidCatalog(String),
    #[error("Raw preview catalog version {received} does not match current version {current}")]
    StaleCatalog { current: u16, received: u16 },
    #[error("Raw preview command is unknown: {0}")]
    UnknownCommand(RawCommandId),
    #[error("Raw preview command is reference-only: {0}")]
    ReferenceOnly(RawCommandId),
    #[error("Raw preview has no current daemon capability authority")]
    MissingAuthority,
    #[error(
        "Raw preview capability generation {received} is stale; current generation is {current}"
    )]
    StaleCapabilityGeneration { current: u64, received: u64 },
    #[error("Raw preview capability is {state:?}: {reasons:?}")]
    CapabilityUnavailable {
        state: RawAvailabilityState,
        reasons: Vec<String>,
    },
    #[error("Raw preview has no authoritative build-directory identity")]
    MissingBuildDirectory,
    #[error("Raw preview build-directory identity is invalid: {0:?}")]
    InvalidBuildDirectory(PathBuf),
    #[error("Raw preview build directory {received:?} does not match {current:?}")]
    StaleBuildDirectory { current: PathBuf, received: PathBuf },
    #[error("Raw preview contains an unknown parameter: {0}")]
    UnknownParameter(RawParameterId),
    #[error("Raw preview is missing required parameter: {0}")]
    MissingParameter(RawParameterId),
    #[error(transparent)]
    InvalidParameter(#[from] RawParameterError),
    #[error("Raw preview has invalid additional arguments: {0}")]
    InvalidAdditionalArguments(RawArgvError),
    #[error("Raw preview contains too many indexed argv elements: {count} > {maximum}")]
    TooManyArguments { count: usize, maximum: usize },
    #[error("Raw preview argv element {argument} is {bytes} bytes; maximum is {maximum}")]
    ArgumentTooLong {
        argument: usize,
        bytes: usize,
        maximum: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawBrowserColumn {
    Categories,
    Commands,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawModeView {
    Browser,
    Form,
    Preview,
    Execution,
    History,
    Favorites,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RawModeFocus {
    Categories,
    Commands,
    Search,
    Form,
    Preview,
    Execution,
    History,
    Favorites,
    FavoriteConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RawSearchState {
    pub query: String,
    pub editing: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFormField {
    pub parameter: RawParameterId,
    pub editor: PopupEditor,
    pub value: Option<RawParameterValue>,
    pub validation_error: Option<RawParameterError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawCommandForm {
    pub command: RawCommandId,
    pub fields: BTreeMap<RawParameterId, RawFormField>,
    pub field_order: Vec<RawParameterId>,
    pub field_selection: usize,
    pub additional_arguments: RawArgvEditor,
    pub capability_generation: u64,
    pub build_directory: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFavoriteConfirmation {
    pub command: RawCommandId,
    pub return_focus: RawModeFocus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawModeState {
    pub catalog_version: u16,
    pub category: Option<RawCategoryId>,
    pub command: Option<RawCommandId>,
    pub browser_column: RawBrowserColumn,
    pub view: RawModeView,
    pub focus: RawModeFocus,
    pub search: RawSearchState,
    pub form: Option<RawCommandForm>,
    pub preview: Option<RawExecutionPreview>,
    pub execution: Option<RawCommandId>,
    pub execution_states: BTreeMap<RawRequestId, RawExecutionState>,
    pub history: Vec<RawCommandId>,
    pub history_selection: usize,
    pub favorites: Vec<RawCommandId>,
    pub favorite_selection: usize,
    pub favorite_confirmation: Option<RawFavoriteConfirmation>,
    pub notification: Option<String>,
    return_stack: Vec<(RawModeView, RawModeFocus)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RawModeAction {
    SelectCategory {
        delta: isize,
    },
    SelectCommand {
        delta: isize,
    },
    FocusCategories,
    FocusCommands,
    OpenSelected,
    Back,
    BeginSearch,
    AppendSearch(char),
    BackspaceSearch,
    FinishSearch,
    ClearSearch,
    SetParameterInput {
        parameter: RawParameterId,
        input: String,
    },
    ChooseParameter {
        parameter: RawParameterId,
        value: RawParameterValue,
    },
    EditParameterInput {
        parameter: RawParameterId,
        command: PopupEditorCommand,
    },
    SelectFormField {
        delta: isize,
    },
    EditAdditionalArguments(PopupEditorCommand),
    RequestPreview,
    ConfirmPreview,
    CancelExecution(RawRequestId),
    OpenExecution(RawCommandId),
    OpenHistory,
    SelectHistory {
        delta: isize,
    },
    ActivateHistory,
    RememberHistory(RawCommandId),
    OpenFavorites,
    SelectFavorite {
        delta: isize,
    },
    ActivateFavorite,
    ToggleFavorite,
    ConfirmFavorite,
    CancelFavorite,
    ReprojectCatalog,
    ReprojectAuthority,
    DismissNotification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawInteractionMode {
    NoninteractiveJob,
    InteractivePty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawSafetyClass {
    Inspection,
    Build,
    MetadataMutation,
    Destructive,
    ServerLifecycle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawExecutable {
    BitBake,
}

impl RawExecutable {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BitBake => "bitbake",
        }
    }
}

/// One argv token. Parameters are substituted as data, never as shell text.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RawArgument {
    Literal {
        value: String,
    },
    Empty,
    Parameter {
        parameter: RawParameterId,
    },
    JoinedParameter {
        prefix: String,
        parameter: RawParameterId,
    },
    Composed {
        segments: Vec<RawArgumentSegment>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RawArgumentSegment {
    Literal { value: String },
    Parameter { parameter: RawParameterId },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "requirement", rename_all = "snake_case")]
pub enum RawCapabilityRequirement {
    All { capabilities: Vec<CapabilityId> },
    Any { capabilities: Vec<CapabilityId> },
}

impl RawCapabilityRequirement {
    fn capabilities(&self) -> &[CapabilityId] {
        match self {
            Self::All { capabilities } | Self::Any { capabilities } => capabilities,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawReferenceKind {
    ShellPipeline,
    Conceptual,
    CompanionTool,
    UnsupportedBitBake,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawReference {
    pub id: RawReferenceId,
    pub heading: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawExecutableTemplate {
    pub executable: RawExecutable,
    pub arguments: Vec<RawArgument>,
    pub capabilities: RawCapabilityRequirement,
    pub interaction: RawInteractionMode,
    pub safety: RawSafetyClass,
}

impl RawExecutableTemplate {
    pub fn display_template(&self, parameters: &[RawParameter]) -> Option<String> {
        let mut tokens = Vec::with_capacity(self.arguments.len() + 1);
        tokens.push(self.executable.as_str().to_owned());
        for argument in &self.arguments {
            let token = match argument {
                RawArgument::Literal { value } => value.clone(),
                RawArgument::Empty => "''".into(),
                RawArgument::Parameter { parameter } => {
                    parameter_placeholder(parameters, parameter)?.into()
                }
                RawArgument::JoinedParameter { prefix, parameter } => {
                    format!("{prefix}{}", parameter_placeholder(parameters, parameter)?)
                }
                RawArgument::Composed { segments } => {
                    let mut token = String::new();
                    for segment in segments {
                        match segment {
                            RawArgumentSegment::Literal { value } => token.push_str(value),
                            RawArgumentSegment::Parameter { parameter } => {
                                token.push_str(parameter_placeholder(parameters, parameter)?)
                            }
                        }
                    }
                    token
                }
            };
            tokens.push(token);
        }
        Some(tokens.join(" "))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "policy", rename_all = "snake_case")]
pub enum RawExecutionPolicy {
    Executable {
        template: RawExecutableTemplate,
    },
    ReferenceOnly {
        kind: RawReferenceKind,
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCommand {
    pub id: RawCommandId,
    pub category: RawCategoryId,
    pub label: String,
    pub description: String,
    pub reference: RawReference,
    pub parameters: Vec<RawParameter>,
    pub execution: RawExecutionPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawAvailabilityState {
    Available,
    Limited,
    Unavailable,
    Unknown,
    Unsupported,
}

impl RawAvailabilityState {
    pub const fn is_enabled(self) -> bool {
        matches!(self, Self::Available | Self::Limited)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCapabilityIssue {
    pub capability: Option<CapabilityId>,
    pub reason: String,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCommandAvailability {
    pub state: RawAvailabilityState,
    pub issues: Vec<RawCapabilityIssue>,
    pub implementations: Vec<(CapabilityId, String)>,
}

impl RawCommandAvailability {
    pub const fn is_enabled(&self) -> bool {
        self.state.is_enabled()
    }
}

impl RawCommand {
    /// Project this command only from daemon-owned connected-environment
    /// authority. The bundled reference version never participates.
    pub fn availability(
        &self,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> RawCommandAvailability {
        match &self.execution {
            RawExecutionPolicy::ReferenceOnly { reason, .. } => RawCommandAvailability {
                state: RawAvailabilityState::Unsupported,
                issues: vec![RawCapabilityIssue {
                    capability: None,
                    reason: reason.clone(),
                    limitations: Vec::new(),
                }],
                implementations: Vec::new(),
            },
            RawExecutionPolicy::Executable { template } => match &template.capabilities {
                RawCapabilityRequirement::All { capabilities } => {
                    project_raw_capabilities(authority, capabilities, RawRequirementOperator::All)
                }
                RawCapabilityRequirement::Any { capabilities } => {
                    project_raw_capabilities(authority, capabilities, RawRequirementOperator::Any)
                }
            },
        }
    }

    pub fn selector(
        &self,
        parameter: &RawParameterId,
        authority: &RawSelectorAuthority,
    ) -> Result<RawParameterSelector, RawSelectorError> {
        let definition = self
            .parameters
            .iter()
            .find(|definition| &definition.id == parameter)
            .ok_or_else(|| RawSelectorError::UnknownParameter {
                command: self.id.clone(),
                parameter: parameter.clone(),
            })?;
        let RawExecutionPolicy::Executable { template } = &self.execution else {
            return Err(RawSelectorError::ReferenceOnly(self.id.clone()));
        };
        let inventory = authority.inventory(definition.kind).ok_or_else(|| {
            RawSelectorError::NotInventoryBacked {
                command: self.id.clone(),
                parameter: parameter.clone(),
            }
        })?;

        // Manual entry is exposed only by executable definitions that actually
        // substitute this parameter. Catalog validation normally guarantees
        // this; keeping the check here also fails closed for constructed data.
        let manual_entry = template_references_parameter(template, parameter);
        Ok(RawParameterSelector {
            parameter: definition.clone(),
            inventory: inventory.clone(),
            manual_entry,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RawCatalog {
    pub version: u16,
    pub categories: Vec<RawCategory>,
    pub commands: Vec<RawCommand>,
}

impl RawCatalog {
    pub fn normalize(self) -> Result<Self, RawCatalogError> {
        self.validate()?;
        Ok(self)
    }

    pub fn validate(&self) -> Result<(), RawCatalogError> {
        if self.version == 0 {
            return Err(RawCatalogError::InvalidVersion);
        }
        if self.categories.is_empty() || self.categories.len() > MAX_RAW_CATEGORIES {
            return Err(RawCatalogError::InvalidCategoryCount(self.categories.len()));
        }
        if self.commands.is_empty() || self.commands.len() > MAX_RAW_COMMANDS {
            return Err(RawCatalogError::InvalidCommandCount(self.commands.len()));
        }

        let mut category_ids = BTreeSet::new();
        for category in &self.categories {
            validate_id_value("category", category.id.as_str())?;
            if !category_ids.insert(category.id.clone()) {
                return Err(RawCatalogError::DuplicateCategory(category.id.clone()));
            }
            if !valid_label(&category.label) || !valid_text(&category.reference_heading) {
                return Err(RawCatalogError::InvalidCategory(category.id.clone()));
            }
        }

        let mut command_ids = BTreeSet::new();
        let mut reference_ids = BTreeSet::new();
        for command in &self.commands {
            validate_id_value("command", command.id.as_str())?;
            if !command_ids.insert(command.id.clone()) {
                return Err(RawCatalogError::DuplicateCommand(command.id.clone()));
            }
            if !category_ids.contains(&command.category) {
                return Err(RawCatalogError::UnknownCategory {
                    command: command.id.clone(),
                    category: command.category.clone(),
                });
            }
            validate_command(command, &mut reference_ids)?;
        }
        Ok(())
    }

    pub fn category(&self, id: &RawCategoryId) -> Option<&RawCategory> {
        self.categories.iter().find(|category| &category.id == id)
    }

    /// Category order used by the Raw browser. Favorites is a pinned virtual
    /// collection; every reference-derived category retains catalog order.
    pub fn browser_categories(&self) -> Vec<&RawCategory> {
        self.categories
            .iter()
            .filter(|category| category.kind == RawCategoryKind::Favorites)
            .chain(
                self.categories
                    .iter()
                    .filter(|category| category.kind != RawCategoryKind::Favorites),
            )
            .collect()
    }

    pub fn command(&self, id: &RawCommandId) -> Option<&RawCommand> {
        self.commands.iter().find(|command| &command.id == id)
    }

    pub fn preview(
        &self,
        request: &RawPreviewRequest,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> Result<RawExecutionPreview, RawPreviewError> {
        self.validate()
            .map_err(|error| RawPreviewError::InvalidCatalog(error.to_string()))?;
        if request.catalog_version != self.version {
            return Err(RawPreviewError::StaleCatalog {
                current: self.version,
                received: request.catalog_version,
            });
        }
        let command = self
            .command(&request.command)
            .ok_or_else(|| RawPreviewError::UnknownCommand(request.command.clone()))?;
        let RawExecutionPolicy::Executable { template } = &command.execution else {
            return Err(RawPreviewError::ReferenceOnly(command.id.clone()));
        };
        let authority = authority.ok_or(RawPreviewError::MissingAuthority)?;
        if request.capability_generation != authority.snapshot.generation {
            return Err(RawPreviewError::StaleCapabilityGeneration {
                current: authority.snapshot.generation,
                received: request.capability_generation,
            });
        }
        let availability = command.availability(Some(authority));
        if !availability.is_enabled() {
            return Err(RawPreviewError::CapabilityUnavailable {
                state: availability.state,
                reasons: availability
                    .issues
                    .into_iter()
                    .map(|issue| issue.reason)
                    .collect(),
            });
        }
        let current_build_directory = authority
            .snapshot
            .environment
            .build_directory
            .value()
            .ok_or(RawPreviewError::MissingBuildDirectory)?;
        if current_build_directory != &request.build_directory {
            return Err(RawPreviewError::StaleBuildDirectory {
                current: current_build_directory.clone(),
                received: request.build_directory.clone(),
            });
        }
        validate_raw_preview_build_directory(current_build_directory)?;
        validate_raw_preview_parameters(command, &request.parameters)?;
        request
            .additional_arguments
            .validate()
            .map_err(RawPreviewError::InvalidAdditionalArguments)?;

        let mut arguments = Vec::new();
        let mut indexed_arguments = vec![RawPreviewArgument {
            index: 0,
            value: template.executable.as_str().into(),
            source: RawPreviewArgumentSource::Executable,
        }];
        for (template_index, argument) in template.arguments.iter().enumerate() {
            let Some(value) = render_raw_template_argument(argument, &request.parameters) else {
                continue;
            };
            push_raw_preview_argument(
                &mut arguments,
                &mut indexed_arguments,
                value,
                RawPreviewArgumentSource::Template {
                    index: template_index,
                },
            )?;
        }
        for (additional_index, value) in request.additional_arguments.as_slice().iter().enumerate()
        {
            push_raw_preview_argument(
                &mut arguments,
                &mut indexed_arguments,
                value.clone(),
                RawPreviewArgumentSource::Additional {
                    index: additional_index,
                },
            )?;
        }
        let limitations = availability
            .issues
            .iter()
            .flat_map(|issue| issue.limitations.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(RawExecutionPreview {
            catalog_version: self.version,
            command: command.id.clone(),
            executable: template.executable,
            arguments,
            indexed_arguments,
            capability_generation: authority.snapshot.generation,
            environment: authority.snapshot.environment.clone(),
            build_directory: current_build_directory.clone(),
            implementations: availability.implementations,
            capability_issues: availability.issues,
            interaction: template.interaction,
            safety: template.safety,
            limitations,
        })
    }
}

impl RawModeState {
    pub fn new(catalog: &RawCatalog) -> Self {
        let mut state = Self {
            catalog_version: catalog.version,
            category: catalog
                .browser_categories()
                .first()
                .map(|category| category.id.clone()),
            command: None,
            browser_column: RawBrowserColumn::Categories,
            view: RawModeView::Browser,
            focus: RawModeFocus::Categories,
            search: RawSearchState::default(),
            form: None,
            preview: None,
            execution: None,
            execution_states: BTreeMap::new(),
            history: Vec::new(),
            history_selection: 0,
            favorites: Vec::new(),
            favorite_selection: 0,
            favorite_confirmation: None,
            notification: None,
            return_stack: Vec::new(),
        };
        reconcile_raw_mode(&mut state, catalog);
        state
    }

    pub fn selected_command<'a>(&self, catalog: &'a RawCatalog) -> Option<&'a RawCommand> {
        self.command
            .as_ref()
            .and_then(|command| catalog.command(command))
    }

    pub fn visible_commands<'a>(&self, catalog: &'a RawCatalog) -> Vec<&'a RawCommand> {
        raw_visible_commands(self, catalog)
    }

    pub fn is_favorite(&self, command: &RawCommandId) -> bool {
        self.favorites.contains(command)
    }

    fn enter_view(&mut self, view: RawModeView, focus: RawModeFocus) {
        if self.return_stack.len() == MAX_RAW_VIEW_DEPTH {
            self.return_stack.remove(0);
        }
        self.return_stack.push((self.view, self.focus));
        self.view = view;
        self.focus = focus;
    }

    fn leave_view(&mut self) {
        match self.view {
            RawModeView::Preview => self.preview = None,
            RawModeView::Form => self.form = None,
            RawModeView::Execution => self.execution = None,
            RawModeView::Browser | RawModeView::History | RawModeView::Favorites => {}
        }
        if let Some((view, focus)) = self.return_stack.pop() {
            self.view = view;
            self.focus = focus;
        } else {
            self.view = RawModeView::Browser;
            self.focus = match self.browser_column {
                RawBrowserColumn::Categories => RawModeFocus::Categories,
                RawBrowserColumn::Commands => RawModeFocus::Commands,
            };
        }
    }

    fn close_unsafe_work(&mut self, reason: String) {
        self.form = None;
        self.preview = None;
        self.execution = None;
        self.return_stack.clear();
        self.view = RawModeView::Browser;
        self.browser_column = RawBrowserColumn::Commands;
        self.focus = RawModeFocus::Commands;
        self.notification = Some(reason);
    }
}

pub fn reduce_raw_mode(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
    action: RawModeAction,
) {
    reconcile_raw_mode(state, catalog);
    match action {
        RawModeAction::SelectCategory { delta } => select_raw_category(state, catalog, delta),
        RawModeAction::SelectCommand { delta } => select_raw_command(state, catalog, delta),
        RawModeAction::FocusCategories => {
            state.browser_column = RawBrowserColumn::Categories;
            state.focus = RawModeFocus::Categories;
        }
        RawModeAction::FocusCommands => {
            state.browser_column = RawBrowserColumn::Commands;
            state.focus = RawModeFocus::Commands;
        }
        RawModeAction::OpenSelected => open_raw_selected(state, catalog, authority),
        RawModeAction::Back => raw_mode_back(state),
        RawModeAction::BeginSearch => {
            state.search.editing = true;
            state.focus = RawModeFocus::Search;
        }
        RawModeAction::AppendSearch(character) => {
            if state.search.editing
                && !character.is_control()
                && state.search.query.len() + character.len_utf8() <= MAX_RAW_SEARCH_BYTES
            {
                state.search.query.push(character);
                reconcile_raw_command(state, catalog);
            }
        }
        RawModeAction::BackspaceSearch => {
            if state.search.editing {
                state.search.query.pop();
                reconcile_raw_command(state, catalog);
            }
        }
        RawModeAction::FinishSearch => {
            state.search.editing = false;
            state.focus = match state.browser_column {
                RawBrowserColumn::Categories => RawModeFocus::Categories,
                RawBrowserColumn::Commands => RawModeFocus::Commands,
            };
        }
        RawModeAction::ClearSearch => {
            state.search.query.clear();
            reconcile_raw_command(state, catalog);
        }
        RawModeAction::SetParameterInput { parameter, input } => {
            set_raw_parameter_input(state, catalog, &parameter, input)
        }
        RawModeAction::ChooseParameter { parameter, value } => {
            choose_raw_parameter(state, catalog, &parameter, value)
        }
        RawModeAction::EditParameterInput { parameter, command } => {
            edit_raw_parameter_input(state, catalog, &parameter, command)
        }
        RawModeAction::SelectFormField { delta } => select_raw_form_field(state, delta),
        RawModeAction::EditAdditionalArguments(command) => {
            if let Some(form) = state.form.as_mut()
                && let Err(error) = form.additional_arguments.apply(command)
            {
                state.notification = Some(error.to_string());
            }
        }
        RawModeAction::RequestPreview => request_raw_preview(state, catalog, authority),
        RawModeAction::ConfirmPreview | RawModeAction::CancelExecution(_) => {}
        RawModeAction::OpenExecution(command) => {
            if catalog.command(&command).is_some() {
                state.execution = Some(command);
                state.enter_view(RawModeView::Execution, RawModeFocus::Execution);
            }
        }
        RawModeAction::OpenHistory => {
            state.history_selection = state
                .history_selection
                .min(state.history.len().saturating_sub(1));
            state.enter_view(RawModeView::History, RawModeFocus::History);
        }
        RawModeAction::SelectHistory { delta } => {
            state.history_selection =
                shifted_index(state.history_selection, state.history.len(), delta)
        }
        RawModeAction::ActivateHistory => activate_raw_retained(state, catalog, true),
        RawModeAction::RememberHistory(command) => remember_raw_history(state, catalog, command),
        RawModeAction::OpenFavorites => {
            state.favorite_selection = state
                .favorite_selection
                .min(state.favorites.len().saturating_sub(1));
            state.enter_view(RawModeView::Favorites, RawModeFocus::Favorites);
        }
        RawModeAction::SelectFavorite { delta } => {
            state.favorite_selection =
                shifted_index(state.favorite_selection, state.favorites.len(), delta)
        }
        RawModeAction::ActivateFavorite => activate_raw_retained(state, catalog, false),
        RawModeAction::ToggleFavorite => toggle_raw_favorite(state, catalog),
        RawModeAction::ConfirmFavorite => confirm_raw_favorite(state),
        RawModeAction::CancelFavorite => cancel_raw_favorite(state),
        RawModeAction::ReprojectCatalog => reconcile_raw_mode(state, catalog),
        RawModeAction::ReprojectAuthority => reproject_raw_authority(state, catalog, authority),
        RawModeAction::DismissNotification => state.notification = None,
    }
    reconcile_raw_mode(state, catalog);
}

pub fn confirmed_raw_execution_request(
    state: &RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
    request_id: RawRequestId,
) -> Result<RawConfirmedExecutionRequest, RawExecutionError> {
    if state.view != RawModeView::Preview {
        return Err(RawExecutionError::InvalidLifecycle);
    }
    let form = state
        .form
        .as_ref()
        .ok_or(RawExecutionError::PreviewRequestMismatch)?;
    let reviewed = state
        .preview
        .as_ref()
        .ok_or(RawExecutionError::PreviewRequestMismatch)?;
    let command = catalog
        .command(&form.command)
        .ok_or(RawExecutionError::InvalidCommand)?;
    let mut parameters = BTreeMap::new();
    for definition in &command.parameters {
        let field = form
            .fields
            .get(&definition.id)
            .ok_or(RawExecutionError::InvalidParameterValue)?;
        if let Some(value) = definition
            .parse_value(&field.editor.text)
            .map_err(|_| RawExecutionError::InvalidParameterValue)?
        {
            parameters.insert(definition.id.clone(), value);
        }
    }
    let preview_request = RawPreviewRequest {
        catalog_version: state.catalog_version,
        command: form.command.clone(),
        parameters,
        additional_arguments: RawAdditionalArguments::parse(&form.additional_arguments.editor.text)
            .map_err(|_| RawExecutionError::InvalidParameterValue)?,
        capability_generation: form.capability_generation,
        build_directory: form.build_directory.clone(),
    };
    let current = catalog
        .preview(&preview_request, authority)
        .map_err(|error| RawExecutionError::InvalidReviewedPreview(error.to_string()))?;
    if &current != reviewed {
        return Err(RawExecutionError::PreviewRequestMismatch);
    }
    RawConfirmedExecutionRequest::from_reviewed_preview(
        request_id,
        catalog,
        &preview_request,
        reviewed,
    )
}

fn raw_visible_commands<'a>(state: &RawModeState, catalog: &'a RawCatalog) -> Vec<&'a RawCommand> {
    let query = state.search.query.to_lowercase();
    if !query.is_empty() {
        return catalog
            .commands
            .iter()
            .filter(|command| {
                let category = catalog.category(&command.category);
                [
                    command.label.as_str(),
                    command.description.as_str(),
                    command.reference.command.as_str(),
                    command.reference.description.as_str(),
                    category.map_or("", |category| category.label.as_str()),
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&query))
            })
            .collect();
    }
    let Some(category_id) = state.category.as_ref() else {
        return Vec::new();
    };
    if catalog
        .category(category_id)
        .is_some_and(|category| category.kind == RawCategoryKind::Favorites)
    {
        return state
            .favorites
            .iter()
            .filter_map(|command| catalog.command(command))
            .collect();
    }
    catalog
        .commands
        .iter()
        .filter(|command| &command.category == category_id)
        .collect()
}

fn reconcile_raw_mode(state: &mut RawModeState, catalog: &RawCatalog) {
    if state.catalog_version != catalog.version {
        state.catalog_version = catalog.version;
        if state.form.is_some() || state.preview.is_some() {
            state.close_unsafe_work(
                "Raw form closed because the catalog version was replaced.".into(),
            );
        }
    }
    state
        .favorites
        .retain(|command| catalog.command(command).is_some());
    state
        .history
        .retain(|command| catalog.command(command).is_some());
    if state
        .favorite_confirmation
        .as_ref()
        .is_some_and(|confirmation| catalog.command(&confirmation.command).is_none())
    {
        cancel_raw_favorite(state);
    }
    if state
        .category
        .as_ref()
        .is_none_or(|category| catalog.category(category).is_none())
    {
        state.category = catalog
            .browser_categories()
            .first()
            .map(|category| category.id.clone());
    }
    reconcile_raw_command(state, catalog);
    state.history_selection = state
        .history_selection
        .min(state.history.len().saturating_sub(1));
    state.favorite_selection = state
        .favorite_selection
        .min(state.favorites.len().saturating_sub(1));
}

fn reconcile_raw_command(state: &mut RawModeState, catalog: &RawCatalog) {
    let visible = raw_visible_commands(state, catalog);
    if state
        .command
        .as_ref()
        .is_none_or(|selected| !visible.iter().any(|command| &command.id == selected))
    {
        state.command = visible.first().map(|command| command.id.clone());
    }
}

fn shifted_index(current: usize, length: usize, delta: isize) -> usize {
    if length == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current
            .saturating_add(delta as usize)
            .min(length.saturating_sub(1))
    }
}

fn select_raw_category(state: &mut RawModeState, catalog: &RawCatalog, delta: isize) {
    let categories = catalog.browser_categories();
    let current = state
        .category
        .as_ref()
        .and_then(|selected| {
            categories
                .iter()
                .position(|category| &category.id == selected)
        })
        .unwrap_or(0);
    state.category = categories
        .get(shifted_index(current, categories.len(), delta))
        .map(|category| category.id.clone());
    state.command = None;
    reconcile_raw_command(state, catalog);
}

fn select_raw_command(state: &mut RawModeState, catalog: &RawCatalog, delta: isize) {
    let visible = raw_visible_commands(state, catalog);
    let current = state
        .command
        .as_ref()
        .and_then(|selected| visible.iter().position(|command| &command.id == selected))
        .unwrap_or(0);
    state.command = visible
        .get(shifted_index(current, visible.len(), delta))
        .map(|command| command.id.clone());
}

fn open_raw_selected(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    let Some(command) = state.selected_command(catalog) else {
        state.notification = Some("No Raw command is selected.".into());
        return;
    };
    let RawExecutionPolicy::Executable { .. } = &command.execution else {
        state.notification = Some(
            "This catalog entry is reference-only; its exact help remains inspectable.".into(),
        );
        return;
    };
    let availability = command.availability(authority);
    if !availability.is_enabled() {
        state.notification = Some(raw_availability_reason(&availability));
        return;
    }
    let Some(authority) = authority else {
        state.notification = Some("No current Raw capability authority is installed.".into());
        return;
    };
    let Some(build_directory) = authority.snapshot.environment.build_directory.value() else {
        state.notification =
            Some("The current capability authority has no build-directory identity.".into());
        return;
    };
    let fields = command
        .parameters
        .iter()
        .map(|parameter| {
            (
                parameter.id.clone(),
                RawFormField {
                    parameter: parameter.id.clone(),
                    editor: PopupEditor::new(String::new()),
                    value: None,
                    validation_error: None,
                },
            )
        })
        .collect();
    state.form = Some(RawCommandForm {
        command: command.id.clone(),
        fields,
        field_order: command
            .parameters
            .iter()
            .map(|parameter| parameter.id.clone())
            .collect(),
        field_selection: 0,
        additional_arguments: RawArgvEditor::new("").expect("empty Raw argv is bounded"),
        capability_generation: authority.snapshot.generation,
        build_directory: build_directory.clone(),
    });
    state.notification = None;
    state.enter_view(RawModeView::Form, RawModeFocus::Form);
}

fn raw_availability_reason(availability: &RawCommandAvailability) -> String {
    if availability.issues.is_empty() {
        return format!("Raw command is {:?}.", availability.state);
    }
    availability
        .issues
        .iter()
        .map(|issue| issue.reason.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

fn raw_mode_back(state: &mut RawModeState) {
    if state.search.editing {
        state.search.editing = false;
        state.focus = match state.browser_column {
            RawBrowserColumn::Categories => RawModeFocus::Categories,
            RawBrowserColumn::Commands => RawModeFocus::Commands,
        };
        return;
    }
    if state.view != RawModeView::Browser {
        state.leave_view();
    } else if state.browser_column == RawBrowserColumn::Commands {
        state.browser_column = RawBrowserColumn::Categories;
        state.focus = RawModeFocus::Categories;
    }
}

fn set_raw_parameter_input(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    parameter: &RawParameterId,
    input: String,
) {
    let Some(form) = state.form.as_mut() else {
        return;
    };
    let Some(definition) = catalog
        .command(&form.command)
        .and_then(|command| command.parameters.iter().find(|item| &item.id == parameter))
    else {
        state.notification = Some(format!("Raw form has no parameter {parameter}."));
        return;
    };
    let Some(field) = form.fields.get_mut(parameter) else {
        state.notification = Some(format!("Raw form state has no parameter {parameter}."));
        return;
    };
    if input.len() > raw_parameter_input_limit(definition.kind) {
        state.notification = Some(format!(
            "Raw parameter {parameter} input exceeds its typed byte limit."
        ));
        return;
    }
    field.editor = PopupEditor::new(input);
    match definition.parse_value(&field.editor.text) {
        Ok(value) => {
            field.value = value;
            field.validation_error = None;
        }
        Err(error) => {
            field.value = None;
            field.validation_error = Some(error);
        }
    }
    state.preview = None;
}

fn raw_parameter_input_limit(kind: RawParameterKind) -> usize {
    match kind {
        RawParameterKind::Recipe => MAX_RAW_RECIPE_BYTES,
        RawParameterKind::Image => MAX_RAW_IMAGE_BYTES,
        RawParameterKind::Target => MAX_RAW_TARGET_BYTES,
        RawParameterKind::Task => MAX_RAW_TASK_BYTES,
        RawParameterKind::UserInterface => MAX_RAW_UI_BYTES,
        RawParameterKind::File => MAX_RAW_FILE_BYTES,
        RawParameterKind::Integer => MAX_RAW_INTEGER_INPUT_BYTES,
        RawParameterKind::Text => MAX_RAW_PARAMETER_TEXT_BYTES,
        RawParameterKind::Multiconfig => MAX_RAW_MULTICONFIG_BYTES,
    }
}

fn edit_raw_parameter_input(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    parameter: &RawParameterId,
    command: PopupEditorCommand,
) {
    let Some(form) = state.form.as_mut() else {
        return;
    };
    let Some(definition) = catalog
        .command(&form.command)
        .and_then(|command| command.parameters.iter().find(|item| &item.id == parameter))
    else {
        state.notification = Some(format!("Raw form has no parameter {parameter}."));
        return;
    };
    let Some(field) = form.fields.get_mut(parameter) else {
        state.notification = Some(format!("Raw form state has no parameter {parameter}."));
        return;
    };
    let previous = field.editor.clone();
    match command {
        PopupEditorCommand::ToggleInsert => field.editor.editing = !field.editor.editing,
        PopupEditorCommand::Insert(character)
            if field.editor.editing && !character.is_control() =>
        {
            field.editor.insert(&character.to_string());
        }
        PopupEditorCommand::Insert(_) => {}
        PopupEditorCommand::Backspace if field.editor.editing => field.editor.backspace(),
        PopupEditorCommand::Backspace => {}
        PopupEditorCommand::Left => field.editor.left(),
        PopupEditorCommand::Right => field.editor.right(),
        PopupEditorCommand::Up => field.editor.up(),
        PopupEditorCommand::Down => field.editor.down(),
        PopupEditorCommand::Home => field.editor.home(),
        PopupEditorCommand::End => field.editor.end(),
        PopupEditorCommand::SelectValue => {
            field.editor.select_range(0, field.editor.text.len());
            field.editor.editing = true;
        }
        PopupEditorCommand::Copy => {
            field.editor.copy_selection_or_line();
        }
        PopupEditorCommand::Paste if field.editor.editing => field.editor.paste(),
        PopupEditorCommand::Paste => {}
    }
    if field.editor.text.len() > raw_parameter_input_limit(definition.kind) {
        field.editor = previous;
    }
    match definition.parse_value(&field.editor.text) {
        Ok(value) => {
            field.value = value;
            field.validation_error = None;
        }
        Err(error) => {
            field.value = None;
            field.validation_error = Some(error);
        }
    }
    state.preview = None;
}

fn choose_raw_parameter(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    parameter: &RawParameterId,
    value: RawParameterValue,
) {
    let Some(form) = state.form.as_ref() else {
        return;
    };
    let Some(definition) = catalog
        .command(&form.command)
        .and_then(|command| command.parameters.iter().find(|item| &item.id == parameter))
    else {
        state.notification = Some(format!("Raw form has no parameter {parameter}."));
        return;
    };
    match definition.validate_value(&value) {
        Ok(()) => set_raw_parameter_input(state, catalog, parameter, value.argument()),
        Err(error) => {
            if let Some(field) = state
                .form
                .as_mut()
                .and_then(|form| form.fields.get_mut(parameter))
            {
                field.value = None;
                field.validation_error = Some(error.clone());
            }
            state.notification = Some(error.to_string());
        }
    }
}

fn select_raw_form_field(state: &mut RawModeState, delta: isize) {
    if let Some(form) = state.form.as_mut() {
        for field in form.fields.values_mut() {
            field.editor.editing = false;
        }
        form.additional_arguments.editor.editing = false;
        form.field_selection = shifted_index(
            form.field_selection,
            form.field_order.len().saturating_add(1),
            delta,
        );
    }
}

fn request_raw_preview(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    let request = {
        let Some(form) = state.form.as_mut() else {
            state.notification = Some("No Raw command form is open.".into());
            return;
        };
        let Some(command) = catalog.command(&form.command) else {
            state.close_unsafe_work("Raw form command is no longer in the catalog.".into());
            return;
        };
        let mut values = BTreeMap::new();
        let mut first_error = None;
        for definition in &command.parameters {
            let Some(field) = form.fields.get_mut(&definition.id) else {
                first_error.get_or_insert_with(|| {
                    format!("Raw form state has no parameter {}.", definition.id)
                });
                continue;
            };
            match definition.parse_value(&field.editor.text) {
                Ok(value) => {
                    field.value.clone_from(&value);
                    field.validation_error = None;
                    if let Some(value) = value {
                        values.insert(definition.id.clone(), value);
                    }
                }
                Err(error) => {
                    field.value = None;
                    field.validation_error = Some(error.clone());
                    first_error.get_or_insert_with(|| error.to_string());
                }
            }
        }
        let additional_arguments = match form.additional_arguments.validate() {
            Ok(arguments) => arguments.clone(),
            Err(error) => {
                first_error.get_or_insert_with(|| error.to_string());
                RawAdditionalArguments::default()
            }
        };
        if let Some(error) = first_error {
            state.notification = Some(error);
            return;
        }
        RawPreviewRequest {
            catalog_version: state.catalog_version,
            command: form.command.clone(),
            parameters: values,
            additional_arguments,
            capability_generation: form.capability_generation,
            build_directory: form.build_directory.clone(),
        }
    };
    match catalog.preview(&request, authority) {
        Ok(preview) => {
            state.preview = Some(preview);
            state.notification = None;
            state.enter_view(RawModeView::Preview, RawModeFocus::Preview);
        }
        Err(error) => state.notification = Some(error.to_string()),
    }
}

fn remember_raw_history(state: &mut RawModeState, catalog: &RawCatalog, command: RawCommandId) {
    if catalog.command(&command).is_none() {
        return;
    }
    state.history.insert(0, command);
    state.history.truncate(MAX_RAW_HISTORY_STUBS);
    state.history_selection = 0;
}

fn activate_raw_retained(state: &mut RawModeState, catalog: &RawCatalog, history: bool) {
    let command = if history {
        state.history.get(state.history_selection)
    } else {
        state.favorites.get(state.favorite_selection)
    }
    .cloned();
    let Some(command) = command.and_then(|command| {
        catalog
            .command(&command)
            .map(|catalog_command| (command, catalog_command.category.clone()))
    }) else {
        state.notification = Some("The retained Raw command is stale or unavailable.".into());
        return;
    };
    state.command = Some(command.0);
    state.category = Some(command.1);
    state.search.query.clear();
    state.search.editing = false;
    state.return_stack.clear();
    state.view = RawModeView::Browser;
    state.browser_column = RawBrowserColumn::Commands;
    state.focus = RawModeFocus::Commands;
}

fn toggle_raw_favorite(state: &mut RawModeState, catalog: &RawCatalog) {
    let Some(command) = state.command.clone() else {
        state.notification = Some("No Raw command is selected.".into());
        return;
    };
    if catalog.command(&command).is_none() {
        state.notification = Some("The selected Raw command is stale.".into());
        return;
    }
    if state.favorites.contains(&command) {
        state.favorite_confirmation = Some(RawFavoriteConfirmation {
            command,
            return_focus: state.focus,
        });
        state.focus = RawModeFocus::FavoriteConfirmation;
        state.notification = Some("Confirm removal of the exact Raw favorite.".into());
    } else if state.favorites.len() == MAX_RAW_FAVORITES {
        state.notification = Some(format!(
            "Raw favorites are bounded to {MAX_RAW_FAVORITES} entries."
        ));
    } else {
        state.favorites.push(command);
        state.favorite_selection = state.favorites.len().saturating_sub(1);
        state.notification = Some("Raw favorite added.".into());
    }
}

fn confirm_raw_favorite(state: &mut RawModeState) {
    let Some(confirmation) = state.favorite_confirmation.take() else {
        return;
    };
    state
        .favorites
        .retain(|favorite| favorite != &confirmation.command);
    state.favorite_selection = state
        .favorite_selection
        .min(state.favorites.len().saturating_sub(1));
    state.focus = confirmation.return_focus;
    state.notification = Some("Raw favorite removed.".into());
}

fn cancel_raw_favorite(state: &mut RawModeState) {
    let Some(confirmation) = state.favorite_confirmation.take() else {
        return;
    };
    state.focus = confirmation.return_focus;
    state.notification = None;
}

fn reproject_raw_authority(
    state: &mut RawModeState,
    catalog: &RawCatalog,
    authority: Option<&DaemonCompatibilitySnapshot>,
) {
    if state.view == RawModeView::Execution {
        return;
    }
    let Some(form) = state.form.as_ref() else {
        return;
    };
    let Some(command) = catalog.command(&form.command) else {
        state.close_unsafe_work("Raw form closed because its command was removed.".into());
        return;
    };
    let availability = command.availability(authority);
    if !availability.is_enabled() {
        state.close_unsafe_work(format!(
            "Raw form closed after capability update: {}",
            raw_availability_reason(&availability)
        ));
        return;
    }
    let Some(authority) = authority else {
        state.close_unsafe_work("Raw form closed because capability authority was lost.".into());
        return;
    };
    let Some(build_directory) = authority.snapshot.environment.build_directory.value() else {
        state.close_unsafe_work(
            "Raw form closed because build-directory authority was lost.".into(),
        );
        return;
    };
    if build_directory != &form.build_directory {
        state.close_unsafe_work(
            "Raw form closed because the authoritative build directory changed.".into(),
        );
        return;
    }
    let preview_stale = state.preview.as_ref().is_some_and(|preview| {
        preview.capability_generation != authority.snapshot.generation
            || preview.build_directory != *build_directory
    });
    if preview_stale && state.view == RawModeView::Preview {
        state.leave_view();
        state.notification = Some(
            "Raw preview closed after a safe capability generation update; review it again.".into(),
        );
    } else if preview_stale {
        state.preview = None;
    }
    if let Some(form) = state.form.as_mut() {
        form.capability_generation = authority.snapshot.generation;
        form.build_directory.clone_from(build_directory);
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawCatalogError {
    #[error("Raw catalog version must be non-zero")]
    InvalidVersion,
    #[error("Raw catalog category count is invalid: {0}")]
    InvalidCategoryCount(usize),
    #[error("Raw catalog command count is invalid: {0}")]
    InvalidCommandCount(usize),
    #[error("invalid {field} identity: {value:?}")]
    InvalidIdentity { field: &'static str, value: String },
    #[error("duplicate Raw category: {0}")]
    DuplicateCategory(RawCategoryId),
    #[error("duplicate Raw command: {0}")]
    DuplicateCommand(RawCommandId),
    #[error("duplicate Raw reference: {0}")]
    DuplicateReference(RawReferenceId),
    #[error("invalid Raw category: {0}")]
    InvalidCategory(RawCategoryId),
    #[error("Raw command {command} refers to unknown category {category}")]
    UnknownCategory {
        command: RawCommandId,
        category: RawCategoryId,
    },
    #[error("invalid Raw command: {0}")]
    InvalidCommand(RawCommandId),
    #[error("Raw command {0} has invalid or duplicate parameters")]
    InvalidParameters(RawCommandId),
    #[error("Raw command {0} parameter placeholders disagree with its argv template")]
    PlaceholderDisagreement(RawCommandId),
    #[error("Raw command {0} contains an unsafe argv template")]
    UnsafeTemplate(RawCommandId),
    #[error("Raw command {0} has invalid capability requirements")]
    InvalidCapabilityRequirement(RawCommandId),
    #[error("Raw command {0} does not have a coherent execution policy")]
    InvalidExecutionPolicy(RawCommandId),
}

#[derive(Debug, Clone, Copy)]
enum RawRequirementOperator {
    All,
    Any,
}

struct RawCapabilityResult {
    id: CapabilityId,
    state: RawAvailabilityState,
    reason: Option<String>,
    limitations: Vec<String>,
    implementation: Option<String>,
}

impl RawCapabilityResult {
    fn is_enabled(&self) -> bool {
        self.state.is_enabled() && self.implementation.is_some()
    }

    fn issue(&self) -> Option<RawCapabilityIssue> {
        self.reason.as_ref().map(|reason| RawCapabilityIssue {
            capability: Some(self.id),
            reason: reason.clone(),
            limitations: self.limitations.clone(),
        })
    }
}

fn project_raw_capabilities(
    authority: Option<&DaemonCompatibilitySnapshot>,
    capabilities: &[CapabilityId],
    operator: RawRequirementOperator,
) -> RawCommandAvailability {
    let results = capabilities
        .iter()
        .copied()
        .map(|id| raw_capability_result(authority, id))
        .collect::<Vec<_>>();

    let selected = match operator {
        RawRequirementOperator::All if results.iter().all(RawCapabilityResult::is_enabled) => {
            Some(results.iter().collect::<Vec<_>>())
        }
        RawRequirementOperator::Any => results
            .iter()
            .find(|result| result.state == RawAvailabilityState::Available && result.is_enabled())
            .or_else(|| results.iter().find(|result| result.is_enabled()))
            .map(|result| vec![result]),
        RawRequirementOperator::All => None,
    };

    if let Some(selected) = selected {
        let limited = selected
            .iter()
            .any(|result| result.state == RawAvailabilityState::Limited);
        return RawCommandAvailability {
            state: if limited {
                RawAvailabilityState::Limited
            } else {
                RawAvailabilityState::Available
            },
            issues: selected
                .iter()
                .filter_map(|result| result.issue())
                .collect(),
            implementations: selected
                .iter()
                .filter_map(|result| {
                    result
                        .implementation
                        .as_ref()
                        .map(|implementation| (result.id, implementation.clone()))
                })
                .collect(),
        };
    }

    let failures = results
        .iter()
        .filter(|result| !result.is_enabled())
        .collect::<Vec<_>>();
    let state = if failures
        .iter()
        .any(|result| result.state == RawAvailabilityState::Unknown)
    {
        RawAvailabilityState::Unknown
    } else if !failures.is_empty()
        && failures
            .iter()
            .all(|result| result.state == RawAvailabilityState::Unsupported)
    {
        RawAvailabilityState::Unsupported
    } else {
        RawAvailabilityState::Unavailable
    };
    RawCommandAvailability {
        state,
        issues: failures
            .iter()
            .filter_map(|result| result.issue())
            .collect(),
        implementations: Vec::new(),
    }
}

fn raw_capability_result(
    authority: Option<&DaemonCompatibilitySnapshot>,
    id: CapabilityId,
) -> RawCapabilityResult {
    let Some(authority) = authority else {
        return RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unknown,
            reason: Some(format!(
                "No current environment capability snapshot: {}.",
                id.as_str()
            )),
            limitations: Vec::new(),
            implementation: None,
        };
    };
    let Some(record) = authority.snapshot.capability(id) else {
        return RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unknown,
            reason: Some(format!("{} has no capability evidence.", id.as_str())),
            limitations: Vec::new(),
            implementation: None,
        };
    };
    let implementation = authority
        .implementations
        .get(&id)
        .map(|implementation| implementation.id.clone());
    match &record.state {
        CapabilityState::Available if implementation.is_some() => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Available,
            reason: None,
            limitations: Vec::new(),
            implementation,
        },
        CapabilityState::Available => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unknown,
            reason: Some(format!(
                "{} is enabled but has no selected implementation.",
                id.as_str()
            )),
            limitations: Vec::new(),
            implementation: None,
        },
        CapabilityState::AvailableWithLimitations {
            reason,
            limitations,
        } if implementation.is_some() => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Limited,
            reason: Some(reason.message.clone()),
            limitations: limitations.clone(),
            implementation,
        },
        CapabilityState::AvailableWithLimitations {
            reason,
            limitations,
        } => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unknown,
            reason: Some(format!(
                "{} {} is limited but has no selected implementation.",
                reason.message,
                id.as_str()
            )),
            limitations: limitations.clone(),
            implementation: None,
        },
        CapabilityState::Unavailable { reason } => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unavailable,
            reason: Some(reason.message.clone()),
            limitations: Vec::new(),
            implementation: None,
        },
        CapabilityState::Unknown { reason } => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unknown,
            reason: Some(reason.message.clone()),
            limitations: Vec::new(),
            implementation: None,
        },
        CapabilityState::Unsupported { reason } => RawCapabilityResult {
            id,
            state: RawAvailabilityState::Unsupported,
            reason: Some(reason.message.clone()),
            limitations: Vec::new(),
            implementation: None,
        },
    }
}

fn validate_command(
    command: &RawCommand,
    reference_ids: &mut BTreeSet<RawReferenceId>,
) -> Result<(), RawCatalogError> {
    validate_id_value("reference", command.reference.id.as_str())?;
    if !reference_ids.insert(command.reference.id.clone()) {
        return Err(RawCatalogError::DuplicateReference(
            command.reference.id.clone(),
        ));
    }
    if !valid_label(&command.label)
        || !valid_text(&command.description)
        || !valid_label(&command.reference.heading)
        || !valid_reference_command(&command.reference.command)
        || !valid_text(&command.reference.description)
        || command.parameters.len() > MAX_RAW_PARAMETERS
    {
        return Err(RawCatalogError::InvalidCommand(command.id.clone()));
    }

    let mut parameters = BTreeSet::new();
    for parameter in &command.parameters {
        validate_id_value("parameter", parameter.id.as_str())?;
        if !parameters.insert(parameter.id.clone())
            || !valid_label(&parameter.label)
            || !valid_placeholder(&parameter.placeholder)
        {
            return Err(RawCatalogError::InvalidParameters(command.id.clone()));
        }
    }

    match &command.execution {
        RawExecutionPolicy::ReferenceOnly { reason, .. } => {
            if !command.parameters.is_empty() || !valid_text(reason) {
                return Err(RawCatalogError::InvalidExecutionPolicy(command.id.clone()));
            }
        }
        RawExecutionPolicy::Executable { template } => {
            validate_executable(command, template, &parameters)?;
        }
    }
    Ok(())
}

fn validate_executable(
    command: &RawCommand,
    template: &RawExecutableTemplate,
    parameters: &BTreeSet<RawParameterId>,
) -> Result<(), RawCatalogError> {
    let capabilities = template.capabilities.capabilities();
    let unique_capabilities = capabilities.iter().copied().collect::<BTreeSet<_>>();
    if capabilities.is_empty()
        || capabilities.len() > MAX_RAW_CAPABILITY_REQUIREMENTS
        || unique_capabilities.len() != capabilities.len()
    {
        return Err(RawCatalogError::InvalidCapabilityRequirement(
            command.id.clone(),
        ));
    }

    let mut placeholders = BTreeSet::new();
    for argument in &template.arguments {
        let parameter = match argument {
            RawArgument::Literal { value } => {
                if !valid_argv_fragment(value) {
                    return Err(RawCatalogError::UnsafeTemplate(command.id.clone()));
                }
                None
            }
            RawArgument::Empty => None,
            RawArgument::Parameter { parameter } => Some(parameter),
            RawArgument::JoinedParameter { prefix, parameter } => {
                if !valid_argv_prefix(prefix) {
                    return Err(RawCatalogError::UnsafeTemplate(command.id.clone()));
                }
                Some(parameter)
            }
            RawArgument::Composed { segments } => {
                if segments.len() < 2
                    || segments.len() > MAX_RAW_PARAMETERS * 2 + 1
                    || segments.iter().any(|segment| match segment {
                        RawArgumentSegment::Literal { value } => !valid_composed_literal(value),
                        RawArgumentSegment::Parameter { .. } => false,
                    })
                {
                    return Err(RawCatalogError::UnsafeTemplate(command.id.clone()));
                }
                for segment in segments {
                    if let RawArgumentSegment::Parameter { parameter } = segment {
                        placeholders.insert(parameter.clone());
                    }
                }
                None
            }
        };
        if let Some(parameter) = parameter {
            placeholders.insert(parameter.clone());
        }
    }
    if &placeholders != parameters
        || template.display_template(&command.parameters).as_deref()
            != Some(command.reference.command.as_str())
    {
        return Err(RawCatalogError::PlaceholderDisagreement(command.id.clone()));
    }
    Ok(())
}

fn validate_id_value(field: &'static str, value: &str) -> Result<(), RawCatalogError> {
    if valid_id(value) {
        Ok(())
    } else {
        Err(RawCatalogError::InvalidIdentity {
            field,
            value: value.to_owned(),
        })
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RAW_ID_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
        })
        && value.as_bytes().first().is_some_and(u8::is_ascii_lowercase)
        && value
            .as_bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

fn valid_label(value: &str) -> bool {
    valid_bounded_text(value, MAX_RAW_LABEL_BYTES)
}

fn valid_text(value: &str) -> bool {
    valid_bounded_text(value, MAX_RAW_TEXT_BYTES)
}

fn valid_reference_command(value: &str) -> bool {
    valid_bounded_text(value, MAX_RAW_REFERENCE_COMMAND_BYTES)
}

fn valid_placeholder(value: &str) -> bool {
    valid_bounded_text(value, MAX_RAW_ARG_BYTES)
        && !value.chars().any(char::is_whitespace)
        && !value
            .chars()
            .any(|character| matches!(character, '|' | '&' | ';' | '`' | '$'))
}

fn valid_bounded_text(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_argv_fragment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RAW_ARG_BYTES
        && !value.chars().any(char::is_whitespace)
        && !contains_shell_syntax(value)
}

fn valid_argv_prefix(value: &str) -> bool {
    valid_argv_fragment(value) && (value.starts_with('-') || value.ends_with('='))
}

fn valid_composed_literal(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_RAW_ARG_BYTES
        && !value.chars().any(char::is_whitespace)
        && !contains_shell_syntax(value)
}

fn unavailable_selector(reason: impl Into<String>) -> RawSelectorInventory {
    RawSelectorInventory::Unavailable {
        reason: reason.into(),
    }
}

fn project_recipe_choices(recipes: Option<&[Recipe]>) -> RawSelectorInventory {
    let Some(recipes) = recipes else {
        return unavailable_selector("The current recipe inventory is unavailable.");
    };
    let mut identities = BTreeSet::new();
    let mut choices = Vec::with_capacity(recipes.len());
    for recipe in recipes {
        if !valid_raw_identifier(&recipe.name, MAX_RAW_RECIPE_BYTES) {
            return unavailable_selector(format!(
                "The recipe inventory contains an invalid identity: {:?}.",
                recipe.name
            ));
        }
        let identity = RawSelectorIdentity::Recipe {
            name: recipe.name.clone(),
            file: recipe.file.clone(),
        };
        if identities.insert(identity.clone()) {
            choices.push(RawSelectorChoice {
                identity,
                value: RawParameterValue::Recipe(recipe.name.clone()),
            });
        }
    }
    RawSelectorInventory::Available { choices }
}

fn project_image_choices(images: Option<&[String]>) -> RawSelectorInventory {
    let Some(images) = images else {
        return unavailable_selector("The current image inventory is unavailable.");
    };
    project_named_choices(images, RawParameterKind::Image, |name| RawSelectorChoice {
        identity: RawSelectorIdentity::Image(name.to_owned()),
        value: RawParameterValue::Image(name.to_owned()),
    })
}

fn project_target_choices(
    current_target: Option<&str>,
    recent_targets: Option<&[String]>,
) -> RawSelectorInventory {
    if current_target.is_none() && recent_targets.is_none() {
        return unavailable_selector("Current and recent target authority is unavailable.");
    }
    let values = current_target
        .into_iter()
        .chain(recent_targets.into_iter().flatten().map(String::as_str));
    let mut names = BTreeSet::new();
    let mut choices = Vec::new();
    for name in values {
        if !valid_raw_target(name) {
            return unavailable_selector(format!(
                "The target inventory contains an invalid identity: {name:?}."
            ));
        }
        if names.insert(name.to_owned()) {
            choices.push(RawSelectorChoice {
                identity: RawSelectorIdentity::Target(name.to_owned()),
                value: RawParameterValue::Target(name.to_owned()),
            });
        }
    }
    RawSelectorInventory::Available { choices }
}

fn project_task_choices(
    selected_recipe: Option<&str>,
    metadata: Option<&RecipeMetadata>,
    pending: bool,
    error: Option<&str>,
) -> RawSelectorInventory {
    let Some(recipe) = selected_recipe else {
        return unavailable_selector("Select a recipe before choosing a task.");
    };
    if pending {
        return unavailable_selector(format!("Task metadata for {recipe} is still loading."));
    }
    if let Some(error) = error {
        return unavailable_selector(format!("Task metadata for {recipe} failed: {error}"));
    }
    let Some(metadata) = metadata else {
        return unavailable_selector(format!("Task metadata for {recipe} is unavailable."));
    };
    if metadata.recipe != recipe {
        return unavailable_selector(format!(
            "Task metadata for {} cannot populate the {recipe} selection.",
            metadata.recipe
        ));
    }
    let Some(tasks) = metadata.tasks.as_deref() else {
        return unavailable_selector(format!(
            "Task metadata for {recipe} does not include a task inventory."
        ));
    };
    let mut names = BTreeSet::new();
    let mut choices = Vec::with_capacity(tasks.len());
    for task in tasks {
        if !valid_raw_identifier(task, MAX_RAW_TASK_BYTES) {
            return unavailable_selector(format!(
                "Task metadata for {recipe} contains an invalid identity: {task:?}."
            ));
        }
        if names.insert(task.clone()) {
            choices.push(RawSelectorChoice {
                identity: RawSelectorIdentity::Task {
                    recipe: recipe.to_owned(),
                    task: task.clone(),
                },
                value: RawParameterValue::Task(task.clone()),
            });
        }
    }
    RawSelectorInventory::Available { choices }
}

fn project_multiconfig_choices(multiconfig: Option<&str>) -> RawSelectorInventory {
    let Some(multiconfig) = multiconfig else {
        return unavailable_selector("The current multiconfig inventory is unavailable.");
    };
    let values = multiconfig
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    project_named_choices(&values, RawParameterKind::Multiconfig, |name| {
        RawSelectorChoice {
            identity: RawSelectorIdentity::Multiconfig(name.to_owned()),
            value: RawParameterValue::Multiconfig(name.to_owned()),
        }
    })
}

fn project_named_choices(
    values: &[String],
    kind: RawParameterKind,
    choice: impl Fn(&str) -> RawSelectorChoice,
) -> RawSelectorInventory {
    let (maximum, label) = match kind {
        RawParameterKind::Image => (MAX_RAW_IMAGE_BYTES, "image"),
        RawParameterKind::Multiconfig => (MAX_RAW_MULTICONFIG_BYTES, "multiconfig"),
        _ => return unavailable_selector("The selector projection kind is unsupported."),
    };
    let mut names = BTreeSet::new();
    let mut choices = Vec::with_capacity(values.len());
    for name in values {
        if !valid_raw_identifier(name, maximum) {
            return unavailable_selector(format!(
                "The {label} inventory contains an invalid identity: {name:?}."
            ));
        }
        if names.insert(name.clone()) {
            choices.push(choice(name));
        }
    }
    RawSelectorInventory::Available { choices }
}

fn template_references_parameter(
    template: &RawExecutableTemplate,
    parameter: &RawParameterId,
) -> bool {
    template.arguments.iter().any(|argument| match argument {
        RawArgument::Parameter {
            parameter: candidate,
        }
        | RawArgument::JoinedParameter {
            parameter: candidate,
            ..
        } => candidate == parameter,
        RawArgument::Composed { segments } => segments.iter().any(|segment| {
            matches!(
                segment,
                RawArgumentSegment::Parameter {
                    parameter: candidate
                } if candidate == parameter
            )
        }),
        RawArgument::Literal { .. } | RawArgument::Empty => false,
    })
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

fn validate_raw_preview_build_directory(path: &Path) -> Result<(), RawPreviewError> {
    if path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, Component::ParentDir))
    {
        Ok(())
    } else {
        Err(RawPreviewError::InvalidBuildDirectory(path.to_path_buf()))
    }
}

fn validate_raw_preview_parameters(
    command: &RawCommand,
    values: &BTreeMap<RawParameterId, RawParameterValue>,
) -> Result<(), RawPreviewError> {
    if let Some(parameter) = values
        .keys()
        .find(|parameter| !command.parameters.iter().any(|item| &item.id == *parameter))
    {
        return Err(RawPreviewError::UnknownParameter(parameter.clone()));
    }
    for parameter in &command.parameters {
        match values.get(&parameter.id) {
            Some(value) => parameter.validate_value(value)?,
            None if parameter.presence == RawParameterPresence::Required => {
                return Err(RawPreviewError::MissingParameter(parameter.id.clone()));
            }
            None => {}
        }
    }
    Ok(())
}

fn render_raw_template_argument(
    argument: &RawArgument,
    values: &BTreeMap<RawParameterId, RawParameterValue>,
) -> Option<String> {
    match argument {
        RawArgument::Literal { value } => Some(value.clone()),
        RawArgument::Empty => Some(String::new()),
        RawArgument::Parameter { parameter } => {
            values.get(parameter).map(RawParameterValue::argument)
        }
        RawArgument::JoinedParameter { prefix, parameter } => values
            .get(parameter)
            .map(|value| format!("{prefix}{}", value.argument())),
        RawArgument::Composed { segments } => {
            let mut value = String::new();
            for segment in segments {
                match segment {
                    RawArgumentSegment::Literal { value: literal } => value.push_str(literal),
                    RawArgumentSegment::Parameter { parameter } => {
                        value.push_str(&values.get(parameter)?.argument());
                    }
                }
            }
            Some(value)
        }
    }
}

fn push_raw_preview_argument(
    arguments: &mut Vec<String>,
    indexed: &mut Vec<RawPreviewArgument>,
    argument: String,
    source: RawPreviewArgumentSource,
) -> Result<(), RawPreviewError> {
    let index = indexed.len();
    if index == MAX_RAW_PREVIEW_ARGUMENTS {
        return Err(RawPreviewError::TooManyArguments {
            count: index + 1,
            maximum: MAX_RAW_PREVIEW_ARGUMENTS,
        });
    }
    if argument.len() > MAX_RAW_PREVIEW_ARGUMENT_BYTES {
        return Err(RawPreviewError::ArgumentTooLong {
            argument: index,
            bytes: argument.len(),
            maximum: MAX_RAW_PREVIEW_ARGUMENT_BYTES,
        });
    }
    indexed.push(RawPreviewArgument {
        index,
        value: argument.clone(),
        source,
    });
    arguments.push(argument);
    Ok(())
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

#[cfg(test)]
mod raw_mode_state_tests {
    use super::*;
    use crate::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
        CapabilitySnapshot, IdentityAuthority, YoctoEnvironmentIdentity,
    };

    fn category(value: &str) -> RawCategoryId {
        RawCategoryId::new(value).unwrap()
    }

    fn command(value: &str) -> RawCommandId {
        RawCommandId::new(value).unwrap()
    }

    fn parameter(value: &str) -> RawParameterId {
        RawParameterId::new(value).unwrap()
    }

    fn executable(id: &str, label: &str, description: &str, target: bool) -> RawCommand {
        let parameters = target.then(|| RawParameter {
            id: parameter("target"),
            label: "Target".into(),
            placeholder: "<target>".into(),
            kind: RawParameterKind::Target,
            presence: RawParameterPresence::Required,
        });
        RawCommand {
            id: command(id),
            category: category("build"),
            label: label.into(),
            description: description.into(),
            reference: RawReference {
                id: RawReferenceId::new(format!("{id}.reference")).unwrap(),
                heading: "Build".into(),
                command: if target {
                    "bitbake <target>".into()
                } else {
                    "bitbake --version".into()
                },
                description: description.into(),
            },
            parameters: parameters.iter().cloned().collect(),
            execution: RawExecutionPolicy::Executable {
                template: RawExecutableTemplate {
                    executable: RawExecutable::BitBake,
                    arguments: if target {
                        vec![RawArgument::Parameter {
                            parameter: parameter("target"),
                        }]
                    } else {
                        vec![RawArgument::Literal {
                            value: "--version".into(),
                        }]
                    },
                    capabilities: RawCapabilityRequirement::All {
                        capabilities: vec![CapabilityId::BitBakeRawCli],
                    },
                    interaction: RawInteractionMode::NoninteractiveJob,
                    safety: RawSafetyClass::Inspection,
                },
            },
        }
    }

    fn catalog(version: u16) -> RawCatalog {
        RawCatalog {
            version,
            categories: vec![
                RawCategory {
                    id: category("favorites"),
                    label: "Favorites".into(),
                    reference_heading: "Favorites".into(),
                    kind: RawCategoryKind::Favorites,
                },
                RawCategory {
                    id: category("build"),
                    label: "Build commands".into(),
                    reference_heading: "Build commands".into(),
                    kind: RawCategoryKind::Executable,
                },
                RawCategory {
                    id: category("reference"),
                    label: "Reference material".into(),
                    reference_heading: "Reference material".into(),
                    kind: RawCategoryKind::ReferenceOnly,
                },
            ],
            commands: vec![
                executable(
                    "build.target",
                    "Build target",
                    "Build one exact target.",
                    true,
                ),
                executable(
                    "build.version",
                    "Show BitBake version",
                    "Inspect the exact BitBake version.",
                    false,
                ),
                RawCommand {
                    id: command("reference.pipeline"),
                    category: category("reference"),
                    label: "Pipeline example".into(),
                    description: "Reference-only pipeline explanation.".into(),
                    reference: RawReference {
                        id: RawReferenceId::new("reference.pipeline.source").unwrap(),
                        heading: "Reference material".into(),
                        command: "bitbake target | tee log".into(),
                        description: "Reference-only pipeline explanation.".into(),
                    },
                    parameters: Vec::new(),
                    execution: RawExecutionPolicy::ReferenceOnly {
                        kind: RawReferenceKind::ShellPipeline,
                        reason: "Shell pipelines are inert reference material.".into(),
                    },
                },
            ],
        }
        .normalize()
        .unwrap()
    }

    fn authority(generation: u64, available: bool) -> DaemonCompatibilitySnapshot {
        let state = if available {
            CapabilityState::Available
        } else {
            CapabilityState::Unavailable {
                reason: CapabilityReason::new(
                    "raw.test.unavailable",
                    "Raw CLI probe is unavailable.",
                    None,
                )
                .unwrap(),
            }
        };
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        "/work/build".into(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: vec![CapabilityRecord {
                    id: CapabilityId::BitBakeRawCli,
                    state,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: if available {
                            CapabilityEvidenceOutcome::Positive
                        } else {
                            CapabilityEvidenceOutcome::Negative
                        },
                        subject: "bitbake --help".into(),
                        detail: "Raw mode reducer fixture.".into(),
                        argv: vec!["bitbake".into(), "--help".into()],
                    }],
                }],
            },
            implementations: if available {
                BTreeMap::from([(
                    CapabilityId::BitBakeRawCli,
                    CapabilityImplementation {
                        id: "bitbake.raw.argv".into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )])
            } else {
                BTreeMap::new()
            },
        }
        .normalize()
        .unwrap()
    }

    fn select_build_category(state: &mut RawModeState, catalog: &RawCatalog) {
        reduce_raw_mode(
            state,
            catalog,
            None,
            RawModeAction::SelectCategory { delta: 1 },
        );
        reduce_raw_mode(state, catalog, None, RawModeAction::FocusCommands);
    }

    #[test]
    fn raw_mode_browsing_search_and_help_follow_exact_stable_selection() {
        let catalog = catalog(1);
        let mut state = RawModeState::new(&catalog);
        assert_eq!(state.category, Some(category("favorites")));
        assert!(state.command.is_none());

        select_build_category(&mut state, &catalog);
        assert_eq!(state.command, Some(command("build.target")));
        assert_eq!(
            state.selected_command(&catalog).unwrap().description,
            "Build one exact target."
        );
        reduce_raw_mode(
            &mut state,
            &catalog,
            None,
            RawModeAction::SelectCommand { delta: 99 },
        );
        assert_eq!(state.command, Some(command("build.version")));

        reduce_raw_mode(&mut state, &catalog, None, RawModeAction::BeginSearch);
        for character in "reference-only".chars() {
            reduce_raw_mode(
                &mut state,
                &catalog,
                None,
                RawModeAction::AppendSearch(character),
            );
        }
        assert_eq!(state.command, Some(command("reference.pipeline")));
        assert_eq!(state.visible_commands(&catalog).len(), 1);
        reduce_raw_mode(&mut state, &catalog, None, RawModeAction::Back);
        assert!(!state.search.editing);
        assert_eq!(state.focus, RawModeFocus::Commands);
    }

    #[test]
    fn raw_mode_form_preview_and_back_restore_exact_typed_state() {
        let catalog = catalog(1);
        let authority = authority(5, true);
        let mut state = RawModeState::new(&catalog);
        select_build_category(&mut state, &catalog);
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::OpenSelected,
        );
        assert_eq!(state.view, RawModeView::Form);
        assert_eq!(state.focus, RawModeFocus::Form);

        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::RequestPreview,
        );
        assert_eq!(state.view, RawModeView::Form);
        assert!(
            state.form.as_ref().unwrap().fields[&parameter("target")]
                .validation_error
                .is_some()
        );

        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::ChooseParameter {
                parameter: parameter("target"),
                value: RawParameterValue::Target("virtual/kernel".into()),
            },
        );
        state
            .form
            .as_mut()
            .unwrap()
            .additional_arguments
            .replace_input("--verbose")
            .unwrap();
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::RequestPreview,
        );
        assert_eq!(state.view, RawModeView::Preview);
        assert_eq!(
            state.preview.as_ref().unwrap().arguments,
            ["virtual/kernel", "--verbose"]
        );

        reduce_raw_mode(&mut state, &catalog, Some(&authority), RawModeAction::Back);
        assert_eq!(state.view, RawModeView::Form);
        assert_eq!(
            state.form.as_ref().unwrap().fields[&parameter("target")].value,
            Some(RawParameterValue::Target("virtual/kernel".into()))
        );
        reduce_raw_mode(&mut state, &catalog, Some(&authority), RawModeAction::Back);
        assert_eq!(state.view, RawModeView::Browser);
        assert_eq!(state.focus, RawModeFocus::Commands);
    }

    #[test]
    fn raw_form_editor_is_typed_bounded_and_invalidates_stale_values() {
        let catalog = catalog(1);
        let authority = authority(5, true);
        let mut state = RawModeState::new(&catalog);
        select_build_category(&mut state, &catalog);
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::OpenSelected,
        );
        let target = parameter("target");
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::EditParameterInput {
                parameter: target.clone(),
                command: PopupEditorCommand::ToggleInsert,
            },
        );
        for character in "busybox".chars() {
            reduce_raw_mode(
                &mut state,
                &catalog,
                Some(&authority),
                RawModeAction::EditParameterInput {
                    parameter: target.clone(),
                    command: PopupEditorCommand::Insert(character),
                },
            );
        }
        let field = &state.form.as_ref().unwrap().fields[&target];
        assert_eq!(field.editor.text, "busybox");
        assert_eq!(
            field.value,
            Some(RawParameterValue::Target("busybox".into()))
        );

        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::EditParameterInput {
                parameter: target.clone(),
                command: PopupEditorCommand::Insert('é'),
            },
        );
        let field = &state.form.as_ref().unwrap().fields[&target];
        assert_eq!(field.editor.text, "busyboxé");
        assert!(field.value.is_none());
        assert!(field.validation_error.is_some());

        for _ in 0..MAX_RAW_TARGET_BYTES {
            reduce_raw_mode(
                &mut state,
                &catalog,
                Some(&authority),
                RawModeAction::EditParameterInput {
                    parameter: target.clone(),
                    command: PopupEditorCommand::Insert('a'),
                },
            );
        }
        assert_eq!(
            state.form.as_ref().unwrap().fields[&target]
                .editor
                .text
                .len(),
            MAX_RAW_TARGET_BYTES
        );
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&authority),
            RawModeAction::SelectFormField { delta: 1 },
        );
        assert!(!state.form.as_ref().unwrap().fields[&target].editor.editing);
        assert_eq!(state.form.as_ref().unwrap().field_selection, 1);
    }

    #[test]
    fn raw_mode_capability_replacement_closes_stale_preview_or_unsafe_form() {
        let catalog = catalog(1);
        let first = authority(5, true);
        let mut state = RawModeState::new(&catalog);
        select_build_category(&mut state, &catalog);
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&first),
            RawModeAction::OpenSelected,
        );
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&first),
            RawModeAction::SetParameterInput {
                parameter: parameter("target"),
                input: "busybox".into(),
            },
        );
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&first),
            RawModeAction::RequestPreview,
        );
        assert_eq!(state.view, RawModeView::Preview);

        let replacement = authority(6, true);
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&replacement),
            RawModeAction::ReprojectAuthority,
        );
        assert_eq!(state.view, RawModeView::Form);
        assert!(state.preview.is_none());
        assert_eq!(state.form.as_ref().unwrap().capability_generation, 6);

        let unavailable = authority(7, false);
        reduce_raw_mode(
            &mut state,
            &catalog,
            Some(&unavailable),
            RawModeAction::ReprojectAuthority,
        );
        assert_eq!(state.view, RawModeView::Browser);
        assert!(state.form.is_none());
        assert!(
            state
                .notification
                .as_deref()
                .is_some_and(|message| message.contains("Raw CLI probe is unavailable"))
        );
    }

    #[test]
    fn raw_mode_history_favorites_and_catalog_replacement_retain_only_exact_ids() {
        let original = catalog(1);
        let mut state = RawModeState::new(&original);
        select_build_category(&mut state, &original);
        reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);
        reduce_raw_mode(
            &mut state,
            &original,
            None,
            RawModeAction::RememberHistory(command("build.target")),
        );
        assert_eq!(state.favorites, [command("build.target")]);
        assert_eq!(state.history, [command("build.target")]);

        reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);
        assert!(state.favorite_confirmation.is_some());
        assert_eq!(state.favorites, [command("build.target")]);
        reduce_raw_mode(&mut state, &original, None, RawModeAction::CancelFavorite);
        assert_eq!(state.favorites, [command("build.target")]);
        reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);
        reduce_raw_mode(&mut state, &original, None, RawModeAction::ConfirmFavorite);
        assert!(state.favorites.is_empty());
        reduce_raw_mode(&mut state, &original, None, RawModeAction::ToggleFavorite);

        reduce_raw_mode(&mut state, &original, None, RawModeAction::OpenFavorites);
        reduce_raw_mode(&mut state, &original, None, RawModeAction::ActivateFavorite);
        assert_eq!(state.command, Some(command("build.target")));
        assert_eq!(state.view, RawModeView::Browser);
        assert_eq!(state.focus, RawModeFocus::Commands);

        let mut replacement = catalog(2);
        replacement
            .commands
            .retain(|item| item.id != command("build.target"));
        replacement = replacement.normalize().unwrap();
        reduce_raw_mode(
            &mut state,
            &replacement,
            None,
            RawModeAction::ReprojectCatalog,
        );
        assert!(state.favorites.is_empty());
        assert!(state.history.is_empty());
        assert_ne!(state.command, Some(command("build.target")));
    }

    #[test]
    fn raw_mode_empty_replacement_and_large_indices_never_panic() {
        let catalog = catalog(1);
        let mut state = RawModeState::new(&catalog);
        let empty = RawCatalog {
            version: 2,
            categories: Vec::new(),
            commands: Vec::new(),
        };
        reduce_raw_mode(&mut state, &empty, None, RawModeAction::ReprojectCatalog);
        reduce_raw_mode(
            &mut state,
            &empty,
            None,
            RawModeAction::SelectCategory { delta: isize::MAX },
        );
        reduce_raw_mode(
            &mut state,
            &empty,
            None,
            RawModeAction::SelectCommand { delta: isize::MIN },
        );
        assert!(state.category.is_none());
        assert!(state.command.is_none());
        assert_eq!(state.history_selection, 0);
        assert_eq!(state.favorite_selection, 0);
    }

    #[test]
    fn raw_category_browser_pins_favorites_before_reference_order() {
        let catalog = builtin_raw_catalog();
        let categories = catalog.browser_categories();
        assert_eq!(categories.len(), crate::RAW_BUILTIN_CATEGORY_COUNT);
        assert_eq!(categories[0].kind, RawCategoryKind::Favorites);
        assert_eq!(categories[0].label, "Favorites");
        assert_eq!(categories[1].reference_heading, "1. Version and help");
        assert_eq!(
            categories.last().unwrap().reference_heading,
            "One-screen emergency reference"
        );

        let mut state = RawModeState::new(catalog);
        assert_eq!(state.category, Some(categories[0].id.clone()));
        reduce_raw_mode(
            &mut state,
            catalog,
            None,
            RawModeAction::SelectCategory { delta: 1 },
        );
        assert_eq!(state.category, Some(categories[1].id.clone()));
        reduce_raw_mode(
            &mut state,
            catalog,
            None,
            RawModeAction::SelectCategory { delta: isize::MAX },
        );
        assert_eq!(state.category, Some(categories.last().unwrap().id.clone()));
    }
}

#[cfg(test)]
mod raw_preview_tests {
    use super::*;
    use crate::{
        AuthoritativeValue, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityReason, CapabilityRecord,
        CapabilitySnapshot, IdentityAuthority, YoctoEnvironmentIdentity,
    };

    fn id(value: &str) -> RawParameterId {
        RawParameterId::new(value).unwrap()
    }

    pub(super) fn catalog() -> RawCatalog {
        RawCatalog {
            version: 7,
            categories: vec![RawCategory {
                id: RawCategoryId::new("preview").unwrap(),
                label: "Preview".into(),
                reference_heading: "Preview".into(),
                kind: RawCategoryKind::Executable,
            }],
            commands: vec![RawCommand {
                id: RawCommandId::new("preview.run").unwrap(),
                category: RawCategoryId::new("preview").unwrap(),
                label: "bitbake -c <task> --ui=<ui> mc:<config>:<target> ''".into(),
                description: "Preview fixture command.".into(),
                reference: RawReference {
                    id: RawReferenceId::new("preview.reference").unwrap(),
                    heading: "Preview".into(),
                    command: "bitbake -c <task> --ui=<ui> mc:<config>:<target> ''".into(),
                    description: "Preview fixture command.".into(),
                },
                parameters: vec![
                    RawParameter {
                        id: id("task"),
                        label: "Task".into(),
                        placeholder: "<task>".into(),
                        kind: RawParameterKind::Task,
                        presence: RawParameterPresence::Required,
                    },
                    RawParameter {
                        id: id("ui"),
                        label: "UI".into(),
                        placeholder: "<ui>".into(),
                        kind: RawParameterKind::UserInterface,
                        presence: RawParameterPresence::Optional,
                    },
                    RawParameter {
                        id: id("config"),
                        label: "Config".into(),
                        placeholder: "<config>".into(),
                        kind: RawParameterKind::Multiconfig,
                        presence: RawParameterPresence::Required,
                    },
                    RawParameter {
                        id: id("target"),
                        label: "Target".into(),
                        placeholder: "<target>".into(),
                        kind: RawParameterKind::Target,
                        presence: RawParameterPresence::Required,
                    },
                ],
                execution: RawExecutionPolicy::Executable {
                    template: RawExecutableTemplate {
                        executable: RawExecutable::BitBake,
                        arguments: vec![
                            RawArgument::Literal { value: "-c".into() },
                            RawArgument::Parameter {
                                parameter: id("task"),
                            },
                            RawArgument::JoinedParameter {
                                prefix: "--ui=".into(),
                                parameter: id("ui"),
                            },
                            RawArgument::Composed {
                                segments: vec![
                                    RawArgumentSegment::Literal {
                                        value: "mc:".into(),
                                    },
                                    RawArgumentSegment::Parameter {
                                        parameter: id("config"),
                                    },
                                    RawArgumentSegment::Literal { value: ":".into() },
                                    RawArgumentSegment::Parameter {
                                        parameter: id("target"),
                                    },
                                ],
                            },
                            RawArgument::Empty,
                        ],
                        capabilities: RawCapabilityRequirement::All {
                            capabilities: vec![CapabilityId::BitBakeRawCli],
                        },
                        interaction: RawInteractionMode::NoninteractiveJob,
                        safety: RawSafetyClass::Build,
                    },
                },
            }],
        }
        .normalize()
        .unwrap()
    }

    pub(super) fn authority(generation: u64, available: bool) -> DaemonCompatibilitySnapshot {
        let state = if available {
            CapabilityState::AvailableWithLimitations {
                reason: CapabilityReason::new(
                    "preview.limited",
                    "Preview fixture is limited.",
                    None,
                )
                .unwrap(),
                limitations: vec!["Exact fixture limitation.".into()],
            }
        } else {
            CapabilityState::Unavailable {
                reason: CapabilityReason::new(
                    "preview.unavailable",
                    "Preview fixture is unavailable.",
                    None,
                )
                .unwrap(),
            }
        };
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation,
                environment: YoctoEnvironmentIdentity {
                    build_directory: AuthoritativeValue::detected(
                        "/work/build".into(),
                        IdentityAuthority::InitializedEnvironment,
                    ),
                    ..YoctoEnvironmentIdentity::default()
                },
                capabilities: vec![CapabilityRecord {
                    id: CapabilityId::BitBakeRawCli,
                    state,
                    evidence: vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: if available {
                            CapabilityEvidenceOutcome::Positive
                        } else {
                            CapabilityEvidenceOutcome::Negative
                        },
                        subject: "bitbake --help".into(),
                        detail: "Exact fixture evidence.".into(),
                        argv: vec!["bitbake".into(), "--help".into()],
                    }],
                }],
            },
            implementations: if available {
                BTreeMap::from([(
                    CapabilityId::BitBakeRawCli,
                    CapabilityImplementation {
                        id: "bitbake.raw.argv".into(),
                        kind: CapabilityImplementationKind::Command,
                    },
                )])
            } else {
                BTreeMap::new()
            },
        }
        .normalize()
        .unwrap()
    }

    pub(super) fn request() -> RawPreviewRequest {
        RawPreviewRequest {
            catalog_version: 7,
            command: RawCommandId::new("preview.run").unwrap(),
            parameters: BTreeMap::from([
                (id("task"), RawParameterValue::Task("do_compile".into())),
                (id("config"), RawParameterValue::Multiconfig("lib32".into())),
                (
                    id("target"),
                    RawParameterValue::Target("virtual/kernel".into()),
                ),
            ]),
            additional_arguments: RawAdditionalArguments::parse("--verbose 'café value'").unwrap(),
            capability_generation: 9,
            build_directory: "/work/build".into(),
        }
    }

    #[test]
    fn raw_preview_reconstructs_exact_indexed_native_arguments_and_metadata() {
        let preview = catalog()
            .preview(&request(), Some(&authority(9, true)))
            .unwrap();
        assert_eq!(preview.executable, RawExecutable::BitBake);
        assert_eq!(
            preview.arguments,
            [
                "-c",
                "do_compile",
                "mc:lib32:virtual/kernel",
                "",
                "--verbose",
                "café value",
            ]
        );
        assert_eq!(
            preview
                .indexed_arguments
                .iter()
                .map(|argument| (argument.index, argument.value.as_str()))
                .collect::<Vec<_>>(),
            [
                (0, "bitbake"),
                (1, "-c"),
                (2, "do_compile"),
                (3, "mc:lib32:virtual/kernel"),
                (4, ""),
                (5, "--verbose"),
                (6, "café value"),
            ]
        );
        assert_eq!(preview.catalog_version, 7);
        assert_eq!(preview.capability_generation, 9);
        assert_eq!(preview.build_directory, Path::new("/work/build"));
        assert_eq!(preview.interaction, RawInteractionMode::NoninteractiveJob);
        assert_eq!(preview.safety, RawSafetyClass::Build);
        assert_eq!(preview.limitations, ["Exact fixture limitation."]);
        assert_eq!(preview.capability_issues.len(), 1);
        assert_eq!(
            preview.implementations,
            [(CapabilityId::BitBakeRawCli, "bitbake.raw.argv".into())]
        );
    }

    #[test]
    fn raw_preview_includes_optional_joined_value_and_preserves_explicit_empty() {
        let mut request = request();
        request
            .parameters
            .insert(id("ui"), RawParameterValue::UserInterface("knotty".into()));
        let preview = catalog()
            .preview(&request, Some(&authority(9, true)))
            .unwrap();
        assert_eq!(preview.arguments[2], "--ui=knotty");
        assert!(preview.arguments.iter().any(String::is_empty));
    }

    #[test]
    fn raw_preview_rejects_missing_extra_and_mismatched_parameters() {
        let catalog = catalog();
        let authority = authority(9, true);

        let mut missing = request();
        missing.parameters.remove(&id("task"));
        assert_eq!(
            catalog.preview(&missing, Some(&authority)),
            Err(RawPreviewError::MissingParameter(id("task")))
        );

        let mut extra = request();
        extra
            .parameters
            .insert(id("other"), RawParameterValue::Text("ordinary".into()));
        assert_eq!(
            catalog.preview(&extra, Some(&authority)),
            Err(RawPreviewError::UnknownParameter(id("other")))
        );

        let mut mismatch = request();
        mismatch
            .parameters
            .insert(id("task"), RawParameterValue::Target("busybox".into()));
        assert!(matches!(
            catalog.preview(&mismatch, Some(&authority)),
            Err(RawPreviewError::InvalidParameter(
                RawParameterError::KindMismatch { .. }
            ))
        ));
    }

    #[test]
    fn raw_preview_fails_closed_for_missing_stale_or_unavailable_authority() {
        let catalog = catalog();
        let request = request();
        assert_eq!(
            catalog.preview(&request, None),
            Err(RawPreviewError::MissingAuthority)
        );
        assert_eq!(
            catalog.preview(&request, Some(&authority(10, true))),
            Err(RawPreviewError::StaleCapabilityGeneration {
                current: 10,
                received: 9,
            })
        );
        assert!(matches!(
            catalog.preview(&request, Some(&authority(9, false))),
            Err(RawPreviewError::CapabilityUnavailable {
                state: RawAvailabilityState::Unavailable,
                ..
            })
        ));

        let mut stale_directory = request;
        stale_directory.build_directory = "/other/build".into();
        assert!(matches!(
            catalog.preview(&stale_directory, Some(&authority(9, true))),
            Err(RawPreviewError::StaleBuildDirectory { .. })
        ));
    }
}

#[cfg(test)]
mod raw_argv_tests {
    use super::*;

    fn parse(input: &str) -> Result<Vec<String>, RawArgvError> {
        RawAdditionalArguments::parse(input).map(RawAdditionalArguments::into_vec)
    }

    #[test]
    fn raw_argv_tokenizes_quotes_escapes_empty_elements_and_unicode_as_native_arguments() {
        assert_eq!(
            parse("--flag plain 'single value' \"double value\" escaped\\ space '' \"\" café")
                .unwrap(),
            [
                "--flag",
                "plain",
                "single value",
                "double value",
                "escaped space",
                "",
                "",
                "café",
            ]
        );
        assert_eq!(parse("").unwrap(), Vec::<String>::new());
        assert_eq!(
            parse(r#"a\b \"quoted\" slash\\value"#).unwrap(),
            ["ab", "\"quoted\"", "slash\\value",]
        );
    }

    #[test]
    fn raw_argv_rejects_every_documented_operator_even_when_quoted_or_assembled() {
        for (input, operator) in [
            ("'left|right'", "|"),
            ("left>right", ">"),
            ("left>>right", ">>"),
            ("left<right", "<"),
            ("left&&right", "&&"),
            ("left||right", "||"),
            ("left;right", ";"),
            ("'$('date')'", "$("),
            ("`date`", "`"),
            ("'$''('", "$("),
        ] {
            assert_eq!(
                parse(input),
                Err(RawArgvError::ForbiddenOperator {
                    argument: 0,
                    operator: operator.into(),
                }),
                "{input}"
            );
        }
        assert!(matches!(
            parse("left\\|right"),
            Err(RawArgvError::InvalidEscape { character: '|', .. })
        ));
        assert_eq!(
            parse("literal&value $HOME (literal)").unwrap(),
            ["literal&value", "$HOME", "(literal)"]
        );
    }

    #[test]
    fn raw_argv_reports_controls_unterminated_grammar_and_empty_option_names() {
        for input in ["line\nbreak", "tab\tvalue", "nul\0value"] {
            assert!(matches!(
                parse(input),
                Err(RawArgvError::ControlCharacter { .. })
            ));
        }
        assert!(matches!(
            parse("'open"),
            Err(RawArgvError::UnterminatedQuote { quote: '\'', .. })
        ));
        assert!(matches!(
            parse("open\\"),
            Err(RawArgvError::UnterminatedEscape { .. })
        ));
        for input in ["--=value", "-=value"] {
            assert_eq!(
                parse(input),
                Err(RawArgvError::EmptyOptionName { argument: 0 })
            );
        }
        assert_eq!(parse("-- -").unwrap(), ["--", "-"]);
    }

    #[test]
    fn raw_argv_enforces_input_element_count_and_aggregate_byte_boundaries() {
        assert!(parse(&" ".repeat(MAX_RAW_ADDITIONAL_INPUT_BYTES)).is_ok());
        assert_eq!(
            parse(&" ".repeat(MAX_RAW_ADDITIONAL_INPUT_BYTES + 1)),
            Err(RawArgvError::InputTooLong {
                bytes: MAX_RAW_ADDITIONAL_INPUT_BYTES + 1,
                maximum: MAX_RAW_ADDITIONAL_INPUT_BYTES,
            })
        );

        assert!(parse(&"a".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES)).is_ok());
        assert!(matches!(
            parse(&"a".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES + 1)),
            Err(RawArgvError::ArgumentTooLong { .. })
        ));
        let unicode = "é".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES / 2);
        assert_eq!(unicode.len(), MAX_RAW_ADDITIONAL_ARGUMENT_BYTES);
        assert!(parse(&unicode).is_ok());
        assert!(matches!(
            parse(&(unicode + "é")),
            Err(RawArgvError::ArgumentTooLong { .. })
        ));

        let maximum_count = std::iter::repeat_n("x", MAX_RAW_ADDITIONAL_ARGUMENTS)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            parse(&maximum_count).unwrap().len(),
            MAX_RAW_ADDITIONAL_ARGUMENTS
        );
        assert!(matches!(
            parse(&(maximum_count + " x")),
            Err(RawArgvError::TooManyArguments { .. })
        ));

        let aggregate = std::iter::repeat_n(
            "a".repeat(MAX_RAW_ADDITIONAL_ARGUMENT_BYTES),
            MAX_RAW_ADDITIONAL_AGGREGATE_BYTES / MAX_RAW_ADDITIONAL_ARGUMENT_BYTES,
        )
        .collect::<Vec<_>>()
        .join(" ");
        assert!(parse(&aggregate).is_ok());
        assert!(matches!(
            parse(&(aggregate + " x")),
            Err(RawArgvError::AggregateTooLong { .. })
        ));
    }

    #[test]
    fn raw_argv_editor_invalidates_stale_validation_and_replaces_failures() {
        let mut editor = RawArgvEditor::new("--flag value").unwrap();
        assert_eq!(editor.validate().unwrap().as_slice(), ["--flag", "value"]);
        editor.replace_input("'unterminated").unwrap();
        assert!(editor.validate().is_err());
        assert!(editor.validated.is_none());
        assert!(editor.validation_error.is_some());

        editor.replace_input("--next 'two words'").unwrap();
        assert!(editor.validation_error.is_none());
        assert_eq!(
            editor.validate().unwrap().as_slice(),
            ["--next", "two words"]
        );

        editor.apply(PopupEditorCommand::ToggleInsert).unwrap();
        editor.apply(PopupEditorCommand::Insert('x')).unwrap();
        assert!(editor.validated.is_none());
    }
}

#[cfg(test)]
mod raw_selector_tests {
    use super::*;

    fn selector_command(kind: RawParameterKind) -> (RawCommand, RawParameterId) {
        let command = RawCatalog::builtin()
            .commands
            .into_iter()
            .find(|command| {
                matches!(command.execution, RawExecutionPolicy::Executable { .. })
                    && command
                        .parameters
                        .iter()
                        .any(|parameter| parameter.kind == kind)
            })
            .unwrap();
        let parameter = command
            .parameters
            .iter()
            .find(|parameter| parameter.kind == kind)
            .unwrap()
            .id
            .clone();
        (command, parameter)
    }

    #[test]
    fn raw_selector_distinguishes_absent_and_authoritative_empty_inventories() {
        let absent = RawSelectorAuthority::project(RawSelectorSources::default());
        assert!(matches!(
            absent.recipes,
            RawSelectorInventory::Unavailable { .. }
        ));
        assert!(matches!(
            absent.targets,
            RawSelectorInventory::Unavailable { .. }
        ));

        let recipes = Vec::new();
        let images = Vec::new();
        let recent_targets = Vec::new();
        let metadata = RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(Vec::new()),
            ..RecipeMetadata::default()
        };
        let empty = RawSelectorAuthority::project(RawSelectorSources {
            recipes: Some(&recipes),
            images: Some(&images),
            recent_targets: Some(&recent_targets),
            selected_recipe: Some("busybox"),
            recipe_metadata: Some(&metadata),
            multiconfig: Some(""),
            ..RawSelectorSources::default()
        });
        for inventory in [
            &empty.recipes,
            &empty.images,
            &empty.targets,
            &empty.tasks,
            &empty.multiconfigs,
        ] {
            assert_eq!(inventory.choices(), Some([].as_slice()));
        }
    }

    #[test]
    fn raw_selector_retains_exact_recipe_and_typed_inventory_identities() {
        let recipes = vec![
            Recipe {
                name: "busybox".into(),
                file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "busybox".into(),
                file: Some("/workspace/recipes/busybox.bb".into()),
                ..Recipe::default()
            },
        ];
        let images = vec![
            "core-image-minimal".into(),
            "core-image-full-cmdline".into(),
        ];
        let recent_targets = vec!["busybox".into(), "virtual/kernel".into(), "busybox".into()];
        let authority = RawSelectorAuthority::project(RawSelectorSources {
            recipes: Some(&recipes),
            images: Some(&images),
            current_target: Some("core-image-minimal"),
            recent_targets: Some(&recent_targets),
            multiconfig: Some("lib32 board1 lib32"),
            ..RawSelectorSources::default()
        });

        assert_eq!(authority.recipes.choices().unwrap().len(), 2);
        assert_ne!(
            authority.recipes.choices().unwrap()[0].identity,
            authority.recipes.choices().unwrap()[1].identity
        );
        assert_eq!(authority.images.choices().unwrap().len(), 2);
        assert_eq!(authority.targets.choices().unwrap().len(), 3);
        assert_eq!(authority.multiconfigs.choices().unwrap().len(), 2);
        assert_eq!(
            authority.targets.choices().unwrap()[1].value,
            RawParameterValue::Target("busybox".into())
        );
    }

    #[test]
    fn raw_selector_correlates_tasks_to_the_exact_recipe_and_replaces_results() {
        let alpha = RecipeMetadata {
            recipe: "alpha".into(),
            tasks: Some(vec!["do_build".into(), "do_compile".into()]),
            ..RecipeMetadata::default()
        };
        let current = RawSelectorAuthority::project(RawSelectorSources {
            selected_recipe: Some("alpha"),
            recipe_metadata: Some(&alpha),
            ..RawSelectorSources::default()
        });
        assert_eq!(current.tasks.choices().unwrap().len(), 2);

        let stale = RawSelectorAuthority::project(RawSelectorSources {
            selected_recipe: Some("beta"),
            recipe_metadata: Some(&alpha),
            ..RawSelectorSources::default()
        });
        assert!(matches!(
            stale.tasks,
            RawSelectorInventory::Unavailable { ref reason }
                if reason.contains("cannot populate")
        ));

        let beta = RecipeMetadata {
            recipe: "beta".into(),
            tasks: Some(vec!["do_install".into()]),
            ..RecipeMetadata::default()
        };
        let replacement = RawSelectorAuthority::project(RawSelectorSources {
            selected_recipe: Some("beta"),
            recipe_metadata: Some(&beta),
            ..RawSelectorSources::default()
        });
        assert_eq!(
            replacement.tasks.choices().unwrap()[0].identity,
            RawSelectorIdentity::Task {
                recipe: "beta".into(),
                task: "do_install".into()
            }
        );
    }

    #[test]
    fn raw_selector_manual_target_and_task_entry_uses_parameter_validation() {
        let authority = RawSelectorAuthority::project(RawSelectorSources::default());
        for kind in [RawParameterKind::Target, RawParameterKind::Task] {
            let (command, parameter) = selector_command(kind);
            let selector = command.selector(&parameter, &authority).unwrap();
            assert!(selector.manual_entry);
            let valid = if kind == RawParameterKind::Target {
                "virtual/kernel"
            } else {
                "do_compile"
            };
            assert!(selector.parse_manual(valid).is_ok());
            assert!(selector.parse_manual("--option").is_err());
            assert!(selector.parse_manual("bad;command").is_err());
        }
    }
}

#[cfg(test)]
mod raw_parameter_tests {
    use super::*;

    fn parameter(kind: RawParameterKind, presence: RawParameterPresence) -> RawParameter {
        RawParameter {
            id: RawParameterId::new("value").unwrap(),
            label: "Value".into(),
            placeholder: "<value>".into(),
            kind,
            presence,
        }
    }

    fn parsed(kind: RawParameterKind, input: &str) -> RawParameterValue {
        parameter(kind, RawParameterPresence::Required)
            .parse_value(input)
            .unwrap()
            .unwrap()
    }

    #[test]
    fn raw_parameter_accepts_every_typed_kind_as_one_argument() {
        let fixtures = [
            (
                RawParameterKind::Recipe,
                "busybox",
                RawParameterValue::Recipe("busybox".into()),
            ),
            (
                RawParameterKind::Image,
                "core-image-minimal",
                RawParameterValue::Image("core-image-minimal".into()),
            ),
            (
                RawParameterKind::Target,
                "virtual/kernel",
                RawParameterValue::Target("virtual/kernel".into()),
            ),
            (
                RawParameterKind::Task,
                "do_compile",
                RawParameterValue::Task("do_compile".into()),
            ),
            (
                RawParameterKind::UserInterface,
                "knotty",
                RawParameterValue::UserInterface("knotty".into()),
            ),
            (
                RawParameterKind::File,
                "/tmp/Raw Mode/recipe.bb",
                RawParameterValue::File("/tmp/Raw Mode/recipe.bb".into()),
            ),
            (
                RawParameterKind::Integer,
                "4294967295",
                RawParameterValue::Integer(MAX_RAW_INTEGER),
            ),
            (
                RawParameterKind::Text,
                "例え:値/=+",
                RawParameterValue::Text("例え:値/=+".into()),
            ),
            (
                RawParameterKind::Multiconfig,
                "lib32",
                RawParameterValue::Multiconfig("lib32".into()),
            ),
        ];

        for (kind, input, expected) in fixtures {
            let value = parsed(kind, input);
            assert_eq!(value, expected);
            assert_eq!(value.argument(), input);
        }
    }

    #[test]
    fn raw_parameter_required_and_optional_empty_inputs_remain_distinct() {
        let required = parameter(RawParameterKind::Target, RawParameterPresence::Required);
        assert_eq!(
            required.parse_value(""),
            Err(RawParameterError::Required {
                parameter: required.id.clone(),
            })
        );

        let optional = parameter(RawParameterKind::Target, RawParameterPresence::Optional);
        assert_eq!(optional.parse_value(""), Ok(None));
        assert_eq!(
            optional.parse_value("busybox").unwrap(),
            Some(RawParameterValue::Target("busybox".into()))
        );
        assert!(optional.parse_value(" ").is_err());
    }

    #[test]
    fn raw_parameter_enforces_identifier_and_unicode_byte_boundaries() {
        for (kind, maximum) in [
            (RawParameterKind::Recipe, MAX_RAW_RECIPE_BYTES),
            (RawParameterKind::Image, MAX_RAW_IMAGE_BYTES),
            (RawParameterKind::Target, MAX_RAW_TARGET_BYTES),
            (RawParameterKind::Task, MAX_RAW_TASK_BYTES),
            (RawParameterKind::UserInterface, MAX_RAW_UI_BYTES),
            (RawParameterKind::Multiconfig, MAX_RAW_MULTICONFIG_BYTES),
        ] {
            assert!(
                parameter(kind, RawParameterPresence::Required)
                    .parse_value(&"a".repeat(maximum))
                    .is_ok()
            );
            assert!(
                parameter(kind, RawParameterPresence::Required)
                    .parse_value(&"a".repeat(maximum + 1))
                    .is_err()
            );
        }

        let text = parameter(RawParameterKind::Text, RawParameterPresence::Required);
        let exact_unicode = "é".repeat(MAX_RAW_PARAMETER_TEXT_BYTES / 2);
        assert_eq!(exact_unicode.len(), MAX_RAW_PARAMETER_TEXT_BYTES);
        assert!(text.parse_value(&exact_unicode).is_ok());
        assert!(text.parse_value(&(exact_unicode + "é")).is_err());
        assert!(
            parameter(RawParameterKind::Recipe, RawParameterPresence::Required)
                .parse_value("récipe")
                .is_err()
        );

        let file = parameter(RawParameterKind::File, RawParameterPresence::Required);
        assert!(file.parse_value("レイヤ/recipe.bb").is_ok());
        assert!(
            file.parse_value(&format!("/{}", "a".repeat(MAX_RAW_FILE_BYTES - 1)))
                .is_ok()
        );
        assert!(
            file.parse_value(&format!("/{}", "a".repeat(MAX_RAW_FILE_BYTES)))
                .is_err()
        );
    }

    #[test]
    fn raw_parameter_rejects_shell_syntax_traversal_and_invalid_numbers() {
        for kind in [
            RawParameterKind::Recipe,
            RawParameterKind::Image,
            RawParameterKind::Target,
            RawParameterKind::Task,
            RawParameterKind::UserInterface,
            RawParameterKind::Text,
            RawParameterKind::Multiconfig,
        ] {
            let parameter = parameter(kind, RawParameterPresence::Required);
            assert!(parameter.parse_value("--option").is_err());
            assert!(parameter.parse_value("value;other").is_err());
            assert!(parameter.parse_value("value\nother").is_err());
        }

        let file = parameter(RawParameterKind::File, RawParameterPresence::Required);
        for invalid in [
            "../recipe.bb",
            "layers/../recipe.bb",
            "./recipe.bb",
            "events$(date).json",
            "-events.json",
            "events.json/",
        ] {
            assert!(file.parse_value(invalid).is_err(), "{invalid}");
        }

        let integer = parameter(RawParameterKind::Integer, RawParameterPresence::Required);
        assert!(integer.parse_value("-1").is_err());
        assert!(integer.parse_value("1.5").is_err());
        assert_eq!(
            integer.parse_value("4294967296"),
            Err(integer.invalid(RawParameterInvalidReason::IntegerOutOfRange))
        );
    }

    #[test]
    fn raw_parameter_rejects_typed_kind_definition_disagreement() {
        let recipe = parameter(RawParameterKind::Recipe, RawParameterPresence::Required);
        assert_eq!(
            recipe.validate_value(&RawParameterValue::Target("busybox".into())),
            Err(RawParameterError::KindMismatch {
                parameter: recipe.id.clone(),
                expected: RawParameterKind::Recipe,
                actual: RawParameterKind::Target,
            })
        );
    }
}

#[cfg(test)]
mod raw_catalog_model_tests {
    use super::*;

    fn id<T>(value: &str) -> T
    where
        T: TryFrom<&'static str>,
        <T as TryFrom<&'static str>>::Error: std::fmt::Debug,
    {
        T::try_from(Box::leak(value.to_owned().into_boxed_str())).unwrap()
    }

    fn valid_catalog() -> RawCatalog {
        RawCatalog {
            version: RAW_CATALOG_VERSION,
            categories: vec![RawCategory {
                id: id("task-control"),
                label: "Task control".into(),
                reference_heading: "Recipe Task Execution".into(),
                kind: RawCategoryKind::Executable,
            }],
            commands: vec![RawCommand {
                id: id("task-control.run"),
                category: id("task-control"),
                label: "Run a recipe task".into(),
                description: "Execute one named task for a recipe.".into(),
                reference: RawReference {
                    id: id("recipe-task-execution.run-task"),
                    heading: "Recipe Task Execution".into(),
                    command: "bitbake -c <task> <recipe>".into(),
                    description: "Execute one named task for a recipe.".into(),
                },
                parameters: vec![
                    RawParameter {
                        id: id("task"),
                        label: "Task".into(),
                        placeholder: "<task>".into(),
                        kind: RawParameterKind::Task,
                        presence: RawParameterPresence::Required,
                    },
                    RawParameter {
                        id: id("recipe"),
                        label: "Recipe".into(),
                        placeholder: "<recipe>".into(),
                        kind: RawParameterKind::Recipe,
                        presence: RawParameterPresence::Required,
                    },
                ],
                execution: RawExecutionPolicy::Executable {
                    template: RawExecutableTemplate {
                        executable: RawExecutable::BitBake,
                        arguments: vec![
                            RawArgument::Literal { value: "-c".into() },
                            RawArgument::Parameter {
                                parameter: id("task"),
                            },
                            RawArgument::Parameter {
                                parameter: id("recipe"),
                            },
                        ],
                        capabilities: RawCapabilityRequirement::All {
                            capabilities: vec![CapabilityId::BitBakeBuild],
                        },
                        interaction: RawInteractionMode::NoninteractiveJob,
                        safety: RawSafetyClass::Build,
                    },
                },
            }],
        }
    }

    #[test]
    fn raw_catalog_model_accepts_valid_bounded_typed_catalog() {
        let catalog = valid_catalog().normalize().unwrap();
        assert_eq!(catalog.categories[0].id.as_str(), "task-control");
        let RawExecutionPolicy::Executable { template } = &catalog.commands[0].execution else {
            panic!("fixture must be executable");
        };
        assert_eq!(
            template.display_template(&catalog.commands[0].parameters),
            Some("bitbake -c <task> <recipe>".into())
        );
    }

    #[test]
    fn raw_catalog_model_rejects_partial_records() {
        let mut catalog = valid_catalog();
        catalog.commands[0].description.clear();
        assert_eq!(
            catalog.validate(),
            Err(RawCatalogError::InvalidCommand(id("task-control.run")))
        );

        let mut reference_only = valid_catalog();
        reference_only.commands[0].execution = RawExecutionPolicy::ReferenceOnly {
            kind: RawReferenceKind::Conceptual,
            reason: "Documented concept only.".into(),
        };
        assert_eq!(
            reference_only.validate(),
            Err(RawCatalogError::InvalidExecutionPolicy(id(
                "task-control.run"
            )))
        );
    }

    #[test]
    fn raw_catalog_model_rejects_duplicate_identities() {
        let mut duplicate_command = valid_catalog();
        duplicate_command
            .commands
            .push(duplicate_command.commands[0].clone());
        assert_eq!(
            duplicate_command.validate(),
            Err(RawCatalogError::DuplicateCommand(id("task-control.run")))
        );

        let mut duplicate_parameter = valid_catalog();
        let repeated = duplicate_parameter.commands[0].parameters[0].clone();
        duplicate_parameter.commands[0].parameters.push(repeated);
        assert_eq!(
            duplicate_parameter.validate(),
            Err(RawCatalogError::InvalidParameters(id("task-control.run")))
        );
    }

    #[test]
    fn raw_catalog_model_rejects_oversized_records() {
        let mut catalog = valid_catalog();
        catalog.commands[0].description = "x".repeat(MAX_RAW_TEXT_BYTES + 1);
        assert_eq!(
            catalog.validate(),
            Err(RawCatalogError::InvalidCommand(id("task-control.run")))
        );

        let invalid = RawCommandId::new("x".repeat(MAX_RAW_ID_BYTES + 1));
        assert!(matches!(
            invalid,
            Err(RawCatalogError::InvalidIdentity {
                field: "command",
                ..
            })
        ));
    }

    #[test]
    fn raw_catalog_model_rejects_unsafe_or_disagreeing_templates() {
        let mut unsafe_catalog = valid_catalog();
        let RawExecutionPolicy::Executable { template } = &mut unsafe_catalog.commands[0].execution
        else {
            unreachable!()
        };
        template.arguments[0] = RawArgument::Literal {
            value: ";rm".into(),
        };
        assert_eq!(
            unsafe_catalog.validate(),
            Err(RawCatalogError::UnsafeTemplate(id("task-control.run")))
        );

        let mut disagreement = valid_catalog();
        disagreement.commands[0].parameters.pop();
        assert_eq!(
            disagreement.validate(),
            Err(RawCatalogError::PlaceholderDisagreement(id(
                "task-control.run"
            )))
        );
    }

    #[test]
    fn raw_catalog_model_rejects_missing_or_duplicate_capability_policy() {
        let mut catalog = valid_catalog();
        let RawExecutionPolicy::Executable { template } = &mut catalog.commands[0].execution else {
            unreachable!()
        };
        template.capabilities = RawCapabilityRequirement::All {
            capabilities: Vec::new(),
        };
        assert_eq!(
            catalog.validate(),
            Err(RawCatalogError::InvalidCapabilityRequirement(id(
                "task-control.run"
            )))
        );
    }
}

#[cfg(test)]
mod raw_catalog_trace_tests {
    use super::*;
    use crate::{
        RAW_BUILTIN_CATEGORY_COUNT, RAW_BUILTIN_COMMAND_COUNT, RAW_BUILTIN_EXECUTABLE_COUNT,
        RAW_REFERENCE_SHA256,
    };
    use std::collections::BTreeSet;

    const REFERENCE: &str =
        include_str!("../../../docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md");

    #[derive(Debug)]
    struct ReferenceEntry<'a> {
        line: usize,
        category_heading: &'a str,
        heading: &'a str,
        description: &'a str,
        command: &'a str,
    }

    fn reference_entries() -> (Vec<&'static str>, Vec<ReferenceEntry<'static>>) {
        let mut categories = Vec::new();
        let mut entries = Vec::new();
        let mut category_heading = "";
        let mut heading = "";
        let mut description = "";
        let mut in_bash = false;

        for (index, line) in REFERENCE.lines().enumerate() {
            let line_number = index + 1;
            if line == "```bash" {
                in_bash = true;
                description = "";
                continue;
            }
            if in_bash && line == "```" {
                in_bash = false;
                description = "";
                continue;
            }
            if !in_bash && line.starts_with("# ") {
                if line_number != 1 {
                    category_heading = &line[2..];
                    heading = category_heading;
                    categories.push(category_heading);
                }
                continue;
            }
            if !in_bash && line.starts_with("##") {
                heading = line.trim_start_matches('#').trim();
                continue;
            }
            if !in_bash {
                continue;
            }
            if let Some(comment) = line.strip_prefix("# ") {
                description = comment;
                continue;
            }
            if line.is_empty() {
                continue;
            }
            assert!(!category_heading.is_empty(), "line {line_number}");
            assert!(!description.is_empty(), "line {line_number}");
            entries.push(ReferenceEntry {
                line: line_number,
                category_heading,
                heading,
                description,
                command: line,
            });
            description = "";
        }
        (categories, entries)
    }

    fn direct_bitbake(command: &str) -> bool {
        command.starts_with("bitbake ")
            && ![" | ", " > ", " && ", " || ", "; "]
                .iter()
                .any(|operator| command.contains(operator))
    }

    #[test]
    fn raw_catalog_trace_covers_every_reference_command_exactly_once() {
        let (category_headings, reference_entries) = reference_entries();
        let catalog = RawCatalog::builtin();
        assert_eq!(reference_entries.len(), RAW_BUILTIN_COMMAND_COUNT);

        let mut seen_commands = BTreeSet::new();
        for reference in reference_entries {
            let reference_id = format!("wrynose-6-0.l{:04}", reference.line);
            let matches = catalog
                .commands
                .iter()
                .filter(|command| command.reference.id.as_str() == reference_id)
                .collect::<Vec<_>>();
            assert_eq!(matches.len(), 1, "reference line {}", reference.line);
            let command = matches[0];
            assert!(seen_commands.insert(command.id.clone()));
            assert_eq!(command.label, reference.command);
            assert_eq!(command.description, reference.description);
            assert_eq!(command.reference.heading, reference.heading);
            assert_eq!(command.reference.command, reference.command);
            assert_eq!(command.reference.description, reference.description);
            assert_eq!(
                catalog
                    .category(&command.category)
                    .unwrap()
                    .reference_heading,
                reference.category_heading
            );

            match &command.execution {
                RawExecutionPolicy::Executable { template } => {
                    assert!(direct_bitbake(reference.command), "{}", reference.command);
                    assert_eq!(
                        template.display_template(&command.parameters).as_deref(),
                        Some(reference.command)
                    );
                }
                RawExecutionPolicy::ReferenceOnly { kind, .. } => {
                    assert!(!direct_bitbake(reference.command), "{}", reference.command);
                    let expected = if reference.command.starts_with("bitbake ") {
                        RawReferenceKind::ShellPipeline
                    } else {
                        RawReferenceKind::CompanionTool
                    };
                    assert_eq!(*kind, expected, "{}", reference.command);
                    assert!(command.parameters.is_empty());
                }
            }
        }
        assert_eq!(seen_commands.len(), catalog.commands.len());

        let expected_headings = category_headings.into_iter().collect::<BTreeSet<_>>();
        let actual_headings = catalog
            .categories
            .iter()
            .filter(|category| category.kind != RawCategoryKind::Favorites)
            .map(|category| category.reference_heading.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(actual_headings, expected_headings);
    }

    #[test]
    fn raw_catalog_trace_counts_classifications_and_unique_references() {
        let catalog = RawCatalog::builtin();
        let executable = catalog
            .commands
            .iter()
            .filter(|command| matches!(command.execution, RawExecutionPolicy::Executable { .. }))
            .count();
        let references = catalog
            .commands
            .iter()
            .map(|command| command.reference.id.clone())
            .collect::<BTreeSet<_>>();
        assert_eq!(catalog.categories.len(), RAW_BUILTIN_CATEGORY_COUNT);
        assert_eq!(executable, RAW_BUILTIN_EXECUTABLE_COUNT);
        assert_eq!(references.len(), RAW_BUILTIN_COMMAND_COUNT);
        assert_eq!(RAW_REFERENCE_SHA256.len(), 64);
    }
}

#[cfg(test)]
mod raw_capability_tests {
    use super::*;
    use crate::{
        CapabilityCatalog, CapabilityEvidence, CapabilityEvidenceKind, CapabilityEvidenceOutcome,
        CapabilityImplementation, CapabilityImplementationKind, CapabilityProbeSpec,
        CapabilityReason, CapabilityRecord, CapabilitySnapshot, CapabilityToolId,
        YoctoEnvironmentIdentity,
    };
    use std::collections::BTreeMap;

    fn reason(message: &str) -> CapabilityReason {
        CapabilityReason::new("test.raw", message, None).unwrap()
    }

    fn authority(
        records: Vec<(CapabilityId, CapabilityState, Option<&str>)>,
    ) -> DaemonCompatibilitySnapshot {
        let capabilities = records
            .iter()
            .map(|(id, state, _)| CapabilityRecord {
                id: *id,
                state: state.clone(),
                evidence: match state {
                    CapabilityState::Available
                    | CapabilityState::AvailableWithLimitations { .. } => {
                        vec![CapabilityEvidence {
                            kind: CapabilityEvidenceKind::DirectProbe,
                            outcome: CapabilityEvidenceOutcome::Positive,
                            subject: id.as_str().into(),
                            detail: "positive Raw fixture evidence".into(),
                            argv: vec!["fixture".into()],
                        }]
                    }
                    CapabilityState::Unavailable { .. } => vec![CapabilityEvidence {
                        kind: CapabilityEvidenceKind::DirectProbe,
                        outcome: CapabilityEvidenceOutcome::Negative,
                        subject: id.as_str().into(),
                        detail: "negative Raw fixture evidence".into(),
                        argv: vec!["fixture".into()],
                    }],
                    CapabilityState::Unknown { .. } | CapabilityState::Unsupported { .. } => {
                        Vec::new()
                    }
                },
            })
            .collect();
        DaemonCompatibilitySnapshot {
            snapshot: CapabilitySnapshot {
                generation: 1,
                environment: YoctoEnvironmentIdentity::default(),
                capabilities,
            },
            implementations: records
                .into_iter()
                .filter_map(|(id, _, implementation)| {
                    implementation.map(|implementation| {
                        (
                            id,
                            CapabilityImplementation {
                                id: implementation.into(),
                                kind: CapabilityImplementationKind::Command,
                            },
                        )
                    })
                })
                .collect::<BTreeMap<_, _>>(),
        }
        .normalize()
        .unwrap()
    }

    fn command(line: usize) -> RawCommand {
        RawCatalog::builtin()
            .commands
            .into_iter()
            .find(|command| command.reference.id.as_str() == format!("wrynose-6-0.l{line:04}"))
            .unwrap()
    }

    fn with_requirement(requirement: RawCapabilityRequirement) -> RawCommand {
        let mut command = command(167);
        let RawExecutionPolicy::Executable { template } = &mut command.execution else {
            unreachable!()
        };
        template.capabilities = requirement;
        command
    }

    #[test]
    fn raw_capability_builtin_commands_have_explicit_fail_closed_requirements() {
        let catalog = RawCatalog::builtin();
        for command in &catalog.commands {
            match &command.execution {
                RawExecutionPolicy::Executable { template } => {
                    assert!(!template.capabilities.capabilities().is_empty());
                    let availability = command.availability(None);
                    assert_eq!(availability.state, RawAvailabilityState::Unknown);
                    assert!(!availability.issues.is_empty());
                }
                RawExecutionPolicy::ReferenceOnly { reason, .. } => {
                    let availability = command.availability(None);
                    assert_eq!(availability.state, RawAvailabilityState::Unsupported);
                    assert_eq!(availability.issues[0].reason, *reason);
                    assert!(availability.implementations.is_empty());
                }
            }
        }
    }

    #[test]
    fn raw_capability_all_of_preserves_available_and_limited_reasons() {
        let command = with_requirement(RawCapabilityRequirement::All {
            capabilities: vec![
                CapabilityId::BitBakeBuild,
                CapabilityId::BitBakeEnvironmentDump,
            ],
        });
        let authority = authority(vec![
            (
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("bitbake.argv"),
            ),
            (
                CapabilityId::BitBakeEnvironmentDump,
                CapabilityState::AvailableWithLimitations {
                    reason: reason("Environment output is truncated."),
                    limitations: vec!["Maximum 4 MiB output.".into()],
                },
                Some("bitbake.environment.argv"),
            ),
        ]);
        let availability = command.availability(Some(&authority));
        assert_eq!(availability.state, RawAvailabilityState::Limited);
        assert!(availability.is_enabled());
        assert_eq!(
            availability.issues[0].reason,
            "Environment output is truncated."
        );
        assert_eq!(
            availability.issues[0].limitations,
            vec!["Maximum 4 MiB output."]
        );
        assert_eq!(availability.implementations.len(), 2);
    }

    #[test]
    fn raw_capability_any_of_prefers_fully_available_implementation() {
        let command = with_requirement(RawCapabilityRequirement::Any {
            capabilities: vec![
                CapabilityId::BitBakeEnvironmentDump,
                CapabilityId::BitBakeBuild,
            ],
        });
        let authority = authority(vec![
            (
                CapabilityId::BitBakeEnvironmentDump,
                CapabilityState::AvailableWithLimitations {
                    reason: reason("Fallback output is limited."),
                    limitations: vec!["Recipe scope only.".into()],
                },
                Some("bitbake.environment.fallback"),
            ),
            (
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("bitbake.argv"),
            ),
        ]);
        let availability = command.availability(Some(&authority));
        assert_eq!(availability.state, RawAvailabilityState::Available);
        assert!(availability.issues.is_empty());
        assert_eq!(
            availability.implementations,
            vec![(CapabilityId::BitBakeBuild, "bitbake.argv".into())]
        );
    }

    #[test]
    fn raw_capability_preserves_unavailable_unknown_and_unsupported_states() {
        for (state, expected, message) in [
            (
                CapabilityState::Unavailable {
                    reason: reason("Required option was not found."),
                },
                RawAvailabilityState::Unavailable,
                "Required option was not found.",
            ),
            (
                CapabilityState::Unknown {
                    reason: reason("Probe did not complete."),
                },
                RawAvailabilityState::Unknown,
                "Probe did not complete.",
            ),
            (
                CapabilityState::Unsupported {
                    reason: reason("Backend cannot expose this operation."),
                },
                RawAvailabilityState::Unsupported,
                "Backend cannot expose this operation.",
            ),
        ] {
            let authority = authority(vec![(CapabilityId::BitBakeBuild, state, None)]);
            let command = with_requirement(RawCapabilityRequirement::All {
                capabilities: vec![CapabilityId::BitBakeBuild],
            });
            let availability = command.availability(Some(&authority));
            assert_eq!(availability.state, expected);
            assert_eq!(availability.issues[0].reason, message);
            assert!(!availability.is_enabled());
        }
    }

    #[test]
    fn raw_capability_missing_record_is_unknown_without_version_inference() {
        let authority = authority(Vec::new());
        let command = with_requirement(RawCapabilityRequirement::All {
            capabilities: vec![CapabilityId::BitBakeBuild],
        });
        let availability = command.availability(Some(&authority));
        assert_eq!(availability.state, RawAvailabilityState::Unknown);
        assert_eq!(
            availability.issues[0].reason,
            "bitbake.build has no capability evidence."
        );
    }

    #[test]
    fn raw_capability_probe_catalog_is_direct_bounded_and_version_agnostic() {
        let catalog = CapabilityCatalog::builtin();
        catalog.validate().unwrap();
        for id in CapabilityId::RAW_CLI {
            let entry = catalog.entry(id).unwrap();
            assert_eq!(entry.required_tools, vec![CapabilityToolId::BitBake]);
            assert_eq!(entry.probes.len(), 1);
            assert!(entry.fallback.is_none());
            let direct = match (&entry.probes[0], id) {
                (
                    CapabilityProbeSpec::CommandHelp {
                        tool: CapabilityToolId::BitBake,
                        subcommand: None,
                    },
                    CapabilityId::BitBakeRawCli,
                ) => true,
                (
                    CapabilityProbeSpec::CommandOption {
                        tool: CapabilityToolId::BitBake,
                        subcommand: None,
                        ..
                    },
                    id,
                ) => id != CapabilityId::BitBakeRawMulticonfig,
                (
                    CapabilityProbeSpec::CommandHelpText {
                        tool: CapabilityToolId::BitBake,
                        needle,
                    },
                    CapabilityId::BitBakeRawMulticonfig,
                ) => needle == "mc:",
                _ => false,
            };
            assert!(direct, "{}", id.as_str());
        }

        for command in RawCatalog::builtin().commands {
            if let RawExecutionPolicy::Executable { template } = command.execution {
                assert!(
                    template
                        .capabilities
                        .capabilities()
                        .contains(&CapabilityId::BitBakeRawCli)
                );
            }
        }

        let absent = command(201).availability(None);
        assert_eq!(absent.state, RawAvailabilityState::Unknown);
        assert!(absent.issues.iter().any(|issue| {
            issue.capability == Some(CapabilityId::BitBakeRawCli)
                && issue
                    .reason
                    .contains("No current environment capability snapshot")
        }));
        assert_eq!(
            command(847).availability(None).state,
            RawAvailabilityState::Unsupported
        );
    }

    #[test]
    fn raw_capability_probe_maps_representative_options_exactly() {
        let expected = [
            (145, CapabilityId::BitBakeRawUi),
            (201, CapabilityId::BitBakeRawDryRun),
            (325, CapabilityId::BitBakeRawServerToken),
            (1193, CapabilityId::BitBakeRawMulticonfig),
            (179, CapabilityId::BitBakeRawRunAll),
            (185, CapabilityId::BitBakeRawNoSetscene),
        ];
        for (line, capability) in expected {
            let command = command(line);
            let RawExecutionPolicy::Executable { template } = command.execution else {
                panic!("reference line {line} must be executable");
            };
            assert!(template.capabilities.capabilities().contains(&capability));
        }
    }
}
