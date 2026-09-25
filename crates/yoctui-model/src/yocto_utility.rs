use std::path::PathBuf;

use crate::{BitBakeLayersOperation, CapabilityId};

pub const MAX_YOCTO_UTILITY_TEXT_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoctoUtilityCommand {
    ConfigBuild,
    LayersShowLayers,
    LayersShowRecipes,
    LayersShowOverlayed,
    LayersCreateLayer,
}

impl YoctoUtilityCommand {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ConfigBuild => "BitBake config build",
            Self::LayersShowLayers => "Show configured layers",
            Self::LayersShowRecipes => "Show matching recipes",
            Self::LayersShowOverlayed => "Show overlayed recipes",
            Self::LayersCreateLayer => "Create layer",
        }
    }

    pub const fn tool(self) -> &'static str {
        match self {
            Self::ConfigBuild => "bitbake-config-build",
            _ => "bitbake-layers",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfigBuildOperation {
    #[default]
    ListFragments,
    ShowFragment,
    EnableFragments,
    DisableFragments,
    DisableAllFragments,
}

impl ConfigBuildOperation {
    pub const ALL: [Self; 5] = [
        Self::ListFragments,
        Self::ShowFragment,
        Self::EnableFragments,
        Self::DisableFragments,
        Self::DisableAllFragments,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::ListFragments => "list-fragments",
            Self::ShowFragment => "show-fragment",
            Self::EnableFragments => "enable-fragment",
            Self::DisableFragments => "disable-fragment",
            Self::DisableAllFragments => "disable-all-fragments",
        }
    }

    pub const fn needs_fragments(self) -> bool {
        matches!(
            self,
            Self::ShowFragment | Self::EnableFragments | Self::DisableFragments
        )
    }

    pub const fn capability(self) -> CapabilityId {
        match self {
            Self::ListFragments => CapabilityId::BitBakeConfigBuildListFragments,
            Self::ShowFragment => CapabilityId::BitBakeConfigBuildShowFragment,
            Self::EnableFragments => CapabilityId::BitBakeConfigBuildEnableFragment,
            Self::DisableFragments => CapabilityId::BitBakeConfigBuildDisableFragment,
            Self::DisableAllFragments => CapabilityId::BitBakeConfigBuildDisableAllFragments,
        }
    }

    fn shifted(self, delta: isize) -> Self {
        let index = Self::ALL
            .iter()
            .position(|value| *value == self)
            .unwrap_or(0);
        let count = Self::ALL.len();
        let next = if delta.is_negative() {
            (index + count - delta.unsigned_abs() % count) % count
        } else {
            (index + delta as usize % count) % count
        };
        Self::ALL[next]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YoctoUtilityDraft {
    ConfigBuild {
        operation: ConfigBuildOperation,
        fragments: String,
    },
    LayersShowLayers,
    LayersShowRecipes {
        pattern: String,
    },
    LayersShowOverlayed,
    LayersCreateLayer {
        directory: String,
        add_to_bblayers: bool,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoctoUtilityFieldKind {
    Choice,
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoctoUtilityDialog {
    pub command: YoctoUtilityCommand,
    pub draft: YoctoUtilityDraft,
    pub selected_field: usize,
    pub validation_error: Option<String>,
}

impl YoctoUtilityDialog {
    pub fn new(command: YoctoUtilityCommand) -> Self {
        let draft = match command {
            YoctoUtilityCommand::ConfigBuild => YoctoUtilityDraft::ConfigBuild {
                operation: ConfigBuildOperation::ListFragments,
                fragments: String::new(),
            },
            YoctoUtilityCommand::LayersShowLayers => YoctoUtilityDraft::LayersShowLayers,
            YoctoUtilityCommand::LayersShowRecipes => YoctoUtilityDraft::LayersShowRecipes {
                pattern: "linux-*".into(),
            },
            YoctoUtilityCommand::LayersShowOverlayed => YoctoUtilityDraft::LayersShowOverlayed,
            YoctoUtilityCommand::LayersCreateLayer => YoctoUtilityDraft::LayersCreateLayer {
                directory: String::new(),
                add_to_bblayers: false,
            },
        };
        Self {
            command,
            draft,
            selected_field: 0,
            validation_error: None,
        }
    }

    pub fn fields(&self) -> Vec<(&'static str, String, YoctoUtilityFieldKind)> {
        match &self.draft {
            YoctoUtilityDraft::ConfigBuild {
                operation,
                fragments,
            } => {
                let mut fields = vec![(
                    "Operation",
                    operation.label().into(),
                    YoctoUtilityFieldKind::Choice,
                )];
                if operation.needs_fragments() {
                    fields.push((
                        if *operation == ConfigBuildOperation::ShowFragment {
                            "Fragment"
                        } else {
                            "Fragments"
                        },
                        fragments.clone(),
                        YoctoUtilityFieldKind::Text,
                    ));
                }
                fields
            }
            YoctoUtilityDraft::LayersShowLayers | YoctoUtilityDraft::LayersShowOverlayed => {
                Vec::new()
            }
            YoctoUtilityDraft::LayersShowRecipes { pattern } => vec![(
                "Recipe pattern",
                pattern.clone(),
                YoctoUtilityFieldKind::Text,
            )],
            YoctoUtilityDraft::LayersCreateLayer {
                directory,
                add_to_bblayers,
            } => vec![
                (
                    "Layer directory",
                    directory.clone(),
                    YoctoUtilityFieldKind::Text,
                ),
                (
                    "Add to bblayers.conf",
                    if *add_to_bblayers { "yes" } else { "no" }.into(),
                    YoctoUtilityFieldKind::Choice,
                ),
            ],
        }
    }

    pub fn select_field(&mut self, delta: isize) {
        let count = self.fields().len();
        if count == 0 {
            self.selected_field = 0;
            return;
        }
        self.selected_field = if delta.is_negative() {
            (self.selected_field + count - delta.unsigned_abs() % count) % count
        } else {
            (self.selected_field + delta as usize % count) % count
        };
    }

    pub fn cycle_choice(&mut self, delta: isize) {
        match &mut self.draft {
            YoctoUtilityDraft::ConfigBuild { operation, .. } if self.selected_field == 0 => {
                *operation = operation.shifted(delta);
                self.selected_field = self.selected_field.min(self.fields().len() - 1);
            }
            YoctoUtilityDraft::LayersCreateLayer {
                add_to_bblayers, ..
            } if self.selected_field == 1 => *add_to_bblayers = !*add_to_bblayers,
            _ => {}
        }
        self.validation_error = None;
    }

    pub fn append(&mut self, character: char) {
        let value = self.selected_text_mut();
        if let Some(value) = value
            && !character.is_control()
            && value.len() + character.len_utf8() <= MAX_YOCTO_UTILITY_TEXT_BYTES
        {
            value.push(character);
            self.validation_error = None;
        }
    }

    pub fn backspace(&mut self) {
        if let Some(value) = self.selected_text_mut() {
            value.pop();
            self.validation_error = None;
        }
    }

    pub fn clear(&mut self) {
        if let Some(value) = self.selected_text_mut() {
            value.clear();
            self.validation_error = None;
        }
    }

    fn selected_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.draft {
            YoctoUtilityDraft::ConfigBuild { fragments, .. } if self.selected_field == 1 => {
                Some(fragments)
            }
            YoctoUtilityDraft::LayersShowRecipes { pattern } if self.selected_field == 0 => {
                Some(pattern)
            }
            YoctoUtilityDraft::LayersCreateLayer { directory, .. } if self.selected_field == 0 => {
                Some(directory)
            }
            _ => None,
        }
    }

    pub fn capability(&self) -> CapabilityId {
        match &self.draft {
            YoctoUtilityDraft::ConfigBuild { operation, .. } => operation.capability(),
            YoctoUtilityDraft::LayersShowLayers => CapabilityId::BitBakeLayersShowLayers,
            YoctoUtilityDraft::LayersShowRecipes { .. } => CapabilityId::BitBakeLayersShowRecipes,
            YoctoUtilityDraft::LayersShowOverlayed => CapabilityId::BitBakeLayersShowOverlayed,
            YoctoUtilityDraft::LayersCreateLayer {
                add_to_bblayers: true,
                ..
            } => CapabilityId::BitBakeLayersCreateAndAddLayer,
            YoctoUtilityDraft::LayersCreateLayer { .. } => CapabilityId::BitBakeLayersCreateLayer,
        }
    }

    pub fn arguments(&self) -> Result<Vec<String>, String> {
        match &self.draft {
            YoctoUtilityDraft::ConfigBuild {
                operation,
                fragments,
            } => {
                let mut arguments = vec![operation.label().into()];
                if operation.needs_fragments() {
                    let names = parse_fragment_names(fragments)?;
                    if *operation == ConfigBuildOperation::ShowFragment && names.len() != 1 {
                        return Err("show-fragment requires exactly one fragment name".into());
                    }
                    arguments.extend(names);
                }
                Ok(arguments)
            }
            YoctoUtilityDraft::LayersShowLayers => Ok(vec!["show-layers".into()]),
            YoctoUtilityDraft::LayersShowRecipes { pattern } => {
                let pattern = pattern.trim();
                let operation = BitBakeLayersOperation::ShowRecipes {
                    pattern: (!pattern.is_empty()).then(|| pattern.to_owned()),
                };
                operation.validate().map_err(|error| error.to_string())?;
                let mut arguments = vec!["show-recipes".into()];
                if !pattern.is_empty() {
                    arguments.push(pattern.into());
                }
                Ok(arguments)
            }
            YoctoUtilityDraft::LayersShowOverlayed => Ok(vec!["show-overlayed".into()]),
            YoctoUtilityDraft::LayersCreateLayer {
                directory,
                add_to_bblayers,
            } => {
                let operation = BitBakeLayersOperation::CreateLayer {
                    directory: PathBuf::from(directory.trim()),
                    add: *add_to_bblayers,
                };
                operation.validate().map_err(|error| error.to_string())?;
                let mut arguments = vec!["create-layer".into()];
                if *add_to_bblayers {
                    arguments.push("--add-layer".into());
                }
                arguments.push(directory.trim().into());
                Ok(arguments)
            }
        }
    }
}

fn parse_fragment_names(value: &str) -> Result<Vec<String>, String> {
    let names = value
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if names.is_empty() || names.len() > 64 {
        return Err("enter between one and 64 fragment names".into());
    }
    if names.iter().any(|name| {
        name.len() > 256
            || name.starts_with('-')
            || !name
                .chars()
                .all(|character| character.is_alphanumeric() || "/._+-".contains(character))
    }) {
        return Err("fragment names contain unsupported characters".into());
    }
    Ok(names)
}
