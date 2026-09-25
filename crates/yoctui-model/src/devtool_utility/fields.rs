use crate::{DevtoolUtilityCommand as C, YoctoUtilityFieldKind};

#[derive(Clone, Copy)]
pub(super) enum FieldKind {
    Text,
    Choice(&'static [&'static str]),
}

impl FieldKind {
    pub(super) const fn public_kind(self) -> YoctoUtilityFieldKind {
        match self {
            Self::Text => YoctoUtilityFieldKind::Text,
            Self::Choice(_) => YoctoUtilityFieldKind::Choice,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct FieldSpec {
    pub label: &'static str,
    pub kind: FieldKind,
    pub default: &'static str,
}

const NO_YES: &[&str] = &["no", "yes"];
const DEFAULT_SAME_SEPARATE: &[&str] = &["default", "same source", "separate"];
const DEFAULT_FIXED_AUTOREV: &[&str] = &["default", "fixed SRCREV", "AUTOREV"];
const AUTO_PATCH_SRCREV: &[&str] = &["auto", "patch", "srcrev"];

const fn text(label: &'static str) -> FieldSpec {
    FieldSpec {
        label,
        kind: FieldKind::Text,
        default: "",
    }
}

const fn toggle(label: &'static str) -> FieldSpec {
    choice(label, NO_YES, "no")
}

const fn choice(
    label: &'static str,
    choices: &'static [&'static str],
    default: &'static str,
) -> FieldSpec {
    FieldSpec {
        label,
        kind: FieldKind::Choice(choices),
        default,
    }
}

pub(super) fn field_specs(command: C) -> Vec<FieldSpec> {
    match command {
        C::Add => vec![
            text("Recipe name (optional)"),
            text("Source tree (absolute, optional)"),
            text("Fetch URI (optional)"),
            choice(
                "Fetch URI syntax",
                &["positional", "--fetch (deprecated)"],
                "positional",
            ),
            choice("Build directory", DEFAULT_SAME_SEPARATE, "default"),
            toggle("Fetch npm devDependencies"),
            toggle("Do not inherit pypi"),
            text("Recipe version (optional)"),
            toggle("Do not create Git repository"),
            choice("Source revision", DEFAULT_FIXED_AUTOREV, "default"),
            text("SRCREV value (for fixed SRCREV)"),
            text("Source branch (optional)"),
            toggle("Treat source as binary"),
            toggle("Also add native variant"),
            text("Source subdirectory (optional)"),
            toggle("Enable source mirrors"),
            text("Provides alias (optional)"),
        ],
        C::Modify => vec![
            text("Recipe name"),
            text("Source tree (absolute, optional)"),
            toggle("Wildcard bbappend"),
            choice(
                "Source extraction",
                &["default", "extract", "no extract"],
                "default",
            ),
            choice("Build directory", DEFAULT_SAME_SEPARATE, "default"),
            text("Development branch (optional)"),
            toggle("Do not create override branches"),
            toggle("Keep temporary directory"),
            toggle("Enable debug build"),
        ],
        C::Upgrade => vec![
            text("Recipe name"),
            text("Source tree (absolute, optional)"),
            toggle("Stable releases only"),
            text("Version (optional)"),
            text("SRCREV (optional)"),
            text("Source branch (optional)"),
            text("Development branch (optional)"),
            toggle("Do not apply recipe patches"),
            toggle("Do not create override branches"),
            choice("Build directory", DEFAULT_SAME_SEPARATE, "default"),
            toggle("Keep temporary directory"),
            toggle("Keep failed upgrade files"),
        ],
        C::Status => vec![],
        C::LatestVersion => vec![text("Recipe name"), toggle("Stable releases only")],
        C::CheckUpgradeStatus => vec![
            text("Recipes (space-separated, optional)"),
            toggle("Stable releases only"),
            toggle("Show all recipes"),
        ],
        C::Search => vec![text("Search expression")],
        C::Build => vec![
            text("Recipe name"),
            toggle("Disable parallel make"),
            toggle("Clean before build"),
        ],
        C::IdeSdk => vec![
            text("Recipes/images (space-separated)"),
            choice("SDK mode", &["modified", "shared"], "modified"),
            choice("IDE", &["code", "none"], "code"),
            text("Target user@host (optional)"),
            text("GDB server start port (optional)"),
            toggle("Disable SSH host check"),
            text("SSH executable (optional)"),
            text("SSH port (optional)"),
            text("SSH private key (absolute, optional)"),
            toggle("Skip BitBake SDK update"),
            toggle("Continue BitBake after errors"),
            toggle("Do not strip executables"),
            toggle("Dry run"),
            toggle("Show deployment status"),
            toggle("Do not preserve files"),
            toggle("Do not check target space"),
        ],
        C::Rename => vec![
            text("Current recipe name"),
            text("New recipe name (optional)"),
            text("New version (optional)"),
            toggle("Do not rename source tree"),
        ],
        C::EditRecipe | C::FindRecipe => {
            vec![text("Recipe name"), toggle("Accept any recipe")]
        }
        C::ConfigureHelp => vec![
            text("Recipe name"),
            toggle("Disable pager"),
            toggle("Disable explanatory header"),
            text("Configure arguments (space-separated, optional)"),
        ],
        C::UpdateRecipe => vec![
            text("Recipe name"),
            choice("Update mode", AUTO_PATCH_SRCREV, "auto"),
            text("Initial revision (optional)"),
            text("Append layer directory (absolute, optional)"),
            toggle("Wildcard bbappend version"),
            toggle("Do not remove patches"),
            toggle("Do not handle override branches"),
            toggle("Dry run"),
            toggle("Force patch refresh"),
        ],
        C::Reset => vec![
            text("Recipes (space-separated, optional)"),
            toggle("Reset all workspace recipes"),
            toggle("Do not clean sysroot"),
            toggle("Remove source work trees"),
        ],
        C::Finish => vec![
            text("Recipe name"),
            text("Destination layer/path"),
            choice("Update mode", AUTO_PATCH_SRCREV, "auto"),
            text("Initial revision (optional)"),
            toggle("Force with uncommitted changes"),
            toggle("Remove source work tree"),
            toggle("Do not clean sysroot"),
            toggle("Do not handle override branches"),
            toggle("Dry run"),
            toggle("Force patch refresh"),
        ],
        C::DeployTarget => vec![
            text("Recipe name"),
            text("Target user@host[:destdir]"),
            toggle("Disable SSH host check"),
            toggle("Show status"),
            toggle("Dry run"),
            toggle("Do not preserve files"),
            toggle("Do not check target space"),
            text("SSH executable (optional)"),
            text("SSH port (optional)"),
            text("SSH private key (absolute, optional)"),
            choice(
                "Executable stripping",
                &["default", "strip", "no strip"],
                "default",
            ),
        ],
        C::UndeployTarget => vec![
            text("Recipe name (optional with all)"),
            text("Target user@host"),
            toggle("Disable SSH host check"),
            toggle("Show status"),
            toggle("Undeploy all recipes"),
            toggle("Dry run"),
            text("SSH executable (optional)"),
            text("SSH port (optional)"),
            text("SSH private key (absolute, optional)"),
        ],
        C::BuildImage => vec![
            text("Image recipe (optional)"),
            text("Additional packages (comma-separated, optional)"),
        ],
        C::CreateWorkspace => vec![
            text("Workspace layer path (absolute, optional)"),
            text("Layer series (optional)"),
            toggle("Create only; do not alter configuration"),
        ],
        C::Export => vec![
            text("Archive file (absolute, optional)"),
            toggle("Overwrite archive"),
            choice("Recipe filter", &["all", "include", "exclude"], "all"),
            text("Recipes (space-separated for filter)"),
        ],
        C::Extract => vec![
            text("Recipe name"),
            text("Source tree (absolute)"),
            text("Development branch (optional)"),
            toggle("Do not create override branches"),
            toggle("Keep temporary directory"),
        ],
        C::Sync => vec![
            text("Recipe name"),
            text("Source tree (absolute)"),
            text("Development branch (optional)"),
            toggle("Keep temporary directory"),
        ],
        C::Import => vec![text("Archive file (absolute)"), toggle("Overwrite files")],
        C::Menuconfig => vec![text("Recipe/component name")],
    }
}
