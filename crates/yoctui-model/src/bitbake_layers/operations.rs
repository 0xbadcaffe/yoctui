use std::path::{Path, PathBuf};

use crate::CapabilityId;

mod validation;
pub use validation::BitBakeLayersOperationError;
use validation::{
    validate_directories, validate_directory, validate_optional, validate_optional_csv,
    validate_value, validate_values,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BitBakeLayersOperation {
    ShowLayers,
    ShowOverlayed {
        filenames: bool,
        same_version: bool,
        multiconfig: Option<String>,
    },
    ShowRecipes {
        patterns: Vec<String>,
        filenames: bool,
        recipes_only: bool,
        multiple: bool,
        inherits: Option<String>,
        layer: Option<String>,
        bare: bool,
        show_variants: bool,
        multiconfig: Option<String>,
    },
    ShowAppends {
        patterns: Vec<String>,
        multiconfig: Option<String>,
    },
    ShowCrossDepends {
        filenames: bool,
        ignored_layers: Option<String>,
    },
    AddLayers {
        directories: Vec<PathBuf>,
    },
    RemoveLayers {
        directories: Vec<PathBuf>,
    },
    Flatten {
        layers: Vec<String>,
        output_directory: PathBuf,
    },
    LayerIndexFetch {
        layers: Vec<String>,
        show_only: bool,
        branch: Option<String>,
        shallow: bool,
        ignored_layers: Option<String>,
        fetch_directory: Option<PathBuf>,
    },
    LayerIndexShowDepends {
        layers: Vec<String>,
        branch: Option<String>,
    },
    CreateLayer {
        directory: PathBuf,
        add: bool,
        layer_id: Option<String>,
        priority: Option<String>,
        example_recipe: Option<String>,
        example_version: Option<String>,
    },
    ShowMachines {
        bare: bool,
        layer: Option<String>,
    },
    SaveBuildConf {
        layer_path: PathBuf,
        template_name: String,
    },
    CreateLayersSetup {
        destination: PathBuf,
        output_prefix: Option<String>,
        writer: Option<String>,
        json_only: bool,
        update: bool,
        custom_references: Vec<String>,
    },
}

impl BitBakeLayersOperation {
    pub fn capability(&self) -> CapabilityId {
        match self {
            Self::ShowLayers => CapabilityId::BitBakeLayersShowLayers,
            Self::ShowOverlayed { .. } => CapabilityId::BitBakeLayersShowOverlayed,
            Self::ShowRecipes { .. } => CapabilityId::BitBakeLayersShowRecipes,
            Self::ShowAppends { .. } => CapabilityId::BitBakeLayersShowAppends,
            Self::ShowCrossDepends { .. } => CapabilityId::BitBakeLayersShowCrossDepends,
            Self::AddLayers { .. } => CapabilityId::BitBakeLayersAddLayer,
            Self::RemoveLayers { .. } => CapabilityId::BitBakeLayersRemoveLayer,
            Self::Flatten { .. } => CapabilityId::BitBakeLayersFlatten,
            Self::LayerIndexFetch { .. } => CapabilityId::BitBakeLayersLayerIndexFetch,
            Self::LayerIndexShowDepends { .. } => CapabilityId::BitBakeLayersLayerIndexShowDepends,
            Self::CreateLayer { add: true, .. } => CapabilityId::BitBakeLayersCreateAndAddLayer,
            Self::CreateLayer { .. } => CapabilityId::BitBakeLayersCreateLayer,
            Self::ShowMachines { .. } => CapabilityId::BitBakeLayersShowMachines,
            Self::SaveBuildConf { .. } => CapabilityId::BitBakeLayersSaveBuildConf,
            Self::CreateLayersSetup { .. } => CapabilityId::BitBakeLayersCreateLayersSetup,
        }
    }

    pub fn validate(&self) -> Result<(), BitBakeLayersOperationError> {
        match self {
            Self::ShowLayers => Ok(()),
            Self::ShowOverlayed { multiconfig, .. } => {
                validate_optional(multiconfig, "multiconfig")
            }
            Self::ShowRecipes {
                patterns,
                inherits,
                layer,
                multiconfig,
                ..
            } => {
                validate_values(patterns, "recipe patterns", false)?;
                validate_optional_csv(inherits, "inherited classes")?;
                validate_optional(layer, "layer")?;
                validate_optional(multiconfig, "multiconfig")
            }
            Self::ShowAppends {
                patterns,
                multiconfig,
            } => {
                validate_values(patterns, "recipe patterns", false)?;
                validate_optional(multiconfig, "multiconfig")
            }
            Self::ShowCrossDepends { ignored_layers, .. } => {
                validate_optional_csv(ignored_layers, "ignored layers")
            }
            Self::AddLayers { directories } | Self::RemoveLayers { directories } => {
                validate_directories(directories)
            }
            Self::Flatten {
                layers,
                output_directory,
            } => {
                validate_values(layers, "layers", false)?;
                validate_directory(output_directory)
            }
            Self::LayerIndexFetch {
                layers,
                branch,
                ignored_layers,
                fetch_directory,
                ..
            } => {
                validate_values(layers, "layer names", true)?;
                validate_optional(branch, "branch")?;
                validate_optional_csv(ignored_layers, "ignored layers")?;
                if let Some(directory) = fetch_directory {
                    validate_directory(directory)?;
                }
                Ok(())
            }
            Self::LayerIndexShowDepends { layers, branch } => {
                validate_values(layers, "layer names", true)?;
                validate_optional(branch, "branch")
            }
            Self::CreateLayer {
                directory,
                layer_id,
                priority,
                example_recipe,
                example_version,
                ..
            } => {
                validate_directory(directory)?;
                validate_optional(layer_id, "layer ID")?;
                if priority
                    .as_ref()
                    .is_some_and(|priority| priority.parse::<u32>().is_err())
                {
                    return Err(BitBakeLayersOperationError::InvalidPriority);
                }
                validate_optional(example_recipe, "example recipe")?;
                validate_optional(example_version, "example recipe version")
            }
            Self::ShowMachines { layer, .. } => validate_optional(layer, "layer"),
            Self::SaveBuildConf {
                layer_path,
                template_name,
            } => {
                validate_directory(layer_path)?;
                validate_value(template_name, "template name")
            }
            Self::CreateLayersSetup {
                destination,
                output_prefix,
                writer,
                custom_references,
                ..
            } => {
                validate_directory(destination)?;
                validate_optional(output_prefix, "output prefix")?;
                if writer
                    .as_deref()
                    .is_some_and(|value| value != "oe-setup-layers")
                {
                    return Err(BitBakeLayersOperationError::InvalidValue { field: "writer" });
                }
                for reference in custom_references {
                    validate_value(reference, "custom reference")?;
                    let Some((repository, revision)) = reference.split_once(':') else {
                        return Err(BitBakeLayersOperationError::InvalidCustomReference);
                    };
                    if repository.is_empty() || revision.is_empty() {
                        return Err(BitBakeLayersOperationError::InvalidCustomReference);
                    }
                }
                Ok(())
            }
        }
    }

    pub fn arguments(&self) -> Result<Vec<String>, BitBakeLayersOperationError> {
        self.validate()?;
        Ok(match self {
            Self::ShowLayers => vec!["show-layers".into()],
            Self::ShowOverlayed {
                filenames,
                same_version,
                multiconfig,
            } => options(
                "show-overlayed",
                [(*filenames, "-f"), (*same_version, "-s")],
            )
            .with_option("--mc", multiconfig)
            .finish(),
            Self::ShowRecipes {
                patterns,
                filenames,
                recipes_only,
                multiple,
                inherits,
                layer,
                bare,
                show_variants,
                multiconfig,
            } => options(
                "show-recipes",
                [
                    (*filenames, "-f"),
                    (*recipes_only, "-r"),
                    (*multiple, "-m"),
                    (*bare, "-b"),
                    (*show_variants, "--show-variants"),
                ],
            )
            .with_option("-i", inherits)
            .with_option("-l", layer)
            .with_option("--mc", multiconfig)
            .with_values(patterns)
            .finish(),
            Self::ShowAppends {
                patterns,
                multiconfig,
            } => options("show-appends", [])
                .with_option("--mc", multiconfig)
                .with_values(patterns)
                .finish(),
            Self::ShowCrossDepends {
                filenames,
                ignored_layers,
            } => options("show-cross-depends", [(*filenames, "-f")])
                .with_option("-i", ignored_layers)
                .finish(),
            Self::AddLayers { directories } => path_arguments("add-layer", directories),
            Self::RemoveLayers { directories } => path_arguments("remove-layer", directories),
            Self::Flatten {
                layers,
                output_directory,
            } => options("flatten", [])
                .with_values(layers)
                .with_path(output_directory)
                .finish(),
            Self::LayerIndexFetch {
                layers,
                show_only,
                branch,
                shallow,
                ignored_layers,
                fetch_directory,
            } => options("layerindex-fetch", [(*show_only, "-n"), (*shallow, "-s")])
                .with_option("-b", branch)
                .with_option("-i", ignored_layers)
                .with_optional_path("-f", fetch_directory)
                .with_values(layers)
                .finish(),
            Self::LayerIndexShowDepends { layers, branch } => {
                options("layerindex-show-depends", [])
                    .with_option("-b", branch)
                    .with_values(layers)
                    .finish()
            }
            Self::CreateLayer {
                directory,
                add,
                layer_id,
                priority,
                example_recipe,
                example_version,
            } => options("create-layer", [(*add, "--add-layer")])
                .with_option("--layerid", layer_id)
                .with_option("--priority", priority)
                .with_option("--example-recipe-name", example_recipe)
                .with_option("--example-recipe-version", example_version)
                .with_path(directory)
                .finish(),
            Self::ShowMachines { bare, layer } => options("show-machines", [(*bare, "-b")])
                .with_option("-l", layer)
                .finish(),
            Self::SaveBuildConf {
                layer_path,
                template_name,
            } => options("save-build-conf", [])
                .with_path(layer_path)
                .with_value(template_name)
                .finish(),
            Self::CreateLayersSetup {
                destination,
                output_prefix,
                writer,
                json_only,
                update,
                custom_references,
            } => options(
                "create-layers-setup",
                [(*json_only, "--json-only"), (*update, "--update")],
            )
            .with_option("--output-prefix", output_prefix)
            .with_option("--writer", writer)
            .with_repeated_option("--use-custom-reference", custom_references)
            .with_path(destination)
            .finish(),
        })
    }
}

struct Arguments(Vec<String>);

fn options<const N: usize>(command: &str, flags: [(bool, &str); N]) -> Arguments {
    let mut values = vec![command.into()];
    values.extend(
        flags
            .into_iter()
            .filter(|(set, _)| *set)
            .map(|(_, flag)| flag.into()),
    );
    Arguments(values)
}

impl Arguments {
    fn with_option(mut self, flag: &str, value: &Option<String>) -> Self {
        if let Some(value) = value {
            self.0.extend([flag.into(), value.clone()]);
        }
        self
    }

    fn with_optional_path(mut self, flag: &str, value: &Option<PathBuf>) -> Self {
        if let Some(value) = value {
            self.0.extend([
                flag.into(),
                value
                    .to_str()
                    .expect("validated utility paths are UTF-8")
                    .into(),
            ]);
        }
        self
    }

    fn with_repeated_option(mut self, flag: &str, values: &[String]) -> Self {
        for value in values {
            self.0.extend([flag.into(), value.clone()]);
        }
        self
    }

    fn with_values(mut self, values: &[String]) -> Self {
        self.0.extend(values.iter().cloned());
        self
    }

    fn with_value(mut self, value: &str) -> Self {
        self.0.push(value.into());
        self
    }

    fn with_path(mut self, value: &Path) -> Self {
        self.0.push(
            value
                .to_str()
                .expect("validated utility paths are UTF-8")
                .into(),
        );
        self
    }

    fn finish(self) -> Vec<String> {
        self.0
    }
}

fn path_arguments(command: &str, directories: &[PathBuf]) -> Vec<String> {
    let values = directories
        .iter()
        .map(|path| {
            path.to_str()
                .expect("validated utility paths are UTF-8")
                .into()
        })
        .collect::<Vec<_>>();
    options(command, []).with_values(&values).finish()
}
