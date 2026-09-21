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
