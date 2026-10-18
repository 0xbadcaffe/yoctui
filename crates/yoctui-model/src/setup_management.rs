//! Reviewed operations against a pinned build profile, never an implicit daemon switch.
use crate::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupFragment {
    pub name: String,
    pub summary: String,
    pub enabled: bool,
    pub pattern: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SetupManagementState {
    pub visible: bool,
    pub profile: Option<BuildEnvironmentProfile>,
    pub trusted_profile: Option<BuildEnvironmentProfile>,
    pub fragments: Vec<SetupFragment>,
    pub selected: usize,
    pub generation: u64,
    pub loading: bool,
    pub error: Option<String>,
    pub output: Option<Vec<String>>,
    pub output_title: String,
    pub output_scroll: usize,
    pub last_read: Option<SetupOperationDialog>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetupOperation {
    Fragments,
    Show,
    Enable,
    Disable,
    Status,
    Update,
}

impl SetupOperation {
    pub fn label(self) -> &'static str {
        match self {
            Self::Fragments => "Inspect configuration fragments",
            Self::Show => "Show fragment",
            Self::Enable => "Enable fragment",
            Self::Disable => "Disable fragment",
            Self::Status => "Check setup status",
            Self::Update => "Update setup",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupOperationDialog {
    pub operation: SetupOperation,
    pub profile: BuildEnvironmentProfile,
    pub tool: String,
    pub fragment: String,
    pub editor: Option<TextAreaState>,
    /// 0 prompt, 1 keep (no), 2 apply (yes). These are upstream CLI values.
    pub config_policy: usize,
    pub backup_conflicts: bool,
    pub offline: bool,
    pub preparing: Option<u64>,
    pub error: Option<String>,
}

impl SetupOperationDialog {
    pub fn config_policy(&self) -> &'static str {
        ["prompt", "no", "yes"][self.config_policy.min(2)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupManageAction {
    Fragments,
    Refresh,
    Select(isize),
    Show,
    Enable,
    Disable,
    Custom,
    Status,
    Update,
    Edit,
    Insert(String),
    Motion(TextAreaMotion),
    Backspace,
    Clear,
    AcceptEdit,
    Policy,
    Backup,
    Offline,
    Confirm,
    Cancel,
    Loaded {
        generation: u64,
        result: Result<Vec<SetupFragment>, String>,
    },
    Prepared {
        generation: u64,
        result: Result<TerminalLaunchRequest, String>,
    },
    Completed(BuildEnvironmentProfile),
    Read {
        generation: u64,
        result: Result<String, String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupManageRequest {
    Inspect {
        generation: u64,
        profile: BuildEnvironmentProfile,
    },
    Prepare {
        generation: u64,
        dialog: SetupOperationDialog,
    },
    Read {
        generation: u64,
        dialog: SetupOperationDialog,
    },
}

/// A current replica is required even for inspection: source scripts/tooling
/// execute locally, and an unknown daemon state cannot prove the build idle.
pub fn setup_operation_blocked(app: &App) -> Option<String> {
    if app.daemon.status != ClientReplicaStatus::Current {
        Some("Connect to a current daemon before inspecting/changing a setup.".into())
    } else if app.daemon.jobs.iter().any(|j| !j.lifecycle.is_terminal())
        || app
            .daemon
            .pty_sessions
            .iter()
            .any(|s| !s.lifecycle.is_terminal())
    {
        Some("Wait for daemon jobs and close live terminal sessions before setup operations. Shared source trees may be affected.".into())
    } else {
        None
    }
}

pub fn valid_setup_fragment(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 512
        && name.contains('/')
        && !name.starts_with('-')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-/+.".contains(c))
        && name
            .split('/')
            .all(|part| !matches!(part, "" | "." | ".." | "..."))
}

fn profile(app: &App) -> Option<BuildEnvironmentProfile> {
    if app.bitbake_setup.management.visible {
        return app.bitbake_setup.management.profile.clone();
    }
    if app.bitbake_setup.catalogue {
        return app
            .bitbake_setup
            .entries
            .get(app.bitbake_setup.selected)
            .map(|e| e.profile.clone());
    }
    match &app.build_environment {
        BuildEnvironmentState::Configured(p)
        | BuildEnvironmentState::Connected(p)
        | BuildEnvironmentState::Failed { profile: p, .. } => Some(p.clone()),
        _ => None,
    }
}

fn inspect(app: &mut App, profile: BuildEnvironmentProfile) -> Option<Effect> {
    if app.bitbake_setup.management.trusted_profile.as_ref() != Some(&profile) {
        app.bitbake_setup.management.error =
            Some("Press 3 to review the trusted initializer before inspecting fragments.".into());
        return None;
    }
    if let Some(error) = setup_operation_blocked(app) {
        app.bitbake_setup.management.error = Some(error);
        return None;
    }
    let state = &mut app.bitbake_setup.management;
    state.generation = state.generation.wrapping_add(1);
    state.profile = Some(profile.clone());
    state.output = None;
    state.last_read = None;
    state.loading = true;
    state.error = None;
    Some(Effect::BitBakeSetup(BitBakeSetupRequest::Manage(
        SetupManageRequest::Inspect {
            generation: state.generation,
            profile,
        },
    )))
}

mod reducer;
pub(crate) use reducer::reduce;

#[cfg(test)]
#[path = "tests/setup_management.rs"]
mod tests;
