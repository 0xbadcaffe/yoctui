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
