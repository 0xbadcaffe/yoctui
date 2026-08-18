use crate::{CapabilityId, CapabilityReason};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const CAPABILITY_CATALOG_VERSION: u32 = 1;
pub const MAX_CATALOG_REQUIREMENTS: usize = 32;
pub const MAX_CATALOG_PROBES: usize = 32;
pub const MAX_CATALOG_BOUNDARIES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityToolId {
    BitBake,
    BitBakeDiffSigs,
    BitBakeDumpSig,
    BitBakeLayers,
    Devtool,
    OePkgdataUtil,
    OeSelftest,
    Recipetool,
    Resulttool,
    Runqemu,
    Wic,
    YoctoCheckLayer,
}

impl CapabilityToolId {
    pub const fn executable_name(self) -> &'static str {
        match self {
            Self::BitBake => "bitbake",
            Self::BitBakeDiffSigs => "bitbake-diffsigs",
            Self::BitBakeDumpSig => "bitbake-dumpsig",
            Self::BitBakeLayers => "bitbake-layers",
            Self::Devtool => "devtool",
            Self::OePkgdataUtil => "oe-pkgdata-util",
            Self::OeSelftest => "oe-selftest",
            Self::Recipetool => "recipetool",
            Self::Resulttool => "resulttool",
            Self::Runqemu => "runqemu",
            Self::Wic => "wic",
            Self::YoctoCheckLayer => "yocto-check-layer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequirement {
    pub tool: CapabilityToolId,
    pub subcommand: Option<String>,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MetadataRequirement {
    AnyTask { names: Vec<String> },
    Variable { name: String },
    Api { name: String },
    Artifact { kind: String },
    Configuration { name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CapabilityProbeSpec {
    Executable {
        tool: CapabilityToolId,
    },
    CommandVersion {
        tool: CapabilityToolId,
    },
    CommandHelp {
        tool: CapabilityToolId,
        subcommand: Option<String>,
    },
    CommandOption {
        tool: CapabilityToolId,
        subcommand: Option<String>,
        option: String,
    },
    MetadataAnyTask {
        names: Vec<String>,
    },
    MetadataVariable {
        name: String,
    },
    BackendCapability {
        name: String,
    },
    ProtocolCapability {
        name: String,
    },
    Artifact {
        kind: String,
    },
    Configuration {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityImplementationKind {
    BackendApi,
    Command,
    MetadataTask,
    ProcessAdapter,
    Protocol,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityImplementation {
    pub id: String,
    pub kind: CapabilityImplementationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FallbackSelector {
    PositiveProbe { index: usize },
    AvailableCapability { id: CapabilityId },
    VersionInferenceWhenUnprobeable { map_key: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackImplementation {
    pub implementation: CapabilityImplementation,
    pub selector: FallbackSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdvisoryReleaseBoundary {
    pub component: String,
    pub introduced: Option<String>,
    pub removed: Option<String>,
    pub source: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCatalogEntry {
    pub id: CapabilityId,
    pub label: String,
    pub required_tools: Vec<CapabilityToolId>,
    pub required_commands: Vec<CommandRequirement>,
    pub required_metadata: Vec<MetadataRequirement>,
    pub probes: Vec<CapabilityProbeSpec>,
    pub preferred: CapabilityImplementation,
    pub fallback: Option<FallbackImplementation>,
    pub known_release_boundaries: Vec<AdvisoryReleaseBoundary>,
    pub unavailable_reason: CapabilityReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityCatalog {
    pub version: u32,
    pub entries: Vec<CapabilityCatalogEntry>,
}

impl CapabilityCatalog {
    pub fn builtin() -> Self {
        Self {
            version: CAPABILITY_CATALOG_VERSION,
            entries: CapabilityId::ALL.into_iter().map(builtin_entry).collect(),
        }
    }

    pub fn validate(&self) -> Result<(), CapabilityCatalogError> {
        if self.version == 0 {
            return Err(CapabilityCatalogError::InvalidVersion);
        }
        let expected = CapabilityId::ALL.into_iter().collect::<BTreeSet<_>>();
        let mut actual = BTreeSet::new();
        for entry in &self.entries {
            if !actual.insert(entry.id) {
                return Err(CapabilityCatalogError::Duplicate(entry.id));
            }
            validate_entry(entry)?;
        }
        let missing = expected.difference(&actual).copied().collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(CapabilityCatalogError::Missing(missing));
        }
        let unknown = actual.difference(&expected).copied().collect::<Vec<_>>();
        if !unknown.is_empty() {
            return Err(CapabilityCatalogError::Unexpected(unknown));
        }
        Ok(())
    }

    pub fn entry(&self, id: CapabilityId) -> Option<&CapabilityCatalogEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityCatalogError {
    #[error("capability catalog version must be non-zero")]
    InvalidVersion,
    #[error("duplicate capability catalog entry: {0}")]
    Duplicate(CapabilityId),
    #[error("missing capability catalog entries: {0:?}")]
    Missing(Vec<CapabilityId>),
    #[error("unexpected capability catalog entries: {0:?}")]
    Unexpected(Vec<CapabilityId>),
    #[error("invalid capability catalog entry: {0}")]
    InvalidEntry(CapabilityId),
    #[error("fallback selector for {0} is invalid")]
    InvalidFallback(CapabilityId),
}

fn validate_entry(entry: &CapabilityCatalogEntry) -> Result<(), CapabilityCatalogError> {
    if !valid_text(&entry.label)
        || entry.required_tools.len() > MAX_CATALOG_REQUIREMENTS
        || entry.required_commands.len() > MAX_CATALOG_REQUIREMENTS
        || entry.required_metadata.len() > MAX_CATALOG_REQUIREMENTS
        || entry.probes.is_empty()
        || entry.probes.len() > MAX_CATALOG_PROBES
        || entry.known_release_boundaries.len() > MAX_CATALOG_BOUNDARIES
        || !valid_id(&entry.preferred.id)
        || entry
            .required_commands
            .iter()
            .any(|command| !valid_command(command, &entry.required_tools))
        || entry
            .required_metadata
            .iter()
            .any(|requirement| !valid_metadata(requirement))
        || entry
            .probes
            .iter()
            .any(|probe| !valid_probe(probe, &entry.required_tools))
        || entry
            .known_release_boundaries
            .iter()
            .any(|boundary| !valid_boundary(boundary))
    {
        return Err(CapabilityCatalogError::InvalidEntry(entry.id));
    }
    if let Some(fallback) = &entry.fallback
        && (!valid_id(&fallback.implementation.id)
            || fallback.implementation == entry.preferred
            || match &fallback.selector {
                FallbackSelector::PositiveProbe { index } => *index >= entry.probes.len(),
                FallbackSelector::AvailableCapability { id } => *id == entry.id,
                FallbackSelector::VersionInferenceWhenUnprobeable { map_key } => !valid_id(map_key),
            })
    {
        return Err(CapabilityCatalogError::InvalidFallback(entry.id));
    }
    Ok(())
}

fn builtin_entry(id: CapabilityId) -> CapabilityCatalogEntry {
    let (label, tools, commands, metadata, probes, preferred, fallback) = definition(id);
    CapabilityCatalogEntry {
        id,
        label: label.into(),
        required_tools: tools,
        required_commands: commands,
        required_metadata: metadata,
        probes,
        preferred,
        fallback,
        known_release_boundaries: Vec::new(),
        unavailable_reason: CapabilityReason::new(
            "environment.capability_unavailable",
            format!("Connected environment does not expose {label}."),
            Some(format!("Required capability: {}", id.as_str())),
        )
        .expect("built-in capability reason must be valid"),
    }
}

type Definition = (
    &'static str,
    Vec<CapabilityToolId>,
    Vec<CommandRequirement>,
    Vec<MetadataRequirement>,
    Vec<CapabilityProbeSpec>,
    CapabilityImplementation,
    Option<FallbackImplementation>,
);

fn definition(id: CapabilityId) -> Definition {
    use CapabilityId as Id;
    use CapabilityImplementationKind as Kind;
    use CapabilityToolId as Tool;

    let command = |tool, subcommand: Option<&str>, options: &[&str]| CommandRequirement {
        tool,
        subcommand: subcommand.map(str::to_owned),
        options: options.iter().map(|value| (*value).to_owned()).collect(),
    };
    let help = |tool, subcommand: Option<&str>| CapabilityProbeSpec::CommandHelp {
        tool,
        subcommand: subcommand.map(str::to_owned),
    };
    let implementation = |value: &str, kind| CapabilityImplementation {
        id: value.into(),
        kind,
    };
    let executable = |tool| CapabilityProbeSpec::Executable { tool };
    let tool_command = |label, tool, subcommand: Option<&str>, options: &[&str], impl_id| {
        (
            label,
            vec![tool],
            vec![command(tool, subcommand, options)],
            Vec::new(),
            vec![
                executable(tool),
                CapabilityProbeSpec::CommandVersion { tool },
                help(tool, subcommand),
            ],
            implementation(impl_id, Kind::Command),
            None,
        )
    };
    let backend = |label, name: &str, impl_id| {
        (
            label,
            Vec::new(),
            Vec::new(),
            vec![MetadataRequirement::Api { name: name.into() }],
            vec![CapabilityProbeSpec::BackendCapability { name: name.into() }],
            implementation(impl_id, Kind::BackendApi),
            None,
        )
    };
    let backend_with_version_fallback = |label, name: &str, impl_id| {
        let mut value = backend(label, name, impl_id);
        value.6 = Some(FallbackImplementation {
            implementation: implementation("tinfoil.version_correlated", Kind::BackendApi),
            selector: FallbackSelector::VersionInferenceWhenUnprobeable {
                map_key: "bitbake.tinfoil_adapter".into(),
            },
        });
        value
    };
    let task = |label, names: &[&str], impl_id| {
        let names = names
            .iter()
            .map(|value| (*value).to_owned())
            .collect::<Vec<_>>();
        (
            label,
            vec![Tool::BitBake],
            Vec::new(),
            vec![MetadataRequirement::AnyTask {
                names: names.clone(),
            }],
            vec![CapabilityProbeSpec::MetadataAnyTask { names }],
            implementation(impl_id, Kind::MetadataTask),
            None,
        )
    };

    match id {
        Id::BitBakeWorkspaceInspection => backend_with_version_fallback(
            "BitBake workspace inspection",
            "workspace",
            "tinfoil.workspace",
        ),
        Id::BitBakeRecipeInventory => {
            backend_with_version_fallback("BitBake recipe inventory", "recipes", "tinfoil.recipes")
        }
        Id::BitBakeLayerInventory => {
            backend_with_version_fallback("BitBake layer inventory", "layers", "tinfoil.layers")
        }
        Id::BitBakeBuild => {
            backend_with_version_fallback("BitBake build control", "build", "tinfoil.build")
        }
        Id::BitBakeCancellation => {
            backend_with_version_fallback("BitBake cancellation", "cancel", "tinfoil.cancel")
        }
        Id::BitBakeTaskList => {
            backend_with_version_fallback("BitBake task inventory", "tasks", "tinfoil.tasks")
        }
        Id::BitBakeForceTask => tool_command(
            "BitBake force-task execution",
            Tool::BitBake,
            None,
            &["-f", "-c"],
            "bitbake.force_task.argv",
        ),
        Id::BitBakeEnvironmentDump => tool_command(
            "BitBake environment dump",
            Tool::BitBake,
            None,
            &["-e"],
            "bitbake.environment_dump.argv",
        ),
        Id::BitBakeGraphGeneration => tool_command(
            "BitBake graph generation",
            Tool::BitBake,
            None,
            &["-g"],
            "bitbake.graph.argv",
        ),
        Id::BitBakeDependencyGraph => {
            let mut value = backend(
                "BitBake dependency graph",
                "dependency_graph",
                "tinfoil.dependency_graph",
            );
            value.6 = Some(FallbackImplementation {
                implementation: implementation("bitbake.graph.argv", Kind::Command),
                selector: FallbackSelector::AvailableCapability {
                    id: Id::BitBakeGraphGeneration,
                },
            });
            value
        }
        Id::BitBakeGetVar => {
            let mut value = backend("BitBake variable lookup", "getvar", "tinfoil.getvar");
            value.6 = Some(FallbackImplementation {
                implementation: implementation("bitbake.environment_lookup", Kind::Command),
                selector: FallbackSelector::AvailableCapability {
                    id: Id::BitBakeEnvironmentDump,
                },
            });
            value
        }
        Id::BitBakeVariableHistory => {
            let mut value = backend(
                "BitBake variable history",
                "variable_history",
                "tinfoil.variable_history",
            );
            value.6 = Some(FallbackImplementation {
                implementation: implementation("bitbake.environment_history", Kind::Command),
                selector: FallbackSelector::AvailableCapability {
                    id: Id::BitBakeEnvironmentDump,
                },
            });
            value
        }
        Id::BitBakeDiffSigs => tool_command(
            "BitBake signature comparison",
            Tool::BitBakeDiffSigs,
            None,
            &[],
            "bitbake_diffsigs.argv",
        ),
        Id::BitBakeDumpSig => tool_command(
            "BitBake signature dump",
            Tool::BitBakeDumpSig,
            None,
            &[],
            "bitbake_dumpsig.argv",
        ),
        Id::BitBakeServerSocket => backend_with_version_fallback(
            "BitBake server socket",
            "server_socket",
            "bitbake.server_socket",
        ),
        Id::BitBakeNativeEvents => backend_with_version_fallback(
            "BitBake native events",
            "native_events",
            "tinfoil.native_events",
        ),
        Id::DevtoolModify => tool_command(
            "Devtool modify",
            Tool::Devtool,
            Some("modify"),
            &[],
            "devtool.modify.argv",
        ),
        Id::DevtoolFinish => tool_command(
            "Devtool finish",
            Tool::Devtool,
            Some("finish"),
            &[],
            "devtool.finish.argv",
        ),
        Id::DevtoolDeployTarget => tool_command(
            "Devtool deploy-target",
            Tool::Devtool,
            Some("deploy-target"),
            &[],
            "devtool.deploy_target.argv",
        ),
        Id::DevtoolUpgrade => tool_command(
            "Devtool upgrade",
            Tool::Devtool,
            Some("upgrade"),
            &[],
            "devtool.upgrade.argv",
        ),
        Id::RecipetoolCreate => tool_command(
            "Recipetool create",
            Tool::Recipetool,
            Some("create"),
            &[],
            "recipetool.create.argv",
        ),
        Id::RecipetoolAppendFile => tool_command(
            "Recipetool appendfile",
            Tool::Recipetool,
            Some("appendfile"),
            &[],
            "recipetool.appendfile.argv",
        ),
        Id::BitBakeLayersShowLayers => tool_command(
            "bitbake-layers show-layers",
            Tool::BitBakeLayers,
            Some("show-layers"),
            &[],
            "bitbake_layers.show_layers.argv",
        ),
        Id::BitBakeLayersCreateLayer => tool_command(
            "bitbake-layers create-layer",
            Tool::BitBakeLayers,
            Some("create-layer"),
            &[],
            "bitbake_layers.create_layer.argv",
        ),
        Id::PkgDataLookupPackage => tool_command(
            "package-data package lookup",
            Tool::OePkgdataUtil,
            Some("lookup-pkg"),
            &[],
            "pkgdata.lookup_pkg.argv",
        ),
        Id::PkgDataFindPath => tool_command(
            "package-data path lookup",
            Tool::OePkgdataUtil,
            Some("find-path"),
            &[],
            "pkgdata.find_path.argv",
        ),
        Id::WicCreate => tool_command(
            "Wic image creation",
            Tool::Wic,
            Some("create"),
            &[],
            "wic.create.argv",
        ),
        Id::RunQemu => tool_command("runqemu launch", Tool::Runqemu, None, &[], "runqemu.argv"),
        Id::SdkPopulate => task(
            "standard SDK population",
            &["populate_sdk"],
            "bitbake.populate_sdk",
        ),
        Id::SdkExtensible => task(
            "extensible SDK population",
            &["populate_sdk_ext"],
            "bitbake.populate_sdk_ext",
        ),
        Id::CveCheck => task("CVE checking", &["cve_check"], "bitbake.cve_check"),
        Id::SpdxCreate => task(
            "SPDX creation",
            &["create_spdx", "create_recipe_sbom", "create_rootfs_sbom"],
            "bitbake.spdx",
        ),
        Id::YoctoCheckLayer => tool_command(
            "Yocto layer checking",
            Tool::YoctoCheckLayer,
            None,
            &[],
            "yocto_check_layer.argv",
        ),
        Id::ResultTool => tool_command(
            "resulttool operations",
            Tool::Resulttool,
            None,
            &[],
            "resulttool.argv",
        ),
        Id::OeSelftest => tool_command(
            "OpenEmbedded selftest",
            Tool::OeSelftest,
            None,
            &[],
            "oe_selftest.argv",
        ),
        Id::MenuConfig => task("menuconfig", &["menuconfig"], "bitbake.menuconfig"),
        Id::DevShell => task("development shell", &["devshell"], "bitbake.devshell"),
        Id::BuildHistory => task(
            "build history",
            &["buildhistory_get_image_installed"],
            "bitbake.buildhistory",
        ),
        Id::LockedSignatures => task(
            "locked signatures",
            &["locked_sigs"],
            "bitbake.locked_signatures",
        ),
        Id::HashservDiagnostics => {
            let mut value = backend(
                "hash equivalence server diagnostics",
                "hashserv",
                "bitbake.hashserv_diagnostics",
            );
            value.3.push(MetadataRequirement::Variable {
                name: "BB_HASHSERVE".into(),
            });
            value.4.push(CapabilityProbeSpec::MetadataVariable {
                name: "BB_HASHSERVE".into(),
            });
            value
        }
        Id::PrservDiagnostics => {
            let mut value = backend(
                "PR service diagnostics",
                "prserv",
                "bitbake.prserv_diagnostics",
            );
            value.3.push(MetadataRequirement::Variable {
                name: "PRSERV_HOST".into(),
            });
            value.4.push(CapabilityProbeSpec::MetadataVariable {
                name: "PRSERV_HOST".into(),
            });
            value
        }
    }
}

fn valid_command(command: &CommandRequirement, tools: &[CapabilityToolId]) -> bool {
    tools.contains(&command.tool)
        && command.subcommand.as_deref().is_none_or(valid_token)
        && command.options.len() <= MAX_CATALOG_REQUIREMENTS
        && command.options.iter().all(|option| valid_option(option))
}

fn valid_metadata(requirement: &MetadataRequirement) -> bool {
    match requirement {
        MetadataRequirement::AnyTask { names } => {
            !names.is_empty()
                && names.len() <= MAX_CATALOG_REQUIREMENTS
                && names.iter().all(|name| valid_token(name))
        }
        MetadataRequirement::Variable { name }
        | MetadataRequirement::Api { name }
        | MetadataRequirement::Artifact { kind: name }
        | MetadataRequirement::Configuration { name } => valid_token(name),
    }
}

fn valid_probe(probe: &CapabilityProbeSpec, tools: &[CapabilityToolId]) -> bool {
    match probe {
        CapabilityProbeSpec::Executable { tool } | CapabilityProbeSpec::CommandVersion { tool } => {
            tools.contains(tool)
        }
        CapabilityProbeSpec::CommandHelp { tool, subcommand } => {
            tools.contains(tool) && subcommand.as_deref().is_none_or(valid_token)
        }
        CapabilityProbeSpec::CommandOption {
            tool,
            subcommand,
            option,
        } => {
            tools.contains(tool)
                && subcommand.as_deref().is_none_or(valid_token)
                && valid_option(option)
        }
        CapabilityProbeSpec::MetadataAnyTask { names } => {
            !names.is_empty()
                && names.len() <= MAX_CATALOG_REQUIREMENTS
                && names.iter().all(|name| valid_token(name))
        }
        CapabilityProbeSpec::MetadataVariable { name }
        | CapabilityProbeSpec::BackendCapability { name }
        | CapabilityProbeSpec::ProtocolCapability { name }
        | CapabilityProbeSpec::Artifact { kind: name }
        | CapabilityProbeSpec::Configuration { name } => valid_token(name),
    }
}

fn valid_boundary(boundary: &AdvisoryReleaseBoundary) -> bool {
    valid_token(&boundary.component)
        && boundary.introduced.as_deref().is_none_or(valid_text)
        && boundary.removed.as_deref().is_none_or(valid_text)
        && valid_text(&boundary.source)
        && valid_text(&boundary.note)
}

fn valid_id(value: &str) -> bool {
    valid_token(value)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn valid_option(value: &str) -> bool {
    valid_token(value) && value.starts_with('-')
}

fn valid_token(value: &str) -> bool {
    valid_text(value) && !value.chars().any(char::is_whitespace)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1_024
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod catalog {
    use super::*;

    #[test]
    fn catalog_builtin_is_versioned_complete_unique_and_valid() {
        let catalog = CapabilityCatalog::builtin();
        catalog.validate().unwrap();
        assert_eq!(catalog.version, CAPABILITY_CATALOG_VERSION);
        assert_eq!(catalog.entries.len(), CapabilityId::ALL.len());
        for id in CapabilityId::ALL {
            let entry = catalog.entry(id).unwrap();
            assert!(!entry.probes.is_empty());
            assert_eq!(entry.id, id);
            assert_eq!(
                entry.unavailable_reason.requirement.as_deref(),
                Some(format!("Required capability: {}", id.as_str()).as_str())
            );
        }
    }

    #[test]
    fn catalog_records_safe_explicit_fallback_selectors() {
        let catalog = CapabilityCatalog::builtin();
        let graph = catalog.entry(CapabilityId::BitBakeDependencyGraph).unwrap();
        assert!(matches!(
            graph.fallback.as_ref().map(|fallback| &fallback.selector),
            Some(FallbackSelector::AvailableCapability {
                id: CapabilityId::BitBakeGraphGeneration
            })
        ));
        let getvar = catalog.entry(CapabilityId::BitBakeGetVar).unwrap();
        assert!(matches!(
            getvar.fallback.as_ref().map(|fallback| &fallback.selector),
            Some(FallbackSelector::AvailableCapability {
                id: CapabilityId::BitBakeEnvironmentDump
            })
        ));
    }

    #[test]
    fn catalog_rejects_missing_duplicate_unsafe_probe_and_unselected_fallback() {
        let mut missing = CapabilityCatalog::builtin();
        missing.entries.pop();
        assert!(matches!(
            missing.validate(),
            Err(CapabilityCatalogError::Missing(_))
        ));

        let mut duplicate = CapabilityCatalog::builtin();
        duplicate.entries.push(duplicate.entries[0].clone());
        assert!(matches!(
            duplicate.validate(),
            Err(CapabilityCatalogError::Duplicate(_))
        ));

        let mut unsafe_probe = CapabilityCatalog::builtin();
        unsafe_probe.entries[0].probes = vec![CapabilityProbeSpec::CommandHelp {
            tool: CapabilityToolId::Devtool,
            subcommand: Some("bad subcommand".into()),
        }];
        assert_eq!(
            unsafe_probe.validate(),
            Err(CapabilityCatalogError::InvalidEntry(
                unsafe_probe.entries[0].id
            ))
        );

        let mut bad_fallback = CapabilityCatalog::builtin();
        bad_fallback.entries[0].fallback = Some(FallbackImplementation {
            implementation: CapabilityImplementation {
                id: "fallback.test".into(),
                kind: CapabilityImplementationKind::Command,
            },
            selector: FallbackSelector::PositiveProbe { index: usize::MAX },
        });
        assert_eq!(
            bad_fallback.validate(),
            Err(CapabilityCatalogError::InvalidFallback(
                bad_fallback.entries[0].id
            ))
        );
    }

    #[test]
    fn catalog_keeps_release_boundaries_advisory_and_empty_until_evidenced() {
        let catalog = CapabilityCatalog::builtin();
        assert!(
            catalog
                .entries
                .iter()
                .all(|entry| entry.known_release_boundaries.is_empty())
        );
        assert!(
            catalog
                .entries
                .iter()
                .all(|entry| !entry.label.contains("Yocto 5"))
        );
    }
}
