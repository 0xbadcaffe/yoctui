use std::{
    collections::{BTreeMap, VecDeque},
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use thiserror::Error;
use yoctui_model::{
    BuildComparisonRequest, GitArchiveRequest, LockedSignatureCacheRequest,
    MAX_MAINTENANCE_ARGUMENTS, MAX_MAINTENANCE_EVIDENCE, MAX_MAINTENANCE_LIMITATIONS,
    MAX_MAINTENANCE_PATHS, MAX_MAINTENANCE_TEXT_BYTES, MaintenanceCapabilitySnapshot,
    MaintenanceEvidence, MaintenanceFileIdentity, MaintenanceMetadata, MaintenanceOperation,
    MaintenanceOperationPreview, MaintenanceSessionId, MaintenanceTool, MaintenanceToolCapability,
    MaintenanceToolInterface,
};

use crate::maintenance_sstate::{
    MaintenanceExternalCommand, MaintenanceFilesystemGuard, MaintenanceSstateAdapterError,
    MaintenanceSstateCommandKind, MaintenanceSstateCommandSpec, guard_directory,
    guard_directory_or_absent, guard_regular_file,
};

const RELEASE_OPERATION_TIMEOUT: Duration = Duration::from_secs(60 * 60);
const MAX_EVIDENCE_SCAN_DIRECTORIES: usize = 4_096;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum MaintenanceReleaseAdapterError {
    #[error("invalid Maintenance release input: {0}")]
    InvalidInput(String),
    #[error("unsafe Maintenance release path: {0}")]
    UnsafePath(PathBuf),
    #[error("Maintenance release capability is unavailable: {0}")]
    Unavailable(String),
    #[error("Maintenance release evidence changed: {0}")]
    StaleEvidence(PathBuf),
    #[error(transparent)]
    Runner(#[from] MaintenanceSstateAdapterError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceReleaseCapabilityInput {
    pub build_dir: PathBuf,
    pub buildhistory_dir: Option<PathBuf>,
    pub native_lsb: Option<String>,
    pub executable_search_path: Vec<PathBuf>,
}

pub struct MaintenanceReleaseCapabilityInspector;

impl MaintenanceReleaseCapabilityInspector {
    pub fn inspect(
        input: MaintenanceReleaseCapabilityInput,
    ) -> Result<MaintenanceCapabilitySnapshot, MaintenanceReleaseAdapterError> {
        let build_dir = canonical_directory(&input.build_dir)?;
        let buildhistory_dir = input
            .buildhistory_dir
            .as_deref()
            .map(canonical_directory)
            .transpose()?;
        let metadata = MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some(build_dir),
            buildhistory_dir,
            native_lsb: input.native_lsb,
            ..MaintenanceMetadata::default()
        })
        .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))?;
        let mut limitations = Vec::new();
        let mut tools = Vec::new();
        for (tool, name) in [
            (MaintenanceTool::LockedSignatureCache, "gen-lockedsig-cache"),
            (MaintenanceTool::BuildHistoryDiff, "buildhistory-diff"),
            (MaintenanceTool::GitArchive, "oe-git-archive"),
        ] {
            tools.push(discover_tool(
                tool,
                name,
                &input.executable_search_path,
                &mut limitations,
            ));
        }
        tools.push(discover_build_compare(
            &input.executable_search_path,
            &mut limitations,
        ));
        MaintenanceCapabilitySnapshot::new(metadata, tools, limitations)
            .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))
    }
}

fn discover_tool(
    tool: MaintenanceTool,
    name: &str,
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> MaintenanceToolCapability {
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
            Ok(executable) => {
                return MaintenanceToolCapability::Available {
                    tool,
                    executable,
                    interface: MaintenanceToolInterface::Native,
                };
            }
            Err(_) if candidate.exists() => push_limitation(
                limitations,
                format!("ignored unsafe executable {}", candidate.display()),
            ),
            Err(_) => {}
        }
    }
    MaintenanceToolCapability::Unavailable {
        tool,
        reason: format!("{name} is unavailable in the configured child search path"),
    }
}

fn discover_build_compare(
    search_path: &[PathBuf],
    limitations: &mut Vec<String>,
) -> MaintenanceToolCapability {
    let discovered = discover_tool(
        MaintenanceTool::BuildCompare,
        "build-compare",
        search_path,
        limitations,
    );
    match discovered {
        MaintenanceToolCapability::Available { executable, .. } => {
            push_limitation(
                limitations,
                format!(
                    "detected {} but its optional interface is not the buildhistory-diff interface",
                    executable.path.display()
                ),
            );
            MaintenanceToolCapability::Unavailable {
                tool: MaintenanceTool::BuildCompare,
                reason: "detected build-compare has no supported typed Yoctui interface".into(),
            }
        }
        unavailable => unavailable,
    }
}

