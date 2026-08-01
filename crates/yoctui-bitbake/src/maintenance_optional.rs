use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use thiserror::Error;
use yoctui_model::{
    MAX_MAINTENANCE_LIMITATIONS, MAX_MAINTENANCE_OUTPUT, MAX_MAINTENANCE_PATHS,
    MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot, MaintenanceFileIdentity,
    MaintenanceIntegrationsSnapshot, MaintenanceMetadata, MaintenanceTool,
    MaintenanceToolCapability, MaintenanceToolInterface, ServiceProcessEvidence,
};
pub use yoctui_model::{
    MaintenanceDirectoryIdentity, MaintenanceGitWorktreeIdentity, OptionalErrorReportIntegration,
    OptionalIntegrationState, OptionalPullRequestIntegration, OptionalRepoManifestIntegration,
    OptionalToasterIntegration,
};

const MAX_PROCESS_ENTRIES: usize = 4_096;
const MAX_PROCESS_BYTES: usize = 512;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaintenanceOptionalAdapterError {
    #[error("invalid Maintenance optional-integration input: {0}")]
    InvalidInput(String),
    #[error("unsafe Maintenance optional-integration path: {0}")]
    UnsafePath(PathBuf),
    #[error("Maintenance optional-integration evidence changed: {0}")]
    StaleEvidence(PathBuf),
    #[error("Maintenance optional-integration process inspection failed: {0}")]
    ProcessInspection(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOptionalCapabilityInput {
    pub build_dir: PathBuf,
    pub executable_search_path: Vec<PathBuf>,
    pub git_worktree_candidates: Vec<PathBuf>,
    pub error_report_candidates: Vec<PathBuf>,
    pub repo_workspace_candidates: Vec<PathBuf>,
    pub toaster_configuration_candidates: Vec<PathBuf>,
    pub process_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceOptionalInspection {
    pub build: MaintenanceDirectoryIdentity,
    pub capability: MaintenanceCapabilitySnapshot,
    pub pull_request: OptionalPullRequestIntegration,
    pub error_report: OptionalErrorReportIntegration,
    pub repo_manifest: OptionalRepoManifestIntegration,
    pub toaster: OptionalToasterIntegration,
    pub limitations: Vec<String>,
}

impl MaintenanceOptionalInspection {
    pub fn integrations_snapshot(
        &self,
    ) -> Result<MaintenanceIntegrationsSnapshot, MaintenanceOptionalAdapterError> {
        MaintenanceIntegrationsSnapshot::new(MaintenanceIntegrationsSnapshot {
            pull_request: self.pull_request.clone(),
            error_report: self.error_report.clone(),
            repo_manifest: self.repo_manifest.clone(),
            toaster: self.toaster.clone(),
            limitations: self.limitations.clone(),
        })
        .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))
    }

    pub fn revalidate(&self) -> Result<(), MaintenanceOptionalAdapterError> {
        revalidate_directory(&self.build)?;
        for capability in &self.capability.tools {
            if let MaintenanceToolCapability::Available { executable, .. } = capability {
                revalidate_file(executable, true)?;
            }
        }
        if let Some(worktree) = &self.pull_request.worktree {
            revalidate_directory(&worktree.root)?;
            revalidate_file(&worktree.head, false)?;
        }
        if let Some(report) = &self.error_report.candidate_report {
            revalidate_file(report, false)?;
        }
        if let Some(workspace) = &self.repo_manifest.workspace {
            revalidate_directory(workspace)?;
        }
        if let Some(executable) = &self.repo_manifest.repo_executable {
            revalidate_file(executable, true)?;
        }
        if let Some(manifest) = &self.repo_manifest.manifest {
            revalidate_file(manifest, false)?;
        }
        for configuration in &self.toaster.configurations {
            revalidate_file(configuration, false)?;
        }
        Ok(())
    }
}

pub struct MaintenanceOptionalCapabilityInspector;

impl MaintenanceOptionalCapabilityInspector {
    pub fn inspect(
        input: MaintenanceOptionalCapabilityInput,
    ) -> Result<MaintenanceOptionalInspection, MaintenanceOptionalAdapterError> {
        let build_dir = directory_identity(&input.build_dir)?;
        let mut limitations = Vec::new();
        note_bound(
            &mut limitations,
            "executable search path",
            input.executable_search_path.len(),
        );
        note_bound(
            &mut limitations,
            "Git worktree candidates",
            input.git_worktree_candidates.len(),
        );
        note_bound(
            &mut limitations,
            "error-report candidates",
            input.error_report_candidates.len(),
        );
        note_bound(
            &mut limitations,
            "repo workspace candidates",
            input.repo_workspace_candidates.len(),
        );
        note_bound(
            &mut limitations,
            "Toaster configuration candidates",
            input.toaster_configuration_candidates.len(),
        );

        let create = discover_executable(
            MaintenanceTool::CreatePullRequest,
            "create-pull-request",
            &input.executable_search_path,
            &mut limitations,
        );
        let send = discover_executable(
            MaintenanceTool::SendPullRequest,
            "send-pull-request",
            &input.executable_search_path,
            &mut limitations,
        );
        let error = discover_executable(
            MaintenanceTool::SendErrorReport,
            "send-error-report",
            &input.executable_search_path,
            &mut limitations,
        );
        let toaster_capability = discover_executable(
            MaintenanceTool::Toaster,
            "toaster",
            &input.executable_search_path,
            &mut limitations,
        );
        let repo_executable =
            discover_named_executable("repo", &input.executable_search_path, &mut limitations);

        let worktree = first_git_worktree(&input.git_worktree_candidates, &mut limitations);
        let report = first_regular_candidate(
            "error report",
            &input.error_report_candidates,
            &mut limitations,
        );
        let (repo_workspace, repo_manifest) =
            first_repo_manifest(&input.repo_workspace_candidates, &mut limitations);
        let configurations = regular_candidates(
            "Toaster configuration",
            &input.toaster_configuration_candidates,
            &mut limitations,
        );
        let (observed_processes, process_limitations) = scan_toaster_processes(&input.process_root)
            .unwrap_or_else(|error| (Vec::new(), vec![error.to_string()]));
        for limitation in &process_limitations {
            push_limitation(&mut limitations, limitation.clone());
        }

        let create_identity = available_identity(&create);
        let send_identity = available_identity(&send);
        let error_identity = available_identity(&error);
        let toaster_identity = available_identity(&toaster_capability);
        let worktree_available = worktree.is_some();
        let report_available = report.is_some();
        let repo_executable_available = repo_executable.is_some();
        let repo_workspace_available = repo_workspace.is_some();
        let repo_manifest_available = repo_manifest.is_some();
        let toaster_configuration_available = !configurations.is_empty();

        let pull_request = OptionalPullRequestIntegration {
            state: integration_state([
                create_identity.is_some(),
                send_identity.is_some(),
                worktree_available,
            ]),
            create_helper: create_identity,
            send_helper: send_identity,
            worktree,
            limitations: missing_limitations(&[
                ("create-pull-request helper", available(&create)),
                ("send-pull-request helper", available(&send)),
                ("canonical Git worktree", worktree_available),
            ]),
        };
        let error_report = OptionalErrorReportIntegration {
            state: integration_state([error_identity.is_some(), report_available]),
            helper: error_identity,
            candidate_report: report,
            limitations: missing_limitations(&[
                ("send-error-report helper", available(&error)),
                ("canonical candidate report", report_available),
            ]),
        };
        let repo_manifest_integration = OptionalRepoManifestIntegration {
            state: integration_state([
                repo_executable_available,
                repo_workspace_available,
                repo_manifest_available,
            ]),
            repo_executable,
            workspace: repo_workspace,
            manifest: repo_manifest,
            limitations: missing_limitations(&[
                ("repo executable", repo_executable_available),
                ("canonical repo workspace", repo_workspace_available),
                ("canonical repo manifest", repo_manifest_available),
            ]),
        };
        let mut toaster_limitations = missing_limitations(&[
            ("Toaster executable", available(&toaster_capability)),
            (
                "canonical Toaster configuration",
                toaster_configuration_available,
            ),
        ]);
        for limitation in process_limitations {
            push_limitation(&mut toaster_limitations, limitation);
        }
        if !observed_processes.is_empty() {
            push_limitation(
                &mut toaster_limitations,
                "Toaster process-name evidence is observational and does not prove service health"
                    .into(),
            );
        }
        let toaster = OptionalToasterIntegration {
            state: integration_state([toaster_identity.is_some(), toaster_configuration_available]),
            executable: toaster_identity,
            configurations,
            observed_processes,
            limitations: toaster_limitations,
        };

        let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some(build_dir.path.clone()),
            ..MaintenanceMetadata::default()
        })
        .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))?;
        let capability = MaintenanceCapabilitySnapshot::new(
            metadata,
            vec![create, send, error, toaster_capability],
            limitations.clone(),
        )
        .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))?;
        let inspection = MaintenanceOptionalInspection {
            build: build_dir,
            capability,
            pull_request,
            error_report,
            repo_manifest: repo_manifest_integration,
            toaster,
            limitations,
        };
        let _ = inspection.integrations_snapshot()?;
        inspection.revalidate()?;
        Ok(inspection)
    }
}

