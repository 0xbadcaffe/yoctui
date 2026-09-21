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
