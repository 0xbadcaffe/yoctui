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
pub const RAW_FAVORITE_SCHEMA_VERSION: u16 = 1;
pub const MAX_RAW_FAVORITE_NAME_BYTES: usize = 160;
pub const MAX_RAW_FAVORITE_AGGREGATE_BYTES: usize = 256 * 1024;
pub const RAW_HISTORY_SCHEMA_VERSION: u16 = 1;
pub const MAX_RAW_HISTORY_RECORDS: usize = 256;
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