fn integration_state<const N: usize>(parts: [bool; N]) -> OptionalIntegrationState {
    if parts.iter().all(|present| *present) {
        OptionalIntegrationState::Available
    } else if parts.iter().any(|present| *present) {
        OptionalIntegrationState::Partial
    } else {
        OptionalIntegrationState::Unavailable
    }
}

fn missing_limitations(parts: &[(&str, bool)]) -> Vec<String> {
    parts
        .iter()
        .filter(|(_, present)| !present)
        .map(|(label, _)| format!("{label} is unavailable"))
        .collect()
}

fn available(capability: &MaintenanceToolCapability) -> bool {
    matches!(capability, MaintenanceToolCapability::Available { .. })
}

fn available_identity(capability: &MaintenanceToolCapability) -> Option<MaintenanceFileIdentity> {
    match capability {
        MaintenanceToolCapability::Available { executable, .. } => Some(executable.clone()),
        MaintenanceToolCapability::Unavailable { .. } => None,
    }
}

fn discover_executable(
    tool: MaintenanceTool,
    name: &str,
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> MaintenanceToolCapability {
    match discover_named_executable(name, search_path, limitations) {
        Some(executable) => MaintenanceToolCapability::Available {
            tool,
            executable,
            interface: MaintenanceToolInterface::DetectionOnly,
        },
        None => MaintenanceToolCapability::Unavailable {
            tool,
            reason: format!("{name} is unavailable in the configured child search path"),
        },
    }
}

fn discover_named_executable(
    name: &str,
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> Option<MaintenanceFileIdentity> {
    for directory in search_path.iter().take(MAX_MAINTENANCE_PATHS) {
        let Ok(directory) = canonical_directory(directory) else {
            push_limitation(
                limitations,
                format!("ignored unsafe tool directory {}", directory.display()),
            );
            continue;
        };
        let candidate = directory.join(name);
        match executable_identity(&candidate, name) {
            Ok(identity) => return Some(identity),
            Err(_) if candidate.exists() => push_limitation(
                limitations,
                format!("ignored unsafe executable {}", candidate.display()),
            ),
            Err(_) => {}
        }
    }
    None
}

fn first_git_worktree(
    candidates: &[PathBuf],
    limitations: &mut Vec<String>,
) -> Option<MaintenanceGitWorktreeIdentity> {
    for candidate in candidates.iter().take(MAX_MAINTENANCE_PATHS) {
        let result: Result<MaintenanceGitWorktreeIdentity, MaintenanceOptionalAdapterError> =
            (|| {
                let root = directory_identity(candidate)?;
                let git = canonical_directory(&root.path.join(".git"))?;
                let head = regular_file_identity(&git.join("HEAD"))?;
                Ok(MaintenanceGitWorktreeIdentity { root, head })
            })();
        match result {
            Ok(identity) => return Some(identity),
            Err(_) => push_limitation(
                limitations,
                format!(
                    "ignored unsafe Git worktree candidate {}",
                    candidate.display()
                ),
            ),
        }
    }
    None
}

fn first_regular_candidate(
    label: &str,
    candidates: &[PathBuf],
    limitations: &mut Vec<String>,
) -> Option<MaintenanceFileIdentity> {
    regular_candidates(label, candidates, limitations)
        .into_iter()
        .next()
}

fn regular_candidates(
    label: &str,
    candidates: &[PathBuf],
    limitations: &mut Vec<String>,
) -> Vec<MaintenanceFileIdentity> {
    let mut identities = Vec::new();
    for candidate in candidates.iter().take(MAX_MAINTENANCE_PATHS) {
        match regular_file_identity(candidate) {
            Ok(identity) => identities.push(identity),
            Err(_) => push_limitation(
                limitations,
                format!("ignored unsafe {label} candidate {}", candidate.display()),
            ),
        }
    }
    identities.sort_by(|left, right| left.path.cmp(&right.path));
    identities.dedup_by(|left, right| left.path == right.path);
    identities.truncate(MAX_MAINTENANCE_PATHS);
    identities
}

fn first_repo_manifest(
    candidates: &[PathBuf],
    limitations: &mut Vec<String>,
) -> (
    Option<MaintenanceDirectoryIdentity>,
    Option<MaintenanceFileIdentity>,
) {
    for candidate in candidates.iter().take(MAX_MAINTENANCE_PATHS) {
        let result: Result<
            (MaintenanceDirectoryIdentity, MaintenanceFileIdentity),
            MaintenanceOptionalAdapterError,
        > = (|| {
            let workspace = directory_identity(candidate)?;
            let repo_dir = canonical_directory(&workspace.path.join(".repo"))?;
            let manifest_path = workspace.path.join(".repo/manifest.xml");
            let canonical_manifest = fs::canonicalize(&manifest_path)
                .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(manifest_path.clone()))?;
            if !canonical_manifest.starts_with(&repo_dir) {
                return Err(MaintenanceOptionalAdapterError::UnsafePath(manifest_path));
            }
            let manifest = regular_file_identity(&canonical_manifest)?;
            Ok((workspace, manifest))
        })();
        match result {
            Ok(identity) => return (Some(identity.0), Some(identity.1)),
            Err(_) => push_limitation(
                limitations,
                format!(
                    "ignored unsafe repo workspace candidate {}",
                    candidate.display()
                ),
            ),
        }
    }
    (None, None)
}

fn scan_toaster_processes(
    process_root: &Path,
) -> Result<(Vec<ServiceProcessEvidence>, Vec<String>), MaintenanceOptionalAdapterError> {
    let process_root = canonical_directory(process_root)?;
    let directory = fs::read_dir(&process_root)
        .map_err(|error| MaintenanceOptionalAdapterError::ProcessInspection(error.to_string()))?;
    let mut entries = directory.take(MAX_PROCESS_ENTRIES + 1).collect::<Vec<_>>();
    let mut limitations = Vec::new();
    if entries.len() > MAX_PROCESS_ENTRIES {
        entries.truncate(MAX_PROCESS_ENTRIES);
        push_limitation(
            &mut limitations,
            "Toaster process inspection reached the entry limit".into(),
        );
    }
    entries.sort_by_key(|entry| entry.as_ref().ok().map(fs::DirEntry::file_name));
    let mut processes = Vec::new();
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                push_limitation(
                    &mut limitations,
                    format!("one process entry could not be inspected: {error}"),
                );
                continue;
            }
        };
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            continue;
        }
        let Some(name) = toaster_process_name(&path) else {
            continue;
        };
        processes
            .push(ServiceProcessEvidence::new(pid, name).map_err(|message| {
                MaintenanceOptionalAdapterError::InvalidInput(message.into())
            })?);
    }
    processes.sort();
    processes.dedup();
    processes.truncate(MAX_MAINTENANCE_OUTPUT);
    Ok((processes, limitations))
}

