use serde::{Deserialize, Serialize};

use crate::{CapabilityId, DevtoolUtilityCommand, DevtoolUtilityDraft, OperatorActionSafety};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UtilityMenuKind {
    Devtool,
    Recipetool,
    BitBakeLayers,
    BitBakeConfigBuild,
    Pkgdata,
    Core,
    Advanced,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UtilityMenuEntry {
    pub kind: UtilityMenuKind,
    pub operation: String,
    pub capability: Option<CapabilityId>,
    pub typed_fields: Vec<String>,
    pub destructive: bool,
    pub network: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpertArguments {
    pub input: String,
    pub argv: Vec<String>,
    pub validation_error: Option<String>,
}

impl ExpertArguments {
    pub fn parse(&mut self) -> Result<&[String], String> {
        match parse_words(&self.input) {
            Ok(argv) => {
                self.argv = argv;
                self.validation_error = None;
                Ok(&self.argv)
            }
            Err(error) => {
                self.validation_error = Some(error.clone());
                Err(error)
            }
        }
    }
}

pub fn utility_menu_catalog() -> Vec<UtilityMenuEntry> {
    let mut entries = DevtoolUtilityCommand::ALL
        .into_iter()
        .map(|command| UtilityMenuEntry {
            kind: UtilityMenuKind::Devtool,
            operation: command.subcommand().into(),
            capability: Some(command.capability()),
            typed_fields: DevtoolUtilityDraft::new(command)
                .fields()
                .into_iter()
                .map(|(label, _, _)| label.into())
                .collect(),
            destructive: command.safety() == OperatorActionSafety::DestructiveConfirmation,
            network: matches!(
                command,
                DevtoolUtilityCommand::Add
                    | DevtoolUtilityCommand::Upgrade
                    | DevtoolUtilityCommand::LatestVersion
                    | DevtoolUtilityCommand::CheckUpgradeStatus
                    | DevtoolUtilityCommand::DeployTarget
                    | DevtoolUtilityCommand::UndeployTarget
            ),
        })
        .collect::<Vec<_>>();
    let other_entries = vec![
        (
            UtilityMenuKind::Recipetool,
            "create",
            Some(CapabilityId::RecipetoolCreateOutfile),
            true,
            false,
        ),
        (
            UtilityMenuKind::Recipetool,
            "appendfile",
            Some(CapabilityId::RecipetoolAppendFile),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-layers",
            Some(CapabilityId::BitBakeLayersShowLayers),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-recipes",
            Some(CapabilityId::BitBakeLayersShowRecipes),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-overlayed",
            Some(CapabilityId::BitBakeLayersShowOverlayed),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-appends",
            Some(CapabilityId::BitBakeLayersShowAppends),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-cross-depends",
            Some(CapabilityId::BitBakeLayersShowCrossDepends),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "add-layer",
            Some(CapabilityId::BitBakeLayersAddLayer),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "create-layer",
            Some(CapabilityId::BitBakeLayersCreateLayer),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "remove-layer",
            Some(CapabilityId::BitBakeLayersRemoveLayer),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "flatten",
            Some(CapabilityId::BitBakeLayersFlatten),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "layerindex-fetch",
            Some(CapabilityId::BitBakeLayersLayerIndexFetch),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "layerindex-show-depends",
            Some(CapabilityId::BitBakeLayersLayerIndexShowDepends),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "show-machines",
            Some(CapabilityId::BitBakeLayersShowMachines),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "save-build-conf",
            Some(CapabilityId::BitBakeLayersSaveBuildConf),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeLayers,
            "create-layers-setup",
            Some(CapabilityId::BitBakeLayersCreateLayersSetup),
            true,
            false,
        ),
        (
            UtilityMenuKind::Pkgdata,
            "lookup-pkg",
            Some(CapabilityId::PkgDataLookupPackage),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeConfigBuild,
            "list-fragments",
            Some(CapabilityId::BitBakeConfigBuildListFragments),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeConfigBuild,
            "show-fragment",
            Some(CapabilityId::BitBakeConfigBuildShowFragment),
            false,
            false,
        ),
        (
            UtilityMenuKind::BitBakeConfigBuild,
            "enable-fragment",
            Some(CapabilityId::BitBakeConfigBuildEnableFragment),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeConfigBuild,
            "disable-fragment",
            Some(CapabilityId::BitBakeConfigBuildDisableFragment),
            true,
            false,
        ),
        (
            UtilityMenuKind::BitBakeConfigBuild,
            "disable-all-fragments",
            Some(CapabilityId::BitBakeConfigBuildDisableAllFragments),
            true,
            false,
        ),
        (
            UtilityMenuKind::Core,
            "bitbake-target",
            Some(CapabilityId::BitBakeBuild),
            true,
            false,
        ),
        (UtilityMenuKind::Advanced, "expert-argv", None, false, false),
    ];
    entries.extend(other_entries.into_iter().map(
        |(kind, operation, capability, destructive, network)| UtilityMenuEntry {
            kind,
            operation: operation.into(),
            capability,
            typed_fields: Vec::new(),
            destructive,
            network,
        },
    ));
    entries
}

fn parse_words(input: &str) -> Result<Vec<String>, String> {
    if input
        .chars()
        .any(|ch| ch == '\0' || (ch.is_control() && !ch.is_whitespace()))
    {
        return Err("arguments contain control bytes".into());
    }
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escape = false;
    for ch in input.chars() {
        if escape {
            current.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '\'' || ch == '"' {
            if quote == Some(ch) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(ch);
            } else {
                current.push(ch);
            }
            continue;
        }
        if ch.is_whitespace() && quote.is_none() {
            if !current.is_empty() {
                out.push(std::mem::take(&mut current));
            }
        } else {
            current.push(ch);
        }
    }
    if escape || quote.is_some() {
        return Err("unterminated quote or escape".into());
    }
    if !current.is_empty() {
        out.push(current);
    }
    Ok(out)
}

#[cfg(test)]
#[path = "tests/utility_menu/mod.rs"]
mod tests;
