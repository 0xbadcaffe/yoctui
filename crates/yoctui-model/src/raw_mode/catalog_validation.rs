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
