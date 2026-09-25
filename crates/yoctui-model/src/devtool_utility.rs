mod arguments;
mod fields;
mod validation;

use crate::{CapabilityId, OperatorActionSafety, YoctoUtilityFieldKind};
use fields::{FieldKind, field_specs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevtoolUtilityCommand {
    Add,
    Modify,
    Upgrade,
    Status,
    LatestVersion,
    CheckUpgradeStatus,
    Search,
    Build,
    IdeSdk,
    Rename,
    EditRecipe,
    FindRecipe,
    ConfigureHelp,
    UpdateRecipe,
    Reset,
    Finish,
    DeployTarget,
    UndeployTarget,
    BuildImage,
    CreateWorkspace,
    Export,
    Extract,
    Sync,
    Import,
    Menuconfig,
}

impl DevtoolUtilityCommand {
    pub const ALL: [Self; 25] = [
        Self::Add,
        Self::Modify,
        Self::Upgrade,
        Self::Status,
        Self::LatestVersion,
        Self::CheckUpgradeStatus,
        Self::Search,
        Self::Build,
        Self::IdeSdk,
        Self::Rename,
        Self::EditRecipe,
        Self::FindRecipe,
        Self::ConfigureHelp,
        Self::UpdateRecipe,
        Self::Reset,
        Self::Finish,
        Self::DeployTarget,
        Self::UndeployTarget,
        Self::BuildImage,
        Self::CreateWorkspace,
        Self::Export,
        Self::Extract,
        Self::Sync,
        Self::Import,
        Self::Menuconfig,
    ];

    pub const fn subcommand(self) -> &'static str {
        match self {
            Self::Add => "add",
            Self::Modify => "modify",
            Self::Upgrade => "upgrade",
            Self::Status => "status",
            Self::LatestVersion => "latest-version",
            Self::CheckUpgradeStatus => "check-upgrade-status",
            Self::Search => "search",
            Self::Build => "build",
            Self::IdeSdk => "ide-sdk",
            Self::Rename => "rename",
            Self::EditRecipe => "edit-recipe",
            Self::FindRecipe => "find-recipe",
            Self::ConfigureHelp => "configure-help",
            Self::UpdateRecipe => "update-recipe",
            Self::Reset => "reset",
            Self::Finish => "finish",
            Self::DeployTarget => "deploy-target",
            Self::UndeployTarget => "undeploy-target",
            Self::BuildImage => "build-image",
            Self::CreateWorkspace => "create-workspace",
            Self::Export => "export",
            Self::Extract => "extract",
            Self::Sync => "sync",
            Self::Import => "import",
            Self::Menuconfig => "menuconfig",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Add => "Add recipe",
            Self::Modify => "Modify recipe",
            Self::Upgrade => "Upgrade recipe",
            Self::Status => "Workspace status",
            Self::LatestVersion => "Latest recipe version",
            Self::CheckUpgradeStatus => "Check upgrade status",
            Self::Search => "Search recipes",
            Self::Build => "Build workspace recipe",
            Self::IdeSdk => "Configure IDE SDK",
            Self::Rename => "Rename workspace recipe",
            Self::EditRecipe => "Edit recipe",
            Self::FindRecipe => "Find recipe file",
            Self::ConfigureHelp => "Configure-script help",
            Self::UpdateRecipe => "Update recipe",
            Self::Reset => "Reset workspace recipes",
            Self::Finish => "Finish recipe",
            Self::DeployTarget => "Deploy to target",
            Self::UndeployTarget => "Undeploy from target",
            Self::BuildImage => "Build workspace image",
            Self::CreateWorkspace => "Create workspace",
            Self::Export => "Export workspace",
            Self::Extract => "Extract recipe source",
            Self::Sync => "Synchronize recipe source",
            Self::Import => "Import workspace",
            Self::Menuconfig => "Recipe menuconfig",
        }
    }

    pub const fn action_id(self) -> &'static str {
        match self {
            Self::Add => "tools.devtool.add",
            Self::Modify => "tools.devtool.modify",
            Self::Upgrade => "tools.devtool.upgrade",
            Self::Status => "tools.devtool.status",
            Self::LatestVersion => "tools.devtool.latest-version",
            Self::CheckUpgradeStatus => "tools.devtool.check-upgrade-status",
            Self::Search => "tools.devtool.search",
            Self::Build => "tools.devtool.build",
            Self::IdeSdk => "tools.devtool.ide-sdk",
            Self::Rename => "tools.devtool.rename",
            Self::EditRecipe => "tools.devtool.edit-recipe",
            Self::FindRecipe => "tools.devtool.find-recipe",
            Self::ConfigureHelp => "tools.devtool.configure-help",
            Self::UpdateRecipe => "tools.devtool.update-recipe",
            Self::Reset => "tools.devtool.reset",
            Self::Finish => "tools.devtool.finish",
            Self::DeployTarget => "tools.devtool.deploy-target",
            Self::UndeployTarget => "tools.devtool.undeploy-target",
            Self::BuildImage => "tools.devtool.build-image",
            Self::CreateWorkspace => "tools.devtool.create-workspace",
            Self::Export => "tools.devtool.export",
            Self::Extract => "tools.devtool.extract",
            Self::Sync => "tools.devtool.sync",
            Self::Import => "tools.devtool.import",
            Self::Menuconfig => "tools.devtool.menuconfig",
        }
    }

    pub const fn description(self) -> &'static str {
        match self {
            Self::Add => "Create a workspace recipe from a source tree or URI",
            Self::Modify => "Extract or attach source for an existing recipe",
            Self::Upgrade => "Upgrade an existing recipe to a new source revision",
            Self::Status => "List recipes currently present in the Devtool workspace",
            Self::LatestVersion => "Query the latest upstream version of a recipe",
            Self::CheckUpgradeStatus => "Report recipes with available upstream upgrades",
            Self::Search => "Search available recipes and package metadata",
            Self::Build => "Build a recipe from the Devtool workspace",
            Self::IdeSdk => "Generate an SDK and IDE configuration for workspace recipes",
            Self::Rename => "Rename a workspace recipe or change its version",
            Self::EditRecipe => "Open a recipe in the configured editor",
            Self::FindRecipe => "Print the authoritative recipe file path",
            Self::ConfigureHelp => "Show configure-script help for a recipe",
            Self::UpdateRecipe => "Write committed source changes back to recipe metadata",
            Self::Reset => "Remove recipes from the Devtool workspace",
            Self::Finish => "Publish changes to a layer and remove the workspace recipe",
            Self::DeployTarget => "Deploy recipe output to a live SSH target",
            Self::UndeployTarget => "Remove previously deployed files from a live target",
            Self::BuildImage => "Build an image containing workspace recipe packages",
            Self::CreateWorkspace => "Create an alternative Devtool workspace layer",
            Self::Export => "Export workspace recipes to a tar archive",
            Self::Extract => "Extract a recipe source tree without adding it to the workspace",
            Self::Sync => "Synchronize a previously extracted recipe source tree",
            Self::Import => "Import a previously exported workspace archive",
            Self::Menuconfig => "Run menuconfig and create a recipe configuration fragment",
        }
    }

    pub const fn section(self) -> &'static str {
        match self {
            Self::Add | Self::Modify | Self::Upgrade => "Begin work",
            Self::Status | Self::LatestVersion | Self::CheckUpgradeStatus | Self::Search => {
                "Information"
            }
            Self::Build
            | Self::IdeSdk
            | Self::Rename
            | Self::EditRecipe
            | Self::FindRecipe
            | Self::ConfigureHelp
            | Self::UpdateRecipe
            | Self::Reset
            | Self::Finish => "Workspace recipe",
            Self::DeployTarget | Self::UndeployTarget | Self::BuildImage => "Target testing",
            Self::CreateWorkspace
            | Self::Export
            | Self::Extract
            | Self::Sync
            | Self::Import
            | Self::Menuconfig => "Advanced",
        }
    }

    pub const fn capability(self) -> CapabilityId {
        match self {
            Self::Add => CapabilityId::DevtoolAdd,
            Self::Modify => CapabilityId::DevtoolModify,
            Self::Upgrade => CapabilityId::DevtoolUpgrade,
            Self::Status => CapabilityId::DevtoolStatus,
            Self::LatestVersion => CapabilityId::DevtoolLatestVersion,
            Self::CheckUpgradeStatus => CapabilityId::DevtoolCheckUpgradeStatus,
            Self::Search => CapabilityId::DevtoolSearch,
            Self::Build => CapabilityId::DevtoolBuild,
            Self::IdeSdk => CapabilityId::DevtoolIdeSdk,
            Self::Rename => CapabilityId::DevtoolRename,
            Self::EditRecipe => CapabilityId::DevtoolEditRecipe,
            Self::FindRecipe => CapabilityId::DevtoolFindRecipe,
            Self::ConfigureHelp => CapabilityId::DevtoolConfigureHelp,
            Self::UpdateRecipe => CapabilityId::DevtoolUpdateRecipe,
            Self::Reset => CapabilityId::DevtoolReset,
            Self::Finish => CapabilityId::DevtoolFinish,
            Self::DeployTarget => CapabilityId::DevtoolDeployTarget,
            Self::UndeployTarget => CapabilityId::DevtoolUndeployTarget,
            Self::BuildImage => CapabilityId::DevtoolBuildImage,
            Self::CreateWorkspace => CapabilityId::DevtoolCreateWorkspace,
            Self::Export => CapabilityId::DevtoolExport,
            Self::Extract => CapabilityId::DevtoolExtract,
            Self::Sync => CapabilityId::DevtoolSync,
            Self::Import => CapabilityId::DevtoolImport,
            Self::Menuconfig => CapabilityId::DevtoolMenuconfig,
        }
    }

    pub const fn safety(self) -> OperatorActionSafety {
        match self {
            Self::Status
            | Self::LatestVersion
            | Self::CheckUpgradeStatus
            | Self::Search
            | Self::FindRecipe
            | Self::ConfigureHelp => OperatorActionSafety::ReadOnly,
            Self::Reset | Self::UndeployTarget => OperatorActionSafety::DestructiveConfirmation,
            _ => OperatorActionSafety::ConfirmationRequired,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevtoolUtilityDraft {
    pub command: DevtoolUtilityCommand,
    values: Vec<String>,
}

impl DevtoolUtilityDraft {
    pub fn new(command: DevtoolUtilityCommand) -> Self {
        let values = field_specs(command)
            .iter()
            .map(|field| field.default.to_owned())
            .collect();
        Self { command, values }
    }

    pub fn fields(&self) -> Vec<(&'static str, String, YoctoUtilityFieldKind)> {
        field_specs(self.command)
            .iter()
            .zip(&self.values)
            .map(|(field, value)| (field.label, value.clone(), field.kind.public_kind()))
            .collect()
    }

    pub fn selected_text_mut(&mut self, selected: usize) -> Option<&mut String> {
        matches!(
            field_specs(self.command).get(selected)?.kind,
            FieldKind::Text
        )
        .then(|| &mut self.values[selected])
    }

    pub fn cycle_choice(&mut self, selected: usize, delta: isize) {
        let Some(FieldKind::Choice(choices)) = field_specs(self.command)
            .get(selected)
            .map(|field| field.kind)
        else {
            return;
        };
        let current = choices
            .iter()
            .position(|choice| *choice == self.values[selected])
            .unwrap_or(0);
        let count = choices.len();
        let next = if delta.is_negative() {
            (current + count - delta.unsigned_abs() % count) % count
        } else {
            (current + delta as usize % count) % count
        };
        self.values[selected] = choices[next].to_owned();
    }

    pub fn arguments(&self) -> Result<Vec<String>, String> {
        arguments::build_arguments(self)
    }

    fn value(&self, index: usize) -> &str {
        self.values[index].trim()
    }
}