pub fn locked_signature_command(
    session: MaintenanceSessionId,
    capability_request: u64,
    snapshot: &MaintenanceCapabilitySnapshot,
    operation_id: u64,
    request: LockedSignatureCacheRequest,
) -> Result<
    (
        MaintenanceOperationPreview,
        MaintenanceSstateCommandSpec,
        MaintenanceReleaseEvidenceSnapshot,
    ),
    MaintenanceReleaseAdapterError,
> {
    let executable = available_tool(snapshot, MaintenanceTool::LockedSignatureCache)?;
    revalidate_executable(executable, "gen-lockedsig-cache")?;
    let build_dir = snapshot_build_dir(snapshot)?;
    if snapshot.metadata.native_lsb.as_ref() != Some(&request.native_lsb) {
        return Err(MaintenanceReleaseAdapterError::InvalidInput(
            "native LSB identity does not match current metadata".into(),
        ));
    }
    let guards = vec![
        guard_regular_file(&request.locked_signatures)?,
        guard_directory(&request.input_cache)?,
        guard_directory(&request.output_cache)?,
    ];
    let mut guards = guards;
    if let Some(filter) = &request.filter {
        guards.push(guard_regular_file(filter)?);
    }
    let arguments = locked_signature_arguments(&request);
    let preview = preview(
        operation_id,
        capability_request,
        MaintenanceOperation::LockedSignatureCache(request.clone()),
        executable,
        &arguments,
        vec![
            format!("output cache root: {}", request.output_cache.display()),
            "matching destination files may be replaced".into(),
        ],
    )?;
    let before = MaintenanceReleaseEvidenceSnapshot::capture(&request.output_cache)?;
    let command = MaintenanceSstateCommandSpec::external(MaintenanceExternalCommand {
        session,
        kind: MaintenanceSstateCommandKind::LockedSignatureCache,
        executable_identity: executable.clone(),
        expected_executable_name: "gen-lockedsig-cache".into(),
        arguments,
        current_directory: build_dir,
        timeout: RELEASE_OPERATION_TIMEOUT,
        preview: preview.clone(),
        guards,
    })?;
    Ok((preview, command, before))
}

pub fn buildhistory_command(
    session: MaintenanceSessionId,
    capability_request: u64,
    snapshot: &MaintenanceCapabilitySnapshot,
    operation_id: u64,
    request: BuildComparisonRequest,
) -> Result<
    (MaintenanceOperationPreview, MaintenanceSstateCommandSpec),
    MaintenanceReleaseAdapterError,
> {
    let executable = available_tool(snapshot, MaintenanceTool::BuildHistoryDiff)?;
    revalidate_executable(executable, "buildhistory-diff")?;
    let build_dir = snapshot_build_dir(snapshot)?;
    validate_buildhistory_request(snapshot, &request)?;
    let arguments = buildhistory_arguments(&request)?;
    let preview = preview(
        operation_id,
        capability_request,
        MaintenanceOperation::BuildHistoryComparison(request.clone()),
        executable,
        &arguments,
        vec!["comparison output is bounded session evidence".into()],
    )?;
    let command = MaintenanceSstateCommandSpec::external(MaintenanceExternalCommand {
        session,
        kind: MaintenanceSstateCommandKind::BuildHistoryComparison,
        executable_identity: executable.clone(),
        expected_executable_name: "buildhistory-diff".into(),
        arguments,
        current_directory: build_dir,
        timeout: RELEASE_OPERATION_TIMEOUT,
        preview: preview.clone(),
        guards: vec![
            guard_git_repository(&request.repository)?,
            guard_regular_file(&git_head_path(&request.repository)?)?,
        ],
    })?;
    Ok((preview, command))
}

pub fn build_compare_command(
    _session: MaintenanceSessionId,
    snapshot: &MaintenanceCapabilitySnapshot,
    _request: BuildComparisonRequest,
) -> Result<MaintenanceSstateCommandSpec, MaintenanceReleaseAdapterError> {
    match snapshot.capability(MaintenanceTool::BuildCompare) {
        Some(MaintenanceToolCapability::Unavailable { reason, .. }) => {
            Err(MaintenanceReleaseAdapterError::Unavailable(reason.clone()))
        }
        _ => Err(MaintenanceReleaseAdapterError::Unavailable(
            "build-compare has no supported typed interface and is not aliased to buildhistory-diff"
                .into(),
        )),
    }
}

pub fn git_archive_local_command(
    session: MaintenanceSessionId,
    capability_request: u64,
    snapshot: &MaintenanceCapabilitySnapshot,
    operation_id: u64,
    request: &GitArchiveRequest,
) -> Result<
    (MaintenanceOperationPreview, MaintenanceSstateCommandSpec),
    MaintenanceReleaseAdapterError,
