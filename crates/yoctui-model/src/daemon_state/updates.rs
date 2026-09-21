#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DaemonStateAction {
    ReplaceWorkspace(Workspace),
    ReplaceBuildEnvironment(BuildEnvironmentState),
    ReplaceProjectProfile(ProjectProfileState),
    ReplaceBitBake(DaemonBitBakeState),
    ReplaceCompatibility(Box<DaemonCompatibilitySnapshot>),
    InvalidateCompatibility,
    ReplaceJobs(Box<DaemonJobState>),
    ReplaceRecovery {
        state: DaemonRecoveryState,
        warnings: Vec<String>,
    },
    RecordLog(String),
    RecordError(String),
    RecordTaskHistory(String),
}

pub fn update_daemon_state(
    state: &mut DaemonGlobalState,
    action: DaemonStateAction,
) -> Result<DaemonRevision, DaemonStateError> {
    if let DaemonStateAction::ReplaceCompatibility(compatibility) = &action {
        let normalized = (**compatibility).clone().normalize()?;
        if state
            .compatibility
            .as_ref()
            .is_some_and(|current| current.snapshot.generation >= normalized.snapshot.generation)
        {
            return Err(DaemonStateError::StaleCompatibilityGeneration {
                current: state
                    .compatibility
                    .as_ref()
                    .map(|current| current.snapshot.generation)
                    .unwrap_or(0),
                received: normalized.snapshot.generation,
            });
        }
    }
    state.mutate(|state| match action {
        DaemonStateAction::ReplaceWorkspace(workspace) => state.workspace = workspace,
        DaemonStateAction::ReplaceBuildEnvironment(environment) => {
            state.build_environment = environment;
        }
        DaemonStateAction::ReplaceProjectProfile(profile) => state.project_profile = profile,
        DaemonStateAction::ReplaceBitBake(bitbake) => state.bitbake = bitbake,
        DaemonStateAction::ReplaceCompatibility(compatibility) => {
            state.compatibility = Some(
                (*compatibility)
                    .normalize()
                    .expect("compatibility was validated before daemon mutation"),
            );
        }
        DaemonStateAction::InvalidateCompatibility => state.compatibility = None,
        DaemonStateAction::ReplaceJobs(jobs) => state.jobs = Some(*jobs),
        DaemonStateAction::ReplaceRecovery {
            state: recovery,
            warnings,
        } => {
            state.session.recovery = recovery;
            state.session.recovery_warnings = warnings;
        }
        DaemonStateAction::RecordLog(message) => state.recent_logs.push_back(message),
        DaemonStateAction::RecordError(message) => state.recent_errors.push_back(message),
        DaemonStateAction::RecordTaskHistory(message) => state.task_history.push_back(message),
    })
}
