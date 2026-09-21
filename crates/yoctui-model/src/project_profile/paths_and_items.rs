#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectWorkflowStep {
    UseBuildPreset { preset: String },
    BuildTargets { targets: Vec<String> },
    RunRecipeTask { recipe: String, task: String },
    OpenProjectFile { path: PortableProjectPath },
    RefreshMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct PortableProjectPath(String);

impl PortableProjectPath {
    pub fn new(value: impl Into<String>) -> Result<Self, ProjectProfileError> {
        let value = value.into();
        validate_portable_path(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for PortableProjectPath {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

impl fmt::Display for PortableProjectPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectIdentityResolution<T> {
    Resolved(T),
    Stale {
        identity: String,
        reason: String,
    },
    Ambiguous {
        identity: String,
        candidates: Vec<T>,
    },
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ProjectProfileState {
    #[default]
    NotLoaded,
    Absent,
    Loaded(ProjectProfile),
    Invalid(String),
    GenerationPreview(ProjectProfile),
    Generating(ProjectProfile),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectProfileItemKind {
    FavoriteRecipe(usize),
    FavoriteImage(usize),
    FavoriteLayer(usize),
    BuildPreset(usize),
    Workflow(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectProfileItemStatus {
    Resolved,
    Stale(String),
    Ambiguous(usize),
    Unavailable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectProfileItem {
    pub kind: ProjectProfileItemKind,
    pub label: String,
    pub status: ProjectProfileItemStatus,
}

pub fn project_profile_items(
    state: &ProjectProfileState,
    workspace: &Workspace,
    available_images: &[String],
) -> Vec<ProjectProfileItem> {
    let (ProjectProfileState::Loaded(profile)
    | ProjectProfileState::GenerationPreview(profile)
    | ProjectProfileState::Generating(profile)) = state
    else {
        return Vec::new();
    };
    let unavailable = workspace.recipes.is_empty() && workspace.layers.is_empty();
    let status = |count: usize, reason: &str| match count {
        _ if unavailable => ProjectProfileItemStatus::Unavailable(reason.into()),
        0 => ProjectProfileItemStatus::Stale("not reported by BitBake".into()),
        1 => ProjectProfileItemStatus::Resolved,
        count => ProjectProfileItemStatus::Ambiguous(count),
    };
    let mut items = Vec::new();
    for (index, identity) in profile.favorites.recipes.iter().enumerate() {
        let count = workspace
            .recipes
            .iter()
            .filter(|item| item.name == *identity)
            .count();
        items.push(ProjectProfileItem {
            kind: ProjectProfileItemKind::FavoriteRecipe(index),
            label: format!("Recipe favorite: {identity}"),
            status: status(count, "recipe inventory unavailable"),
        });
    }
    for (index, identity) in profile.favorites.images.iter().enumerate() {
        let count = available_images
            .iter()
            .filter(|item| *item == identity)
            .count();
        items.push(ProjectProfileItem {
            kind: ProjectProfileItemKind::FavoriteImage(index),
            label: format!("Image favorite: {identity}"),
            status: status(count, "image inventory unavailable"),
        });
    }
    for (index, identity) in profile.favorites.layers.iter().enumerate() {
        let count = workspace
            .layers
            .iter()
            .filter(|item| item.name == *identity)
            .count();
        items.push(ProjectProfileItem {
            kind: ProjectProfileItemKind::FavoriteLayer(index),
            label: format!("Layer favorite: {identity}"),
            status: status(count, "layer inventory unavailable"),
        });
    }
    for (index, preset) in profile.build_presets.iter().enumerate() {
        let counts = preset
            .targets
            .iter()
            .map(|target| {
                workspace
                    .recipes
                    .iter()
                    .filter(|recipe| recipe.name == *target)
                    .count()
            })
            .collect::<Vec<_>>();
        let preset_status = if unavailable {
            ProjectProfileItemStatus::Unavailable("recipe inventory unavailable".into())
        } else if counts.contains(&0) {
            ProjectProfileItemStatus::Stale(
                "one or more targets are not reported by BitBake".into(),
            )
        } else if let Some(count) = counts.iter().find(|count| **count > 1) {
            ProjectProfileItemStatus::Ambiguous(*count)
        } else {
            ProjectProfileItemStatus::Resolved
        };
        items.push(ProjectProfileItem {
            kind: ProjectProfileItemKind::BuildPreset(index),
            label: format!("Build preset: {}", preset.name),
            status: preset_status,
        });
    }
    for (index, workflow) in profile.workflows.iter().enumerate() {
        items.push(ProjectProfileItem {
            kind: ProjectProfileItemKind::Workflow(index),
            label: format!("Workflow: {}", workflow.name),
            status: if unavailable {
                ProjectProfileItemStatus::Unavailable(
                    "authoritative workspace inventory unavailable".into(),
                )
            } else {
                ProjectProfileItemStatus::Resolved
            },
        });
    }
    items
}