> {
    let executable = available_tool(snapshot, MaintenanceTool::GitArchive)?;
    revalidate_executable(executable, "oe-git-archive")?;
    let build_dir = snapshot_build_dir(snapshot)?;
    let mut local_request = request.clone();
    local_request.push_remote = None;
    validate_archive_request(&local_request)?;
    let arguments = git_archive_arguments(&local_request);
    let preview = preview(
        operation_id,
        capability_request,
        MaintenanceOperation::GitArchive(local_request.clone()),
        executable,
        &arguments,
        archive_limitations(request, false),
    )?;
    let command = MaintenanceSstateCommandSpec::external(MaintenanceExternalCommand {
        session,
        kind: MaintenanceSstateCommandKind::GitArchiveLocal,
        executable_identity: executable.clone(),
        expected_executable_name: "oe-git-archive".into(),
        arguments,
        current_directory: build_dir,
        timeout: RELEASE_OPERATION_TIMEOUT,
        preview: preview.clone(),
        guards: archive_guards(&local_request)?,
    })?;
    Ok((preview, command))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitArchiveLocalResult {
    pub git_dir: PathBuf,
    pub head: MaintenanceFileIdentity,
}

impl GitArchiveLocalResult {
    pub fn capture(request: &GitArchiveRequest) -> Result<Self, MaintenanceReleaseAdapterError> {
        let head_path = git_head_path(&request.git_dir)?;
        Ok(Self {
            git_dir: canonical_directory(&request.git_dir)?,
            head: regular_file_identity(&head_path)?,
        })
    }

    fn revalidate(&self) -> Result<(), MaintenanceReleaseAdapterError> {
        if canonical_directory(&self.git_dir)? != self.git_dir
            || regular_file_identity(&self.head.path)? != self.head
        {
            return Err(MaintenanceReleaseAdapterError::StaleEvidence(
                self.git_dir.clone(),
            ));
        }
        Ok(())
    }
}

pub fn git_archive_push_command(
    session: MaintenanceSessionId,
    capability_request: u64,
    snapshot: &MaintenanceCapabilitySnapshot,
    operation_id: u64,
    request: GitArchiveRequest,
    local_result: &GitArchiveLocalResult,
) -> Result<
    (MaintenanceOperationPreview, MaintenanceSstateCommandSpec),
    MaintenanceReleaseAdapterError,
> {
    let remote = request.push_remote.as_ref().ok_or_else(|| {
        MaintenanceReleaseAdapterError::InvalidInput("archive push remote is absent".into())
    })?;
    local_result.revalidate()?;
    if local_result.git_dir != request.git_dir {
        return Err(MaintenanceReleaseAdapterError::StaleEvidence(
            request.git_dir.clone(),
        ));
    }
    let executable = available_tool(snapshot, MaintenanceTool::GitArchive)?;
    revalidate_executable(executable, "oe-git-archive")?;
    let build_dir = snapshot_build_dir(snapshot)?;
    validate_archive_request(&request)?;
    let arguments = git_archive_arguments(&request);
    let preview = preview(
        operation_id,
        capability_request,
        MaintenanceOperation::GitArchive(request.clone()),
        executable,
        &arguments,
        {
            let mut limitations = archive_limitations(&request, true);
            limitations.push(format!("remote push: {remote}"));
            limitations.push(
                "network push is permitted only after the retained local archive result".into(),
            );
            limitations
        },
    )?;
    let mut guards = archive_guards(&request)?;
    guards.push(guard_regular_file(&local_result.head.path)?);
    let command = MaintenanceSstateCommandSpec::external(MaintenanceExternalCommand {
        session,
        kind: MaintenanceSstateCommandKind::GitArchivePush,
        executable_identity: executable.clone(),
        expected_executable_name: "oe-git-archive".into(),
        arguments,
        current_directory: build_dir,
        timeout: RELEASE_OPERATION_TIMEOUT,
        preview: preview.clone(),
        guards,
    })?;
    Ok((preview, command))
}

fn locked_signature_arguments(request: &LockedSignatureCacheRequest) -> Vec<OsString> {
    let mut arguments = vec![
        request.locked_signatures.as_os_str().to_owned(),
        request.input_cache.as_os_str().to_owned(),
        request.output_cache.as_os_str().to_owned(),
        request.native_lsb.clone().into(),
    ];
    if let Some(filter) = &request.filter {
        arguments.push(filter.as_os_str().to_owned());
    }
    arguments
}

fn buildhistory_arguments(
    request: &BuildComparisonRequest,
) -> Result<Vec<OsString>, MaintenanceReleaseAdapterError> {
    if request.from_revision.is_none() && request.to_revision.is_some() {
        return Err(MaintenanceReleaseAdapterError::InvalidInput(
            "a to-revision requires a from-revision".into(),
        ));
    }
    let mut arguments = vec!["-p".into(), request.repository.as_os_str().to_owned()];
    if request.report_version {
        arguments.push("-v".into());
    }
    if request.report_all {
        arguments.push("-a".into());
    }
    if request.signatures {
        arguments.push("-s".into());
    }
    if request.signature_diff {
        arguments.push("-S".into());
    }
    for excluded in &request.exclude_paths {
        arguments.push("-e".into());
        arguments.push(excluded.into());
    }
    if request.no_colour {
        arguments.extend(["-c".into(), "no".into()]);
    }
    for revision in [
        request.from_revision.as_deref(),
        request.to_revision.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_revision(revision)?;
        arguments.push(revision.into());
    }
    Ok(arguments)
}

fn git_archive_arguments(request: &GitArchiveRequest) -> Vec<OsString> {
    let mut arguments = vec!["--git-dir".into(), request.git_dir.as_os_str().to_owned()];
    if !request.create {
        arguments.push("--no-create".into());
    }
    if request.bare {
        arguments.push("--bare".into());
    }
    if let Some(remote) = &request.push_remote {
        arguments.extend(["--push".into(), remote.into()]);
    }
    arguments.extend(["--branch-name".into(), request.branch_name.as_str().into()]);
    if request.create_tag {
        if let Some(tag) = &request.tag_name {
            arguments.extend(["--tag-name".into(), tag.into()]);
        }
    } else {
        arguments.push("--no-tag".into());
    }
    arguments.extend([
        "--commit-msg-subject".into(),
        request.commit_subject.as_str().into(),
        "--commit-msg-body".into(),
        request.commit_body.as_str().into(),
        "--tag-msg-subject".into(),
        request.tag_subject.as_str().into(),
        "--tag-msg-body".into(),
        request.tag_body.as_str().into(),
    ]);
    for exclusion in &request.exclusions {
        arguments.extend(["--exclude".into(), exclusion.into()]);
    }
    for (reference, file) in &request.notes {
        arguments.extend([
            "--notes".into(),
            reference.into(),
            file.as_os_str().to_owned(),
        ]);
    }
    arguments.push(request.data_dir.as_os_str().to_owned());
    arguments
}

fn archive_guards(
    request: &GitArchiveRequest,
) -> Result<Vec<MaintenanceFilesystemGuard>, MaintenanceReleaseAdapterError> {
    let mut guards = vec![guard_directory(&request.data_dir)?];
    if request.create {
        guards.push(guard_directory_or_absent(&request.git_dir)?);
    } else {
        guards.push(guard_git_repository(&request.git_dir)?);
    }
    for (_, file) in &request.notes {
        guards.push(guard_regular_file(file)?);
    }
    Ok(guards)
}

fn archive_limitations(request: &GitArchiveRequest, network: bool) -> Vec<String> {
    let mut limitations = Vec::new();
    if request.create {
        limitations.push("the exact repository may be created".into());
    }
    if request.create_tag {
        limitations.push("tag creation or replacement risk requires confirmation".into());
    }
    if network {
        limitations.push("the command includes a separately confirmed network push".into());
    } else if request.push_remote.is_some() {
        limitations.push("remote push is deferred until the local archive succeeds".into());
    }
    limitations
}

fn validate_archive_request(
    request: &GitArchiveRequest,
) -> Result<(), MaintenanceReleaseAdapterError> {
    canonical_directory(&request.data_dir)?;
    if request.create {
        guard_directory_or_absent(&request.git_dir)?;
    } else {
        guard_git_repository(&request.git_dir)?;
    }
    for (_, path) in &request.notes {
        regular_file_identity(path)?;
    }
    Ok(())
}

fn validate_buildhistory_request(
    snapshot: &MaintenanceCapabilitySnapshot,
    request: &BuildComparisonRequest,
) -> Result<(), MaintenanceReleaseAdapterError> {
    if snapshot.metadata.buildhistory_dir.as_ref() != Some(&request.repository) {
        return Err(MaintenanceReleaseAdapterError::InvalidInput(
            "build-history repository does not match current metadata".into(),
        ));
    }
    guard_git_repository(&request.repository)?;
    for revision in [
        request.from_revision.as_deref(),
        request.to_revision.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        validate_revision(revision)?;
    }
    Ok(())
}

fn validate_revision(revision: &str) -> Result<(), MaintenanceReleaseAdapterError> {
    if revision.is_empty()
        || revision.len() > 256
        || revision.starts_with('-')
        || revision
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(MaintenanceReleaseAdapterError::InvalidInput(
            "Git revision is invalid".into(),
        ));
    }
    Ok(())
}

