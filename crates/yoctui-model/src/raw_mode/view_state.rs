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
pub struct RawRecipePicker {
    pub parameter: RawParameterId,
    pub recipes: Vec<String>,
    pub query: String,
    pub selection: usize,
}

impl RawRecipePicker {
    pub fn filtered(&self) -> Vec<&str> {
        let query = self.query.to_lowercase();
        self.recipes
            .iter()
            .map(String::as_str)
            .filter(|recipe| query.is_empty() || recipe.to_lowercase().contains(&query))
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RawFavoriteTemplateDigest(pub [u8; 32]);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawFavorite {
    pub schema_version: u16,
    pub command: RawCommandId,
    pub template_digest: RawFavoriteTemplateDigest,
    pub name: String,
    pub parameter_defaults: BTreeMap<RawParameterId, RawParameterValue>,
    pub additional_arguments: RawAdditionalArguments,
    pub order: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFavoriteProjection {
    pub stale: bool,
    pub reason: Option<String>,
    pub availability: RawCommandAvailability,
}

impl RawFavorite {
    pub fn new(
        command: &RawCommand,
        name: impl Into<String>,
        parameter_defaults: BTreeMap<RawParameterId, RawParameterValue>,
        additional_arguments: RawAdditionalArguments,
        order: u16,
    ) -> Result<Self, RawFavoriteError> {
        if !matches!(command.execution, RawExecutionPolicy::Executable { .. }) {
            return Err(RawFavoriteError::ReferenceOnly(command.id.clone()));
        }
        validate_raw_favorite_defaults(command, &parameter_defaults)?;
        additional_arguments.validate()?;
        let favorite = Self {
            schema_version: RAW_FAVORITE_SCHEMA_VERSION,
            command: command.id.clone(),
            template_digest: raw_favorite_template_digest(command),
            name: name.into(),
            parameter_defaults,
            additional_arguments,
            order,
        };
        favorite.validate()?;
        Ok(favorite)
    }

    pub fn validate(&self) -> Result<(), RawFavoriteError> {
        if self.schema_version != RAW_FAVORITE_SCHEMA_VERSION {
            return Err(RawFavoriteError::UnsupportedSchema(self.schema_version));
        }
        RawCommandId::new(self.command.as_str())
            .map_err(|_| RawFavoriteError::InvalidCommandIdentity)?;
        if self.name.is_empty()
            || self.name.trim() != self.name
            || self.name.len() > MAX_RAW_FAVORITE_NAME_BYTES
            || self.name.chars().any(char::is_control)
        {
            return Err(RawFavoriteError::InvalidName);
        }
        if self.parameter_defaults.len() > MAX_RAW_PARAMETERS {
            return Err(RawFavoriteError::TooManyDefaults);
        }
        for (parameter, value) in &self.parameter_defaults {
            RawParameterId::new(parameter.as_str())
                .map_err(|_| RawFavoriteError::InvalidParameterIdentity)?;
            validate_raw_execution_parameter_value(value)
                .map_err(|_| RawFavoriteError::InvalidDefault(parameter.clone()))?;
        }
        self.additional_arguments.validate()?;
        Ok(())
    }

    pub fn project(
        &self,
        catalog: &RawCatalog,
        authority: Option<&DaemonCompatibilitySnapshot>,
    ) -> RawFavoriteProjection {
        let Some(command) = catalog.command(&self.command) else {
            return stale_raw_favorite("The favorite command is absent from the current catalog.");
        };
        if raw_favorite_template_digest(command) != self.template_digest {
            return stale_raw_favorite("The favorite command template changed.");
        }
        if let Err(error) = validate_raw_favorite_defaults(command, &self.parameter_defaults) {
            return stale_raw_favorite(&format!("The favorite defaults are stale: {error}"));
        }
        RawFavoriteProjection {
            stale: false,
            reason: None,
            availability: command.availability(authority),
        }
    }
}

fn stale_raw_favorite(reason: &str) -> RawFavoriteProjection {
    RawFavoriteProjection {
        stale: true,
        reason: Some(reason.into()),
        availability: RawCommandAvailability {
            state: RawAvailabilityState::Unsupported,
            issues: vec![RawCapabilityIssue {
                capability: None,
                reason: reason.into(),
                limitations: Vec::new(),
            }],
            implementations: Vec::new(),
        },
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RawFavoriteError {
    #[error("unsupported Raw favorite schema version {0}")]
    UnsupportedSchema(u16),
    #[error("invalid Raw favorite command identity")]
    InvalidCommandIdentity,
    #[error("invalid Raw favorite parameter identity")]
    InvalidParameterIdentity,
    #[error("Raw favorite name is empty or exceeds its bound")]
    InvalidName,
    #[error("Raw favorite has too many parameter defaults")]
    TooManyDefaults,
    #[error("Raw favorite default for {0} is invalid")]
    InvalidDefault(RawParameterId),
    #[error("Raw favorite command is reference-only: {0}")]
    ReferenceOnly(RawCommandId),
    #[error("Raw favorite command is unavailable: {0}")]
    Unavailable(String),
    #[error("Raw favorite repeats command identity {0}")]
    DuplicateCommand(RawCommandId),
    #[error("Raw favorite ordering is not contiguous")]
    InvalidOrder,
    #[error("Raw favorites are bounded to {MAX_RAW_FAVORITES} entries")]
    TooManyFavorites,
    #[error("Raw favorites exceed their aggregate byte bound")]
    AggregateTooLarge,
    #[error("Raw favorite is unknown: {0}")]
    Unknown(RawCommandId),
    #[error(transparent)]
    InvalidAdditionalArguments(#[from] RawArgvError),
    #[error(transparent)]
    InvalidParameter(#[from] RawParameterError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawFavoriteConfirmation {
    pub command: RawCommandId,
    pub return_focus: RawModeFocus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawOutputViewState {
    pub request: Option<RawRequestId>,
    pub stream: RawOutputStream,
    pub follow: bool,
    pub vertical_scroll: usize,
    pub horizontal_scroll: usize,
    pub query: String,
    pub searching: bool,
}

impl Default for RawOutputViewState {
    fn default() -> Self {
        Self {
            request: None,
            stream: RawOutputStream::Stdout,
            follow: true,
            vertical_scroll: 0,
            horizontal_scroll: 0,
            query: String::new(),
            searching: false,
        }
    }
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
    pub recipe_picker: Option<RawRecipePicker>,
    pub execution: Option<RawCommandId>,
    pub execution_states: BTreeMap<RawRequestId, RawExecutionState>,
    pub output: RawOutputViewState,
    pub history: Vec<RawHistoryRecord>,
    pub history_selection: usize,
    pub favorites: Vec<RawFavorite>,
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
    OpenRecipePicker {
        parameter: RawParameterId,
        recipes: Vec<String>,
    },
    SelectRecipePicker {
        delta: isize,
    },
    AppendRecipePickerQuery(char),
    BackspaceRecipePickerQuery,
    ConfirmRecipePicker,
    CancelRecipePicker,
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
    CloseExecution,
    SetExecutionAttachment {
        request: RawRequestId,
        attachment: RawAttachmentState,
    },
    ToggleOutputFollow,
    SelectOutputStream(RawOutputStream),
    ScrollOutput {
        vertical: isize,
        horizontal: isize,
    },
    BeginOutputSearch,
    AppendOutputSearch(char),
    BackspaceOutputSearch,
    FinishOutputSearch,
    ClearOutputSearch,
    OpenExecution(RawCommandId),
    OpenHistory,
    SelectHistory {
        delta: isize,
    },
    ActivateHistory,
    OpenFavorites,
    SelectFavorite {
        delta: isize,
    },
    ActivateFavorite,
    ToggleFavorite,
    RenameFavorite {
        name: String,
    },
    MoveFavorite {
        delta: isize,
    },
    RemoveFavorite,
    InspectFavorite,
    ConfirmFavorite,
    CancelFavorite,
    ReprojectCatalog,
    ReprojectAuthority,
    DismissNotification,
}
