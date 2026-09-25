use std::path::PathBuf;

use crate::{BitBakeLayersOperation, CapabilityId};

use super::YoctoUtilityFieldKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerUtilitySubcommand {
    ShowLayers,
    ShowOverlayed,
    ShowRecipes,
    ShowAppends,
    ShowCrossDepends,
    AddLayer,
    RemoveLayer,
    Flatten,
    LayerIndexFetch,
    LayerIndexShowDepends,
    CreateLayer,
    ShowMachines,
    SaveBuildConf,
    CreateLayersSetup,
}

impl LayerUtilitySubcommand {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ShowLayers => "Show configured layers",
            Self::ShowOverlayed => "Show overlayed recipes",
            Self::ShowRecipes => "Show matching recipes",
            Self::ShowAppends => "Show recipe appends",
            Self::ShowCrossDepends => "Show cross-layer dependencies",
            Self::AddLayer => "Add layers",
            Self::RemoveLayer => "Remove layers",
            Self::Flatten => "Flatten layers",
            Self::LayerIndexFetch => "Fetch from layer index",
            Self::LayerIndexShowDepends => "Show layer-index dependencies",
            Self::CreateLayer => "Create layer",
            Self::ShowMachines => "Show machines",
            Self::SaveBuildConf => "Save build configuration",
            Self::CreateLayersSetup => "Create layers setup",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerUtilityDraft {
    pub subcommand: LayerUtilitySubcommand,
    values: Vec<String>,
}

#[derive(Clone, Copy)]
struct FieldSpec {
    label: &'static str,
    kind: YoctoUtilityFieldKind,
    default: &'static str,
}

impl FieldSpec {
    const fn text(label: &'static str, default: &'static str) -> Self {
        Self {
            label,
            kind: YoctoUtilityFieldKind::Text,
            default,
        }
    }

    const fn toggle(label: &'static str) -> Self {
        Self {
            label,
            kind: YoctoUtilityFieldKind::Choice,
            default: "no",
        }
    }
}

impl LayerUtilityDraft {
    pub fn new(subcommand: LayerUtilitySubcommand) -> Self {
        let values = field_specs(subcommand)
            .iter()
            .map(|field| field.default.into())
            .collect();
        Self { subcommand, values }
    }

    pub fn fields(&self) -> Vec<(&'static str, String, YoctoUtilityFieldKind)> {
        field_specs(self.subcommand)
            .iter()
            .zip(&self.values)
            .map(|(field, value)| (field.label, value.clone(), field.kind))
            .collect()
    }

    pub fn selected_text_mut(&mut self, selected: usize) -> Option<&mut String> {
        (field_specs(self.subcommand).get(selected)?.kind == YoctoUtilityFieldKind::Text)
            .then(|| &mut self.values[selected])
    }

    pub fn cycle_choice(&mut self, selected: usize) {
        if field_specs(self.subcommand)
            .get(selected)
            .is_some_and(|field| field.kind == YoctoUtilityFieldKind::Choice)
        {
            self.values[selected] = if self.values[selected] == "yes" {
                "no".into()
            } else {
                "yes".into()
            };
        }
    }

    pub fn capability(&self) -> CapabilityId {
        self.operation().capability()
    }

    pub fn arguments(&self) -> Result<Vec<String>, String> {
        self.operation()
            .arguments()
            .map_err(|error| error.to_string())
    }

    pub fn operation(&self) -> BitBakeLayersOperation {
        use LayerUtilitySubcommand as S;
        match self.subcommand {
            S::ShowLayers => BitBakeLayersOperation::ShowLayers,
            S::ShowOverlayed => BitBakeLayersOperation::ShowOverlayed {
                filenames: self.yes(0),
                same_version: self.yes(1),
                multiconfig: self.optional(2),
            },
            S::ShowRecipes => BitBakeLayersOperation::ShowRecipes {
                patterns: self.words(0),
                filenames: self.yes(1),
                recipes_only: self.yes(2),
                multiple: self.yes(3),
                inherits: self.optional(4),
                layer: self.optional(5),
                bare: self.yes(6),
                show_variants: self.yes(7),
                multiconfig: self.optional(8),
            },
            S::ShowAppends => BitBakeLayersOperation::ShowAppends {
                patterns: self.words(0),
                multiconfig: self.optional(1),
            },
            S::ShowCrossDepends => BitBakeLayersOperation::ShowCrossDepends {
                filenames: self.yes(0),
                ignored_layers: self.optional(1),
            },
            S::AddLayer => BitBakeLayersOperation::AddLayers {
                directories: self.paths(0),
            },
            S::RemoveLayer => BitBakeLayersOperation::RemoveLayers {
                directories: self.paths(0),
            },
            S::Flatten => BitBakeLayersOperation::Flatten {
                layers: self.words(0),
                output_directory: PathBuf::from(self.trimmed(1)),
            },
            S::LayerIndexFetch => BitBakeLayersOperation::LayerIndexFetch {
                layers: self.words(0),
                show_only: self.yes(1),
                branch: self.optional(2),
                shallow: self.yes(3),
                ignored_layers: self.optional(4),
                fetch_directory: self.optional(5).map(PathBuf::from),
            },
            S::LayerIndexShowDepends => BitBakeLayersOperation::LayerIndexShowDepends {
                layers: self.words(0),
                branch: self.optional(1),
            },
            S::CreateLayer => BitBakeLayersOperation::CreateLayer {
                directory: PathBuf::from(self.trimmed(0)),
                add: self.yes(1),
                layer_id: self.optional(2),
                priority: self.optional(3),
                example_recipe: self.optional(4),
                example_version: self.optional(5),
            },
            S::ShowMachines => BitBakeLayersOperation::ShowMachines {
                bare: self.yes(0),
                layer: self.optional(1),
            },
            S::SaveBuildConf => BitBakeLayersOperation::SaveBuildConf {
                layer_path: PathBuf::from(self.trimmed(0)),
                template_name: self.trimmed(1).into(),
            },
            S::CreateLayersSetup => BitBakeLayersOperation::CreateLayersSetup {
                destination: PathBuf::from(self.trimmed(0)),
                output_prefix: self.optional(1),
                writer: self.optional(2),
                json_only: self.yes(3),
                update: self.yes(4),
                custom_references: self.words(5),
            },
        }
    }