fn toaster_process_name(process_path: &Path) -> Option<String> {
    let comm = read_limited(&process_path.join("comm"), MAX_PROCESS_BYTES).ok()?;
    let comm = String::from_utf8_lossy(&comm).trim().to_string();
    if matches!(comm.as_str(), "toaster" | "toaster-eventreplay") {
        return Some(comm);
    }
    let command = read_limited(&process_path.join("cmdline"), MAX_PROCESS_BYTES).ok()?;
    command
        .split(|byte| *byte == 0)
        .filter_map(|argument| std::str::from_utf8(argument).ok())
        .filter_map(|argument| Path::new(argument).file_name()?.to_str())
        .find(|name| matches!(*name, "toaster" | "toaster-eventreplay"))
        .map(str::to_owned)
}

fn read_limited(path: &Path, limit: usize) -> Result<Vec<u8>, std::io::Error> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "process evidence is not a regular non-symlink file",
        ));
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take((limit + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "process evidence exceeded the byte limit",
        ));
    }
    Ok(bytes)
}

fn executable_identity(
    path: &Path,
    expected_name: &str,
) -> Result<MaintenanceFileIdentity, MaintenanceOptionalAdapterError> {
    if path.file_name().and_then(|name| name.to_str()) != Some(expected_name) {
        return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
    }
    let identity = regular_file_identity(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let metadata = fs::metadata(path)
            .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
        }
    }
    Ok(identity)
}