fn guard_git_repository(
    repository: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceReleaseAdapterError> {
    let repository = canonical_directory(repository)?;
    git_head_path(&repository)?;
    guard_directory(&repository).map_err(Into::into)
}

fn git_head_path(repository: &Path) -> Result<PathBuf, MaintenanceReleaseAdapterError> {
    let worktree_head = repository.join(".git/HEAD");
    let bare_head = repository.join("HEAD");
    if worktree_head.exists() {
        regular_file_identity(&worktree_head)?;
        Ok(worktree_head)
    } else if bare_head.exists() {
        regular_file_identity(&bare_head)?;
        Ok(bare_head)
    } else {
        Err(MaintenanceReleaseAdapterError::UnsafePath(
            repository.into(),
        ))
    }
}

fn preview(
    id: u64,
    capability_request: u64,
    operation: MaintenanceOperation,
    executable: &MaintenanceFileIdentity,
    arguments: &[OsString],
    limitations: Vec<String>,
) -> Result<MaintenanceOperationPreview, MaintenanceReleaseAdapterError> {
    if arguments.len() > MAX_MAINTENANCE_ARGUMENTS {
        return Err(MaintenanceReleaseAdapterError::InvalidInput(
            "release argument count exceeded the limit".into(),
        ));
    }
    let indexed = std::iter::once(format!("0: {}", executable.path.display()))
        .chain(
            arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| format!("{}: {}", index + 1, argument.to_string_lossy())),
        )
        .collect();
    MaintenanceOperationPreview::new(id, capability_request, operation, indexed, limitations)
        .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))
}