    fn trimmed(&self, index: usize) -> &str {
        self.values[index].trim()
    }

    fn optional(&self, index: usize) -> Option<String> {
        let value = self.trimmed(index);
        (!value.is_empty()).then(|| value.into())
    }

    fn words(&self, index: usize) -> Vec<String> {
        self.values[index]
            .split_whitespace()
            .map(str::to_owned)
            .collect()
    }

    fn paths(&self, index: usize) -> Vec<PathBuf> {
        self.words(index).into_iter().map(PathBuf::from).collect()
    }

    fn yes(&self, index: usize) -> bool {
        self.values[index] == "yes"
    }
}

fn field_specs(subcommand: LayerUtilitySubcommand) -> Vec<FieldSpec> {
    use LayerUtilitySubcommand as S;
    match subcommand {
        S::ShowLayers => vec![],
        S::ShowOverlayed => vec![
            FieldSpec::toggle("Show filenames"),
            FieldSpec::toggle("Same version only"),
            FieldSpec::text("Multiconfig (optional)", ""),
        ],
        S::ShowRecipes => vec![
            FieldSpec::text("Recipe patterns (space-separated)", "linux-*"),
            FieldSpec::toggle("Show filenames"),
            FieldSpec::toggle("Recipes only"),
            FieldSpec::toggle("Multiple providers only"),
            FieldSpec::text("Inherited classes (comma-separated)", ""),
            FieldSpec::text("Layer (optional)", ""),
            FieldSpec::toggle("Bare names"),
            FieldSpec::toggle("Show variants"),
            FieldSpec::text("Multiconfig (optional)", ""),
        ],
        S::ShowAppends => vec![
            FieldSpec::text("Recipe patterns (space-separated)", ""),
            FieldSpec::text("Multiconfig (optional)", ""),
        ],
        S::ShowCrossDepends => vec![
            FieldSpec::toggle("Show filenames"),
            FieldSpec::text("Ignored layers (comma-separated)", ""),
        ],
        S::AddLayer => vec![FieldSpec::text(
            "Layer directories (absolute, space-separated)",
            "",
        )],
        S::RemoveLayer => vec![FieldSpec::text(
            "Layer paths/wildcards (absolute, space-separated)",
            "",
        )],
        S::Flatten => vec![
            FieldSpec::text("Layers (space-separated, optional)", ""),
            FieldSpec::text("Output directory (absolute)", ""),
        ],
        S::LayerIndexFetch => vec![
            FieldSpec::text("Layer names (space-separated)", ""),
            FieldSpec::toggle("Show only"),
            FieldSpec::text("Branch (optional)", ""),
            FieldSpec::toggle("Shallow clones"),
            FieldSpec::text("Ignored layers (comma-separated)", ""),
            FieldSpec::text("Fetch directory (absolute, optional)", ""),
        ],
        S::LayerIndexShowDepends => vec![
            FieldSpec::text("Layer names (space-separated)", ""),
            FieldSpec::text("Branch (optional)", ""),
        ],
        S::CreateLayer => vec![
            FieldSpec::text("Layer directory (absolute)", ""),
            FieldSpec::toggle("Add to bblayers.conf"),
            FieldSpec::text("Layer ID (optional)", ""),
            FieldSpec::text("Priority (optional)", ""),
            FieldSpec::text("Example recipe name (optional)", ""),
            FieldSpec::text("Example recipe version (optional)", ""),
        ],
        S::ShowMachines => vec![
            FieldSpec::toggle("Bare names"),
            FieldSpec::text("Layer (optional)", ""),
        ],
        S::SaveBuildConf => vec![
            FieldSpec::text("Layer path (absolute)", ""),
            FieldSpec::text("Template name", ""),
        ],
        S::CreateLayersSetup => vec![
            FieldSpec::text("Destination directory (absolute)", ""),
            FieldSpec::text("Output prefix (optional)", ""),
            FieldSpec::text("Writer (oe-setup-layers, optional)", ""),
            FieldSpec::toggle("JSON only"),
            FieldSpec::toggle("Update existing JSON"),
            FieldSpec::text("Custom refs REPOSITORY:REF (space-separated)", ""),
        ],
    }
}