fn regular_file_identity(
    path: &Path,
) -> Result<MaintenanceFileIdentity, MaintenanceOptionalAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|message| MaintenanceOptionalAdapterError::InvalidInput(message.into()))
}

fn directory_identity(
    path: &Path,
) -> Result<MaintenanceDirectoryIdentity, MaintenanceOptionalAdapterError> {
    let canonical = canonical_directory(path)?;
    let metadata = fs::metadata(&canonical)
        .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?;
    Ok(MaintenanceDirectoryIdentity {
        path: canonical,
        modified_at: metadata
            .modified()
            .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?,
    })
}

fn canonical_directory(path: &Path) -> Result<PathBuf, MaintenanceOptionalAdapterError> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceOptionalAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceOptionalAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

fn revalidate_file(
    identity: &MaintenanceFileIdentity,
    executable: bool,
) -> Result<(), MaintenanceOptionalAdapterError> {
    let current = if executable {
        let name = identity
            .path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| MaintenanceOptionalAdapterError::StaleEvidence(identity.path.clone()))?;
        executable_identity(&identity.path, name)
    } else {
        regular_file_identity(&identity.path)
    }
    .map_err(|_| MaintenanceOptionalAdapterError::StaleEvidence(identity.path.clone()))?;
    if &current != identity {
        return Err(MaintenanceOptionalAdapterError::StaleEvidence(
            identity.path.clone(),
        ));
    }
    Ok(())
}