fn available_tool(
    snapshot: &MaintenanceCapabilitySnapshot,
    tool: MaintenanceTool,
) -> Result<&MaintenanceFileIdentity, MaintenanceReleaseAdapterError> {
    match snapshot.capability(tool) {
        Some(MaintenanceToolCapability::Available {
            executable,
            interface: MaintenanceToolInterface::Native,
            ..
        }) => Ok(executable),
        Some(MaintenanceToolCapability::Unavailable { reason, .. }) => {
            Err(MaintenanceReleaseAdapterError::Unavailable(reason.clone()))
        }
        _ => Err(MaintenanceReleaseAdapterError::Unavailable(
            "release tool capability is unavailable or unsupported".into(),
        )),
    }
}

fn snapshot_build_dir(
    snapshot: &MaintenanceCapabilitySnapshot,
) -> Result<PathBuf, MaintenanceReleaseAdapterError> {
    snapshot
        .metadata
        .build_dir
        .as_deref()
        .ok_or_else(|| MaintenanceReleaseAdapterError::Unavailable("BUILDDIR is absent".into()))
        .and_then(canonical_directory)
}

fn executable_identity(
    path: &Path,
    expected_name: &str,
) -> Result<MaintenanceFileIdentity, MaintenanceReleaseAdapterError> {
    if path.file_name() != Some(OsStr::new(expected_name)) {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
        }
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))
}

fn revalidate_executable(
    identity: &MaintenanceFileIdentity,
    expected_name: &str,
) -> Result<(), MaintenanceReleaseAdapterError> {
    if executable_identity(&identity.path, expected_name)? != *identity {
        return Err(MaintenanceReleaseAdapterError::StaleEvidence(
            identity.path.clone(),
        ));
    }
    Ok(())
}

fn regular_file_identity(
    path: &Path,
) -> Result<MaintenanceFileIdentity, MaintenanceReleaseAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))
}

fn canonical_directory(path: &Path) -> Result<PathBuf, MaintenanceReleaseAdapterError> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceReleaseAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaintenanceReleaseEvidenceSnapshot {
    pub root: PathBuf,
    pub files: Vec<MaintenanceFileIdentity>,
    pub limitations: Vec<String>,
}

impl MaintenanceReleaseEvidenceSnapshot {
    pub fn capture(root: &Path) -> Result<Self, MaintenanceReleaseAdapterError> {
        let root = canonical_directory(root)?;
        let mut queue = VecDeque::from([root.clone()]);
        let mut files = Vec::new();
        let mut limitations = Vec::new();
        let mut directories = 0usize;
        while let Some(directory) = queue.pop_front() {
            directories += 1;
            if directories > MAX_EVIDENCE_SCAN_DIRECTORIES {
                push_limitation(
                    &mut limitations,
                    "release evidence directory count reached the limit".into(),
                );
                break;
            }
            let mut entries = fs::read_dir(&directory)
                .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(directory.clone()))?
                .take(MAX_MAINTENANCE_PATHS + 1)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(directory.clone()))?;
            entries.sort_by_key(fs::DirEntry::file_name);
            if entries.len() > MAX_MAINTENANCE_PATHS {
                entries.truncate(MAX_MAINTENANCE_PATHS);
                push_limitation(
                    &mut limitations,
                    format!(
                        "entry count reached the limit beneath {}",
                        directory.display()
                    ),
                );
            }
            for entry in entries {
                let path = entry.path();
                let metadata = fs::symlink_metadata(&path)
                    .map_err(|_| MaintenanceReleaseAdapterError::UnsafePath(path.clone()))?;
                if metadata.file_type().is_symlink() {
                    push_limitation(
                        &mut limitations,
                        format!("ignored symlink evidence {}", path.display()),
                    );
                } else if metadata.is_dir() {
                    if directories + queue.len() < MAX_EVIDENCE_SCAN_DIRECTORIES {
                        queue.push_back(path);
                    } else {
                        push_limitation(
                            &mut limitations,
                            "release evidence directory count reached the limit".into(),
                        );
                    }
                } else if metadata.is_file() {
                    files.push(regular_file_identity(&path)?);
                    if files.len() >= MAX_MAINTENANCE_EVIDENCE {
                        push_limitation(
                            &mut limitations,
                            "release evidence file count reached the limit".into(),
                        );
                        queue.clear();
                        break;
                    }
                }
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(Self {
            root,
            files,
            limitations,
        })
    }

