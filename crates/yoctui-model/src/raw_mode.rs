use crate::{CapabilityId, CapabilityState, DaemonCompatibilitySnapshot};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Component};
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

    pub fn command(&self, id: &RawCommandId) -> Option<&RawCommand> {
        self.commands.iter().find(|command| &command.id == id)
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