fn revalidate_directory(
    identity: &MaintenanceDirectoryIdentity,
) -> Result<(), MaintenanceOptionalAdapterError> {
    let current = directory_identity(&identity.path)
        .map_err(|_| MaintenanceOptionalAdapterError::StaleEvidence(identity.path.clone()))?;
    if &current != identity {
        return Err(MaintenanceOptionalAdapterError::StaleEvidence(
            identity.path.clone(),
        ));
    }
    Ok(())
}

fn note_bound(limitations: &mut Vec<String>, label: &str, count: usize) {
    if count > MAX_MAINTENANCE_PATHS {
        push_limitation(
            limitations,
            format!("{label} reached the {}-record limit", MAX_MAINTENANCE_PATHS),
        );
    }
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitation.is_empty()
        || limitation.len() > MAX_MAINTENANCE_TEXT_BYTES
        || limitations.len() >= MAX_MAINTENANCE_LIMITATIONS
        || limitations.contains(&limitation)
    {
        return;
    }
    limitations.push(limitation);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-maintenance-optional-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(fs::canonicalize(path).unwrap())
        }

        fn join(&self, path: &str) -> PathBuf {
            self.0.join(path)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn executable(path: &Path) {
        fs::write(path, "#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn process(root: &Path, pid: u32, comm: &str, cmdline: &[u8]) {
        let directory = root.join(pid.to_string());
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("comm"), format!("{comm}\n")).unwrap();
        fs::write(directory.join("cmdline"), cmdline).unwrap();
    }

    fn complete_fixture(fixture: &TestDirectory) -> MaintenanceOptionalCapabilityInput {
        for directory in [
            "build",
            "tools",
            "work/.git",
            "repo/.repo",
            "proc",
            "config",
        ] {
            fs::create_dir_all(fixture.join(directory)).unwrap();
        }
        for tool in [
            "create-pull-request",
            "send-pull-request",
            "send-error-report",
            "toaster",
            "repo",
        ] {
            executable(&fixture.join(&format!("tools/{tool}")));
        }
        fs::write(fixture.join("work/.git/HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(fixture.join("report.json"), "{}\n").unwrap();
        fs::write(fixture.join("repo/.repo/manifest.xml"), "<manifest/>\n").unwrap();
        fs::write(fixture.join("config/toaster.conf"), "setting=true\n").unwrap();
        process(
            &fixture.join("proc"),
            42,
            "sh",
            b"/bin/sh\0/tools/toaster\0start\0",
        );
        MaintenanceOptionalCapabilityInput {
            build_dir: fixture.join("build"),
            executable_search_path: vec![fixture.join("tools")],
            git_worktree_candidates: vec![fixture.join("work")],
            error_report_candidates: vec![fixture.join("report.json")],
            repo_workspace_candidates: vec![fixture.join("repo")],
            toaster_configuration_candidates: vec![fixture.join("config/toaster.conf")],
            process_root: fixture.join("proc"),
        }
    }

    #[test]
    fn maintenance_optional_detects_complete_capabilities_without_side_effects() {
        let fixture = TestDirectory::new("complete");
        let inspection =
            MaintenanceOptionalCapabilityInspector::inspect(complete_fixture(&fixture)).unwrap();
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(
            inspection.error_report.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(
            inspection.toaster.state,
            OptionalIntegrationState::Available
        );
        assert_eq!(inspection.toaster.observed_processes.len(), 1);
        assert_eq!(inspection.toaster.observed_processes[0].pid, 42);
        assert!(
            inspection
                .toaster
                .limitations
                .iter()
                .any(|value| value.contains("observational"))
        );
        assert!(
            inspection
                .capability
                .tools
                .iter()
                .all(|capability| matches!(
                    capability,
                    MaintenanceToolCapability::Available {
                        interface: MaintenanceToolInterface::DetectionOnly,
                        ..
                    }
                ))
        );
        inspection.revalidate().unwrap();
    }

    #[test]
    fn maintenance_optional_preserves_missing_and_partial_states() {
        let fixture = TestDirectory::new("partial");
        let mut input = complete_fixture(&fixture);
        fs::remove_file(fixture.join("tools/send-pull-request")).unwrap();
        fs::remove_file(fixture.join("report.json")).unwrap();
        fs::remove_file(fixture.join("tools/repo")).unwrap();
        fs::remove_file(fixture.join("config/toaster.conf")).unwrap();
        input.toaster_configuration_candidates.clear();
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(
            inspection.error_report.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(inspection.toaster.state, OptionalIntegrationState::Partial);
        assert!(
            inspection
                .pull_request
                .limitations
                .iter()
                .any(|value| value.contains("send-pull-request"))
        );
    }

    #[test]
    fn maintenance_optional_reports_fully_unavailable_inputs() {
        let fixture = TestDirectory::new("missing");
        for directory in ["build", "tools", "proc"] {
            fs::create_dir_all(fixture.join(directory)).unwrap();
        }
        let inspection =
            MaintenanceOptionalCapabilityInspector::inspect(MaintenanceOptionalCapabilityInput {
                build_dir: fixture.join("build"),
                executable_search_path: vec![fixture.join("tools")],
                git_worktree_candidates: Vec::new(),
                error_report_candidates: Vec::new(),
                repo_workspace_candidates: Vec::new(),
                toaster_configuration_candidates: Vec::new(),
                process_root: fixture.join("proc"),
            })
            .unwrap();
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Unavailable
        );
        assert_eq!(
            inspection.error_report.state,
            OptionalIntegrationState::Unavailable
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Unavailable
        );
        assert_eq!(
            inspection.toaster.state,
            OptionalIntegrationState::Unavailable
        );
        assert!(
            inspection
                .capability
                .tools
                .iter()
                .all(|capability| matches!(
                    capability,
                    MaintenanceToolCapability::Unavailable { .. }
                ))
        );
    }

    #[cfg(unix)]
    #[test]
    fn maintenance_optional_rejects_symlinked_helpers_and_escaped_manifests() {
        let fixture = TestDirectory::new("unsafe");
        let input = complete_fixture(&fixture);
        fs::rename(
            fixture.join("tools/create-pull-request"),
            fixture.join("real-create"),
        )
        .unwrap();
        symlink(
            fixture.join("real-create"),
            fixture.join("tools/create-pull-request"),
        )
        .unwrap();
        fs::remove_file(fixture.join("repo/.repo/manifest.xml")).unwrap();
        fs::write(fixture.join("outside.xml"), "<manifest/>\n").unwrap();
        symlink(
            fixture.join("outside.xml"),
            fixture.join("repo/.repo/manifest.xml"),
        )
        .unwrap();
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert!(
            !inspection
                .capability
                .supports(MaintenanceTool::CreatePullRequest)
        );
        assert_eq!(
            inspection.pull_request.state,
            OptionalIntegrationState::Partial
        );
        assert_eq!(
            inspection.repo_manifest.state,
            OptionalIntegrationState::Partial
        );
        assert!(
            inspection
                .limitations
                .iter()
                .any(|value| value.contains("unsafe executable"))
        );
        assert!(
            inspection
                .limitations
                .iter()
                .any(|value| value.contains("unsafe repo workspace"))
        );
    }

    #[test]
    fn maintenance_optional_revalidation_rejects_tampered_evidence() {
        let fixture = TestDirectory::new("tampered");
        let inspection =
            MaintenanceOptionalCapabilityInspector::inspect(complete_fixture(&fixture)).unwrap();
        fs::write(fixture.join("report.json"), "{\"changed\":true}\n").unwrap();
        assert!(matches!(
            inspection.revalidate(),
            Err(MaintenanceOptionalAdapterError::StaleEvidence(path))
                if path == fixture.join("report.json")
        ));
    }

    #[test]
    fn maintenance_optional_bounds_candidates_and_process_records() {
        let fixture = TestDirectory::new("bounds");
        let mut input = complete_fixture(&fixture);
        input.git_worktree_candidates = vec![fixture.join("missing"); MAX_MAINTENANCE_PATHS + 1];
        for pid in 1..=(MAX_MAINTENANCE_OUTPUT as u32 + 1) {
            process(&fixture.join("proc"), pid + 100, "toaster", b"toaster\0");
        }
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert_eq!(
            inspection.toaster.observed_processes.len(),
            MAX_MAINTENANCE_OUTPUT
        );
        assert!(
            inspection
                .limitations
                .iter()
                .any(|value| value.contains("Git worktree candidates reached"))
        );
    }

    #[test]
    fn maintenance_optional_process_evidence_is_observational_only() {
        let fixture = TestDirectory::new("process");
        let mut input = complete_fixture(&fixture);
        fs::remove_file(fixture.join("tools/toaster")).unwrap();
        fs::remove_file(fixture.join("config/toaster.conf")).unwrap();
        input.toaster_configuration_candidates.clear();
        let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
        assert_eq!(
            inspection.toaster.state,
            OptionalIntegrationState::Unavailable
        );
        assert!(!inspection.toaster.observed_processes.is_empty());
        assert!(!inspection.capability.supports(MaintenanceTool::Toaster));
    }
}
