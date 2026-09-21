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
