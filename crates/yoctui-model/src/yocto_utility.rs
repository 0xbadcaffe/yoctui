mod layers;
pub use layers::*;

use crate::CapabilityId;

pub const MAX_YOCTO_UTILITY_TEXT_BYTES: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum YoctoUtilityCommand {
    ConfigBuild,
    LayersShowLayers,
    LayersShowRecipes,
    LayersShowOverlayed,
    LayersShowAppends,
    LayersShowCrossDepends,
    LayersAddLayer,
    LayersRemoveLayer,
    LayersFlatten,
    LayersLayerIndexFetch,
    LayersLayerIndexShowDepends,
    LayersCreateLayer,
    LayersShowMachines,
    LayersSaveBuildConf,
    LayersCreateLayersSetup,
}

impl YoctoUtilityCommand {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ConfigBuild => "BitBake config build",
            command => match command.layer_subcommand() {
                Some(subcommand) => subcommand.label(),
                None => "BitBake layers",
            },
        }
    }

    pub const fn tool(self) -> &'static str {
        match self {
            Self::ConfigBuild => "bitbake-config-build",
            _ => "bitbake-layers",
        }
    }

    pub const fn layer_subcommand(self) -> Option<LayerUtilitySubcommand> {
        Some(match self {
            Self::ConfigBuild => return None,
            Self::LayersShowLayers => LayerUtilitySubcommand::ShowLayers,
            Self::LayersShowRecipes => LayerUtilitySubcommand::ShowRecipes,
            Self::LayersShowOverlayed => LayerUtilitySubcommand::ShowOverlayed,
            Self::LayersShowAppends => LayerUtilitySubcommand::ShowAppends,
            Self::LayersShowCrossDepends => LayerUtilitySubcommand::ShowCrossDepends,
            Self::LayersAddLayer => LayerUtilitySubcommand::AddLayer,
            Self::LayersRemoveLayer => LayerUtilitySubcommand::RemoveLayer,
            Self::LayersFlatten => LayerUtilitySubcommand::Flatten,
            Self::LayersLayerIndexFetch => LayerUtilitySubcommand::LayerIndexFetch,
            Self::LayersLayerIndexShowDepends => LayerUtilitySubcommand::LayerIndexShowDepends,
            Self::LayersCreateLayer => LayerUtilitySubcommand::CreateLayer,
            Self::LayersShowMachines => LayerUtilitySubcommand::ShowMachines,
            Self::LayersSaveBuildConf => LayerUtilitySubcommand::SaveBuildConf,
            Self::LayersCreateLayersSetup => LayerUtilitySubcommand::CreateLayersSetup,
        })
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
    Layers(LayerUtilityDraft),
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
            command => YoctoUtilityDraft::Layers(LayerUtilityDraft::new(
                command
                    .layer_subcommand()
                    .expect("non-config command has a layer subcommand"),
            )),
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
            YoctoUtilityDraft::Layers(draft) => draft.fields(),
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
            YoctoUtilityDraft::Layers(draft) => draft.cycle_choice(self.selected_field),
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
            YoctoUtilityDraft::Layers(draft) => draft.selected_text_mut(self.selected_field),
            _ => None,
        }
    }

    pub fn capability(&self) -> CapabilityId {
        match &self.draft {
            YoctoUtilityDraft::ConfigBuild { operation, .. } => operation.capability(),
            YoctoUtilityDraft::Layers(draft) => draft.capability(),
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
            YoctoUtilityDraft::Layers(draft) => draft.arguments(),
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