    pub fn changed_evidence(
        &self,
    ) -> Result<Vec<MaintenanceEvidence>, MaintenanceReleaseAdapterError> {
        let after = Self::capture(&self.root)?;
        let before = self
            .files
            .iter()
            .map(|identity| (identity.path.clone(), identity))
            .collect::<BTreeMap<_, _>>();
        after
            .files
            .into_iter()
            .filter(|identity| {
                before
                    .get(&identity.path)
                    .is_none_or(|old| *old != identity)
            })
            .map(|identity| {
                let label = if before.contains_key(&identity.path) {
                    "replaced locked-signature cache evidence"
                } else {
                    "created locked-signature cache evidence"
                };
                MaintenanceEvidence::new(identity, label.into())
                    .map_err(|message| MaintenanceReleaseAdapterError::InvalidInput(message.into()))
            })
            .collect()
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
    use crate::maintenance_sstate::{MaintenanceSstateJobRunner, MaintenanceSstateRunnerEvent};
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-maintenance-release-{name}-{}-{}",
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

    fn executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, body);
    }

    fn git_repository(path: &Path) {
        fs::create_dir_all(path.join(".git")).unwrap();
        fs::write(path.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    }

    fn prepare_fixture(fixture: &TestDirectory, body: &str) {
        for directory in [
            "build",
            "tools",
            "history",
            "input-cache",
            "output-cache",
            "archive-data",
        ] {
            fs::create_dir_all(fixture.join(directory)).unwrap();
        }
        git_repository(&fixture.join("history"));
        fs::write(fixture.join("locked.inc"), "SIGGEN_LOCKEDSIGS = \"x\"\n").unwrap();
        fs::write(fixture.join("filter.txt"), "recipe:task\n").unwrap();
        fs::write(fixture.join("note.txt"), "release note\n").unwrap();
        for name in ["gen-lockedsig-cache", "buildhistory-diff", "oe-git-archive"] {
            executable(&fixture.join(&format!("tools/{name}")), body);
        }
    }

    fn inspect(fixture: &TestDirectory) -> MaintenanceCapabilitySnapshot {
        MaintenanceReleaseCapabilityInspector::inspect(MaintenanceReleaseCapabilityInput {
            build_dir: fixture.join("build"),
            buildhistory_dir: Some(fixture.join("history")),
            native_lsb: Some("ubuntu-24.04".into()),
            executable_search_path: vec![fixture.join("tools")],
        })
        .unwrap()
    }

    fn locked_request(fixture: &TestDirectory) -> LockedSignatureCacheRequest {
        LockedSignatureCacheRequest::new(
            fixture.join("locked.inc"),
            fixture.join("input-cache"),
            fixture.join("output-cache"),
            "ubuntu-24.04".into(),
            Some(fixture.join("filter.txt")),
        )
        .unwrap()
    }

    fn comparison_request(fixture: &TestDirectory) -> BuildComparisonRequest {
        BuildComparisonRequest::new(BuildComparisonRequest {
            repository: fixture.join("history"),
            from_revision: Some("HEAD^".into()),
            to_revision: Some("HEAD".into()),
            report_version: true,
            report_all: true,
            signatures: true,
            signature_diff: true,
            exclude_paths: vec!["images/*".into()],
            no_colour: true,
        })
        .unwrap()
    }

    fn archive_request(fixture: &TestDirectory, remote: Option<&str>) -> GitArchiveRequest {
        GitArchiveRequest::new(GitArchiveRequest {
            data_dir: fixture.join("archive-data"),
            git_dir: fixture.join("archive.git"),
            create: true,
            bare: false,
            create_tag: true,
            branch_name: "release/{machine}".into(),
            tag_name: Some("release/{tag_number}".into()),
            commit_subject: "Release {commit}".into(),
            commit_body: "machine: {machine}".into(),
            tag_subject: "Release tag {tag_number}".into(),
            tag_body: "archived by Yoctui".into(),
            exclusions: vec!["tmp/*".into()],
            notes: vec![("release".into(), fixture.join("note.txt"))],
            push_remote: remote.map(str::to_owned),
        })
        .unwrap()
    }

    #[test]
    fn maintenance_release_capability_keeps_optional_build_compare_distinct() {
        let fixture = TestDirectory::new("capability");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        executable(&fixture.join("tools/build-compare"), "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        for tool in [
            MaintenanceTool::LockedSignatureCache,
            MaintenanceTool::BuildHistoryDiff,
            MaintenanceTool::GitArchive,
        ] {
            assert!(snapshot.supports(tool));
        }
        assert!(!snapshot.supports(MaintenanceTool::BuildCompare));
        assert!(
            snapshot
                .limitations
                .iter()
                .any(|line| line.contains("not the buildhistory-diff interface"))
        );
        assert!(matches!(
            build_compare_command(
                MaintenanceSessionId(1),
                &snapshot,
                comparison_request(&fixture)
            ),
            Err(MaintenanceReleaseAdapterError::Unavailable(_))
        ));

        #[cfg(unix)]
        {
            let unsafe_fixture = TestDirectory::new("unsafe-capability");
            prepare_fixture(&unsafe_fixture, "#!/bin/sh\nexit 0\n");
            fs::remove_file(unsafe_fixture.join("tools/gen-lockedsig-cache")).unwrap();
            let real = unsafe_fixture.join("real-tool");
            executable(&real, "#!/bin/sh\nexit 0\n");
            symlink(&real, unsafe_fixture.join("tools/gen-lockedsig-cache")).unwrap();
            let snapshot = inspect(&unsafe_fixture);
            assert!(!snapshot.supports(MaintenanceTool::LockedSignatureCache));
            assert!(
                snapshot
                    .limitations
                    .iter()
                    .any(|line| line.contains("unsafe executable"))
            );
        }
    }

    #[test]
    fn maintenance_release_locked_signature_vector_and_changed_evidence_are_exact() {
        let fixture = TestDirectory::new("locked");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = locked_request(&fixture);
        let (preview, command, before) =
            locked_signature_command(MaintenanceSessionId(1), 2, &snapshot, 3, request.clone())
                .unwrap();
        assert_eq!(
            command.kind(),
            MaintenanceSstateCommandKind::LockedSignatureCache
        );
        assert_eq!(
            command.arguments(),
            [
                request.locked_signatures.as_os_str().to_owned(),
                request.input_cache.as_os_str().to_owned(),
                request.output_cache.as_os_str().to_owned(),
                OsString::from("ubuntu-24.04"),
                request.filter.unwrap().as_os_str().to_owned(),
            ]
        );
        assert!(
            preview
                .limitations
                .iter()
                .any(|line| line.contains("may be replaced"))
        );
        let created = fixture.join("output-cache/aa/new.siginfo");
        fs::create_dir_all(created.parent().unwrap()).unwrap();
        fs::write(&created, "sig\n").unwrap();
        let evidence = before.changed_evidence().unwrap();
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].identity.path, created);
    }

    #[tokio::test]
    async fn maintenance_release_locked_signature_revalidates_every_input_before_spawn() {
        let fixture = TestDirectory::new("locked-stale");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let (_, command, _) = locked_signature_command(
            MaintenanceSessionId(1),
            1,
            &snapshot,
            1,
            locked_request(&fixture),
        )
        .unwrap();
        fs::write(fixture.join("locked.inc"), "changed input identity\n").unwrap();
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(path))
                if path == fixture.join("locked.inc")
        ));
    }

    #[test]
    fn maintenance_release_buildhistory_vector_uses_only_documented_flags() {
        let fixture = TestDirectory::new("buildhistory");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = comparison_request(&fixture);
        let (preview, command) =
            buildhistory_command(MaintenanceSessionId(2), 3, &snapshot, 4, request).unwrap();
        assert_eq!(
            command.arguments(),
            [
                OsString::from("-p"),
                fixture.join("history").into_os_string(),
                OsString::from("-v"),
                OsString::from("-a"),
                OsString::from("-s"),
                OsString::from("-S"),
                OsString::from("-e"),
                OsString::from("images/*"),
                OsString::from("-c"),
                OsString::from("no"),
                OsString::from("HEAD^"),
                OsString::from("HEAD"),
            ]
        );
        assert!(matches!(
            preview.operation,
            MaintenanceOperation::BuildHistoryComparison(_)
        ));
        let mut invalid = comparison_request(&fixture);
        invalid.from_revision = Some("--help".into());
        assert!(buildhistory_command(MaintenanceSessionId(3), 3, &snapshot, 5, invalid).is_err());
    }

    #[test]
    fn maintenance_release_archive_separates_local_result_from_network_push() {
        let fixture = TestDirectory::new("archive");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = archive_request(&fixture, Some("origin"));
        let (local_preview, local_command) =
            git_archive_local_command(MaintenanceSessionId(4), 1, &snapshot, 6, &request).unwrap();
        assert_eq!(
            local_command.kind(),
            MaintenanceSstateCommandKind::GitArchiveLocal
        );
        assert!(
            !local_command
                .arguments()
                .iter()
                .any(|argument| argument == "--push")
        );
        assert!(!local_preview.operation.network_side_effect());

        git_repository(&request.git_dir);
        let local_result = GitArchiveLocalResult::capture(&request).unwrap();
        let (push_preview, push_command) = git_archive_push_command(
            MaintenanceSessionId(5),
            1,
            &snapshot,
            7,
            request.clone(),
            &local_result,
        )
        .unwrap();
        assert_eq!(
            push_command.kind(),
            MaintenanceSstateCommandKind::GitArchivePush
        );
        let push_index = push_command
            .arguments()
            .iter()
            .position(|argument| argument == "--push")
            .unwrap();
        assert_eq!(push_command.arguments()[push_index + 1], "origin");
        assert!(push_preview.operation.network_side_effect());
        assert!(
            push_preview
                .limitations
                .iter()
                .any(|line| line.contains("after the retained local archive result"))
        );
    }

    #[tokio::test]
    async fn maintenance_release_archive_push_rejects_changed_local_head() {
        let fixture = TestDirectory::new("archive-stale");
        prepare_fixture(&fixture, "#!/bin/sh\nexit 0\n");
        let snapshot = inspect(&fixture);
        let request = archive_request(&fixture, Some("origin"));
        git_repository(&request.git_dir);
        let local_result = GitArchiveLocalResult::capture(&request).unwrap();
        let (_, command) = git_archive_push_command(
            MaintenanceSessionId(1),
            1,
            &snapshot,
            1,
            request.clone(),
            &local_result,
        )
        .unwrap();
        fs::write(request.git_dir.join(".git/HEAD"), "changed\n").unwrap();
        assert!(matches!(
            MaintenanceSstateJobRunner::new().start(command).await,
            Err(MaintenanceSstateAdapterError::StaleIdentity(_))
        ));
    }

    async fn terminal_event(
        runner: &mut MaintenanceSstateJobRunner,
    ) -> MaintenanceSstateRunnerEvent {
        loop {
            let event = runner.next_event().await.unwrap();
            if matches!(
                event,
                MaintenanceSstateRunnerEvent::Completed { .. }
                    | MaintenanceSstateRunnerEvent::Failed { .. }
                    | MaintenanceSstateRunnerEvent::Cancelled { .. }
                    | MaintenanceSstateRunnerEvent::TimedOut { .. }
                    | MaintenanceSstateRunnerEvent::Lost { .. }
            ) {
                return event;
            }
        }
    }

    #[tokio::test]
    async fn maintenance_release_shared_runner_keeps_success_nonzero_timeout_cancel_and_loss_typed()
    {
        for (name, body, expected_success) in [
            ("runner-success", "#!/bin/sh\necho release\nexit 0\n", true),
            (
                "runner-failure",
                "#!/bin/sh\necho failed >&2\nexit 8\n",
                false,
            ),
        ] {
            let fixture = TestDirectory::new(name);
            prepare_fixture(&fixture, body);
            let snapshot = inspect(&fixture);
            let (_, command) = buildhistory_command(
                MaintenanceSessionId(8),
                1,
                &snapshot,
                1,
                comparison_request(&fixture),
            )
            .unwrap();
            let mut runner = MaintenanceSstateJobRunner::new();
            runner.start(command).await.unwrap();
            assert!(matches!(
                runner.next_event().await.unwrap(),
                MaintenanceSstateRunnerEvent::Started { .. }
            ));
            let terminal = terminal_event(&mut runner).await;
            assert_eq!(
                matches!(terminal, MaintenanceSstateRunnerEvent::Completed { .. }),
                expected_success
            );
            if !expected_success {
                assert!(matches!(
                    terminal,
                    MaintenanceSstateRunnerEvent::Failed {
                        exit_code: Some(8),
                        ..
                    }
                ));
            }
        }

        let forced = TestDirectory::new("runner-timeout");
        prepare_fixture(
            &forced,
            "#!/bin/sh\ntrap '' TERM\nwhile :; do sleep 1; done\n",
        );
        let forced_snapshot = inspect(&forced);
        let (_, forced_command) = buildhistory_command(
            MaintenanceSessionId(9),
            1,
            &forced_snapshot,
            1,
            comparison_request(&forced),
        )
        .unwrap();
        let mut runner = MaintenanceSstateJobRunner::new()
            .with_operation_timeout(Duration::from_millis(200))
            .with_cancellation_timeout(Duration::from_millis(10));
        runner.start(forced_command).await.unwrap();
        assert!(matches!(
            terminal_event(&mut runner).await,
            MaintenanceSstateRunnerEvent::TimedOut { forced: true, .. }
        ));

        let graceful = TestDirectory::new("runner-cancel");
        prepare_fixture(
            &graceful,
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let graceful_snapshot = inspect(&graceful);
        let make_command = || {
            buildhistory_command(
                MaintenanceSessionId(10),
                1,
                &graceful_snapshot,
                1,
                comparison_request(&graceful),
            )
            .unwrap()
            .1
        };
        let mut cancelled =
            MaintenanceSstateJobRunner::new().with_cancellation_timeout(Duration::from_millis(100));
        cancelled.start(make_command()).await.unwrap();
        cancelled.next_event().await.unwrap();
        assert!(cancelled.cancel(MaintenanceSessionId(10)).await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRequested { .. }
        ));
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Cancelled { .. }
        ));
        assert!(!cancelled.cancel(MaintenanceSessionId(10)).await.unwrap());
        assert!(matches!(
            cancelled.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::CancellationRejected { .. }
        ));

        let mut lost = MaintenanceSstateJobRunner::new();
        lost.start(make_command()).await.unwrap();
        lost.next_event().await.unwrap();
        lost.lose_output_channel();
        assert!(matches!(
            lost.next_event().await.unwrap(),
            MaintenanceSstateRunnerEvent::Lost { .. }
        ));
    }
}
