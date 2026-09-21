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

const RAW_DAEMON_PTY_NAMESPACE: u64 = 6 << 60;
const RAW_DAEMON_PTY_SEQUENCE_LIMIT: u64 = 1 << 60;

impl RawSessionId {
    pub fn daemon_pty_id(&self) -> Option<u64> {
        self.as_str()
            .strip_prefix("raw-session:daemon-")
            .and_then(|sequence| sequence.parse::<u64>().ok())
            .filter(|sequence| *sequence > 0 && *sequence < RAW_DAEMON_PTY_SEQUENCE_LIMIT)
            .map(|sequence| RAW_DAEMON_PTY_NAMESPACE | sequence)
    }
}

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
