use crate::Workspace;
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::BTreeSet, fmt};
use thiserror::Error;

pub const PROJECT_PROFILE_SCHEMA_VERSION: u32 = 1;
const MAX_COLLECTION_ITEMS: usize = 256;
const MAX_WORKFLOW_STEPS: usize = 128;
const MAX_IDENTITY_BYTES: usize = 256;
const MAX_LABEL_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectProfile {
    pub schema_version: u32,
    #[serde(default)]
    pub favorites: ProjectFavorites,
    #[serde(default)]
    pub build_presets: Vec<ProjectBuildPreset>,
    #[serde(default)]
    pub workflows: Vec<ProjectWorkflow>,
}

impl ProjectProfile {
    pub fn validate(&self) -> Result<(), ProjectProfileError> {
        if self.schema_version != PROJECT_PROFILE_SCHEMA_VERSION {
            return Err(ProjectProfileError::UnsupportedSchema(self.schema_version));
        }
        self.favorites.validate()?;
        bounded_collection("build_presets", self.build_presets.len())?;
        bounded_collection("workflows", self.workflows.len())?;

        let mut preset_names = BTreeSet::new();
        for (index, preset) in self.build_presets.iter().enumerate() {
            preset.validate(index)?;
            if !preset_names.insert(preset.name.as_str()) {
                return Err(ProjectProfileError::DuplicateName {
                    collection: "build_presets",
                    name: preset.name.clone(),
                });
            }
        }

        let mut workflow_names = BTreeSet::new();
        for (index, workflow) in self.workflows.iter().enumerate() {
            workflow.validate(index, &preset_names)?;
            if !workflow_names.insert(workflow.name.as_str()) {
                return Err(ProjectProfileError::DuplicateName {
                    collection: "workflows",
                    name: workflow.name.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectFavorites {
    pub recipes: Vec<String>,
    pub images: Vec<String>,
    pub layers: Vec<String>,
}

impl ProjectFavorites {
    fn validate(&self) -> Result<(), ProjectProfileError> {
        validate_identities("favorites.recipes", &self.recipes)?;
        validate_identities("favorites.images", &self.images)?;
        validate_identities("favorites.layers", &self.layers)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBuildPreset {
    pub name: String,
    pub targets: Vec<String>,
    #[serde(default)]
    pub machine: Option<String>,
    #[serde(default)]
    pub distro: Option<String>,
    #[serde(default)]
    pub options: ProjectBuildOptions,
}

impl ProjectBuildPreset {
    fn validate(&self, index: usize) -> Result<(), ProjectProfileError> {
        validate_label(&format!("build_presets[{index}].name"), &self.name)?;
        if self.targets.is_empty() {
            return Err(ProjectProfileError::InvalidField {
                field: format!("build_presets[{index}].targets"),
                reason: "at least one target is required".into(),
            });
        }
        validate_identities(&format!("build_presets[{index}].targets"), &self.targets)?;
        for (field, value) in [("machine", &self.machine), ("distro", &self.distro)] {
            if let Some(value) = value {
                validate_identity(&format!("build_presets[{index}].{field}"), value)?;
            }
        }
        if self.options.jobs == Some(0) {
            return Err(ProjectProfileError::InvalidField {
                field: format!("build_presets[{index}].options.jobs"),
                reason: "jobs must be positive".into(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ProjectBuildOptions {
    pub jobs: Option<u16>,
    pub continue_on_error: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectWorkflow {
    pub name: String,
    pub steps: Vec<ProjectWorkflowStep>,
}

impl ProjectWorkflow {
    fn validate(
        &self,
        index: usize,
        preset_names: &BTreeSet<&str>,
    ) -> Result<(), ProjectProfileError> {
        validate_label(&format!("workflows[{index}].name"), &self.name)?;
        if self.steps.is_empty() || self.steps.len() > MAX_WORKFLOW_STEPS {
            return Err(ProjectProfileError::InvalidField {
                field: format!("workflows[{index}].steps"),
                reason: format!("must contain 1..={MAX_WORKFLOW_STEPS} typed steps"),
            });
        }
        for (step_index, step) in self.steps.iter().enumerate() {
            let field = format!("workflows[{index}].steps[{step_index}]");
            match step {
                ProjectWorkflowStep::UseBuildPreset { preset } => {
                    validate_label(&format!("{field}.preset"), preset)?;
                    if !preset_names.contains(preset.as_str()) {
                        return Err(ProjectProfileError::UnknownPreset(preset.clone()));
                    }
                }
                ProjectWorkflowStep::BuildTargets { targets } => {
                    if targets.is_empty() {
                        return Err(ProjectProfileError::InvalidField {
                            field,
                            reason: "at least one target is required".into(),
                        });
                    }
                    validate_identities(&format!("{field}.targets"), targets)?;
                }
                ProjectWorkflowStep::RunRecipeTask { recipe, task } => {
                    validate_identity(&format!("{field}.recipe"), recipe)?;
                    validate_identity(&format!("{field}.task"), task)?;
                }
                ProjectWorkflowStep::OpenProjectFile { path } => {
                    if path.as_str().is_empty() {
                        return Err(ProjectProfileError::InvalidField {
                            field,
                            reason: "project path is empty".into(),
                        });
                    }
                }
                ProjectWorkflowStep::RefreshMetadata => {}
            }
        }
        Ok(())
    }
}

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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProjectProfileError {
    #[error("unsupported project profile schema version {0}")]
    UnsupportedSchema(u32),
    #[error("invalid project profile field `{field}`: {reason}")]
    InvalidField { field: String, reason: String },
    #[error("duplicate `{name}` in {collection}")]
    DuplicateName {
        collection: &'static str,
        name: String,
    },
    #[error("workflow references unknown build preset `{0}`")]
    UnknownPreset(String),
}

fn bounded_collection(field: &'static str, len: usize) -> Result<(), ProjectProfileError> {
    if len > MAX_COLLECTION_ITEMS {
        return Err(ProjectProfileError::InvalidField {
            field: field.into(),
            reason: format!("contains more than {MAX_COLLECTION_ITEMS} entries"),
        });
    }
    Ok(())
}

fn validate_identities(field: &str, values: &[String]) -> Result<(), ProjectProfileError> {
    bounded_collection(
        match field {
            "favorites.recipes" => "favorites.recipes",
            "favorites.images" => "favorites.images",
            "favorites.layers" => "favorites.layers",
            _ => "profile identities",
        },
        values.len(),
    )?;
    let mut unique = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        validate_identity(&format!("{field}[{index}]"), value)?;
        if !unique.insert(value) {
            return Err(ProjectProfileError::InvalidField {
                field: format!("{field}[{index}]"),
                reason: "duplicate identity".into(),
            });
        }
    }
    Ok(())
}

fn validate_identity(field: &str, value: &str) -> Result<(), ProjectProfileError> {
    if value.is_empty() || value.len() > MAX_IDENTITY_BYTES || value.chars().any(|character| {
        character.is_control()
            || character.is_whitespace()
            || !matches!(character, 'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '.' | '+' | '/')
    }) {
        return Err(ProjectProfileError::InvalidField {
            field: field.into(),
            reason: "must be a bounded portable logical identity".into(),
        });
    }
    Ok(())
}

fn validate_label(field: &str, value: &str) -> Result<(), ProjectProfileError> {
    if value.trim() != value
        || value.is_empty()
        || value.len() > MAX_LABEL_BYTES
        || value.chars().any(char::is_control)
    {
        return Err(ProjectProfileError::InvalidField {
            field: field.into(),
            reason: "must be a nonempty bounded label without control characters".into(),
        });
    }
    Ok(())
}

fn validate_portable_path(value: &str) -> Result<(), ProjectProfileError> {
    let invalid = value.is_empty()
        || value.len() > MAX_IDENTITY_BYTES
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.contains('\\')
        || value.contains(':')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."));
    if invalid {
        return Err(ProjectProfileError::InvalidField {
            field: "project path".into(),
            reason: "must be a portable repository-relative path without escape components".into(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layer, Recipe};
    use std::path::PathBuf;

    fn profile() -> ProjectProfile {
        ProjectProfile {
            schema_version: PROJECT_PROFILE_SCHEMA_VERSION,
            favorites: ProjectFavorites {
                recipes: vec!["busybox".into()],
                images: vec!["core-image-minimal".into()],
                layers: vec!["meta-poky".into()],
            },
            build_presets: vec![ProjectBuildPreset {
                name: "qemu smoke".into(),
                targets: vec!["core-image-minimal".into()],
                machine: Some("qemux86-64".into()),
                distro: None,
                options: ProjectBuildOptions {
                    jobs: Some(4),
                    continue_on_error: false,
                },
            }],
            workflows: vec![ProjectWorkflow {
                name: "smoke".into(),
                steps: vec![
                    ProjectWorkflowStep::RefreshMetadata,
                    ProjectWorkflowStep::UseBuildPreset {
                        preset: "qemu smoke".into(),
                    },
                    ProjectWorkflowStep::OpenProjectFile {
                        path: PortableProjectPath::new("conf/team.inc").unwrap(),
                    },
                ],
            }],
        }
    }

    #[test]
    fn project_profile_accepts_typed_team_intent() {
        assert_eq!(profile().validate(), Ok(()));
    }

    #[test]
    fn project_profile_rejects_schema_duplicates_and_unknown_presets() {
        let mut value = profile();
        value.schema_version = 2;
        assert_eq!(
            value.validate(),
            Err(ProjectProfileError::UnsupportedSchema(2))
        );
        let mut value = profile();
        value.favorites.images.push("core-image-minimal".into());
        assert!(matches!(
            value.validate(),
            Err(ProjectProfileError::InvalidField { .. })
        ));
        let mut value = profile();
        value.workflows[0].steps[1] = ProjectWorkflowStep::UseBuildPreset {
            preset: "missing".into(),
        };
        assert_eq!(
            value.validate(),
            Err(ProjectProfileError::UnknownPreset("missing".into()))
        );
    }

    #[test]
    fn project_profile_paths_reject_absolute_escape_and_platform_syntax() {
        for invalid in ["", "/etc/passwd", "../secret", "a/../b", "a\\b", "C:/tmp"] {
            assert!(PortableProjectPath::new(invalid).is_err(), "{invalid}");
        }
        assert_eq!(
            PortableProjectPath::new("docs/release.toml")
                .unwrap()
                .as_str(),
            "docs/release.toml"
        );
    }

    #[test]
    fn project_profile_workflows_are_closed_typed_actions() {
        let steps = &profile().workflows[0].steps;
        assert!(matches!(steps[0], ProjectWorkflowStep::RefreshMetadata));
        assert!(!format!("{steps:?}").contains("command"));
    }

    #[test]
    fn project_profile_resolution_keeps_stale_and_ambiguous_explicit() {
        let stale: ProjectIdentityResolution<String> = ProjectIdentityResolution::Stale {
            identity: "old-image".into(),
            reason: "not reported by BitBake".into(),
        };
        assert!(matches!(stale, ProjectIdentityResolution::Stale { .. }));
        let ambiguous: ProjectIdentityResolution<String> = ProjectIdentityResolution::Ambiguous {
            identity: "virtual/kernel".into(),
            candidates: vec!["linux-yocto".into(), "linux-vendor".into()],
        };
        assert!(matches!(
            ambiguous,
            ProjectIdentityResolution::Ambiguous { .. }
        ));
    }

    #[test]
    fn project_profile_items_resolve_only_against_authoritative_workspace() {
        let profile = profile();
        let state = ProjectProfileState::Loaded(profile);
        let workspace = Workspace {
            recipes: vec![
                Recipe {
                    name: "busybox".into(),
                    ..Recipe::default()
                },
                Recipe {
                    name: "core-image-minimal".into(),
                    ..Recipe::default()
                },
            ],
            layers: vec![Layer {
                name: "meta-poky".into(),
                path: PathBuf::from("/src/meta-poky"),
                priority: Some(5),
            }],
            ..Workspace::default()
        };
        let items = project_profile_items(&state, &workspace, &["core-image-minimal".into()]);
        assert!(
            items
                .iter()
                .all(|item| matches!(item.status, ProjectProfileItemStatus::Resolved))
        );

        let stale = project_profile_items(&state, &Workspace::default(), &[]);
        assert!(
            stale
                .iter()
                .all(|item| matches!(item.status, ProjectProfileItemStatus::Unavailable(_)))
        );
    }
}
