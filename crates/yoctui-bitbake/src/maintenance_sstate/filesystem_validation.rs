fn identity_for_candidate(
    path: &Path,
    cache_dir: &Path,
) -> Result<MaintenanceFileIdentity, MaintenanceSstateAdapterError> {
    if !path.starts_with(cache_dir) || path == cache_dir {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    let metadata = safe_metadata(path, false)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if canonical != path || !canonical.starts_with(cache_dir) {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))
}

fn available_tool(
    snapshot: &MaintenanceCapabilitySnapshot,
    tool: MaintenanceTool,
) -> Result<(&MaintenanceFileIdentity, MaintenanceToolInterface), MaintenanceSstateAdapterError> {
    match snapshot.capability(tool) {
        Some(MaintenanceToolCapability::Available {
            executable,
            interface,
            ..
        }) => Ok((executable, *interface)),
        Some(MaintenanceToolCapability::Unavailable { reason, .. }) => {
            Err(MaintenanceSstateAdapterError::Unavailable(reason.clone()))
        }
        None => Err(MaintenanceSstateAdapterError::Unavailable(
            "capability was not inspected".into(),
        )),
    }
}

fn expected_name(tool: MaintenanceToolInterface, path: &Path) -> Option<&str> {
    match tool {
        MaintenanceToolInterface::Native => path.file_name().and_then(OsStr::to_str),
        MaintenanceToolInterface::SstatePython => Some("sstate-cache-management.py"),
        MaintenanceToolInterface::SstateLegacyShell => Some("sstate-cache-management.sh"),
        MaintenanceToolInterface::DetectionOnly => None,
    }
}

fn readiness_arguments(request: &SstateReadinessRequest) -> Vec<String> {
    let mut arguments = Vec::new();
    if let Some(output) = &request.output {
        arguments.extend(["--outfile".into(), output.display().to_string()]);
    }
    if let Some(log) = &request.log {
        arguments.extend(["--log".into(), log.display().to_string()]);
    }
    if request.mode == SstateReadinessMode::SameTmpdir {
        arguments.push("--same-tmpdir".into());
    }
    arguments.extend(request.targets.iter().cloned());
    arguments
}

fn cleanup_arguments(request: &SstateCleanupRequest, preview: bool, execute: bool) -> Vec<String> {
    let mut arguments = vec![
        "--cache-dir".into(),
        request.cache_dir.display().to_string(),
    ];
    for mode in &request.modes {
        match mode {
            SstateCleanupMode::Duplicates => arguments.push("--remove-duplicated".into()),
            SstateCleanupMode::Orphans => arguments.push("--remove-orphans".into()),
            SstateCleanupMode::UnreferencedByStamps => {
                for stamps in &request.stamps_dirs {
                    arguments.push("--stamps-dir".into());
                    arguments.push(stamps.display().to_string());
                }
            }
        }
    }
    arguments.extend(["--jobs".into(), request.jobs.to_string()]);
    if preview {
        arguments.push("--debug".into());
    }
    if execute {
        arguments.push("--yes".into());
    }
    arguments
}

fn pr_service_arguments(request: &PrServiceRequest) -> Vec<String> {
    vec![
        match request.operation {
            PrServiceOperation::Export => "export",
            PrServiceOperation::Import => "import",
        }
        .into(),
        request.file.display().to_string(),
    ]
}

fn filesystem_identity(
    path: &Path,
    directory: bool,
) -> Result<FilesystemIdentity, MaintenanceSstateAdapterError> {
    let metadata = safe_metadata(path, directory)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if metadata.is_dir() != directory {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    Ok(FilesystemIdentity {
        path: canonical,
        byte_size: metadata.len(),
        modified_at: metadata
            .modified()
            .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(path.into()))?,
        directory,
    })
}

pub(crate) fn guard_regular_file(
    path: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceSstateAdapterError> {
    Ok(MaintenanceFilesystemGuard::Existing(filesystem_identity(
        path, false,
    )?))
}

pub(crate) fn guard_directory(
    path: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceSstateAdapterError> {
    Ok(MaintenanceFilesystemGuard::Existing(filesystem_identity(
        path, true,
    )?))
}

pub(crate) fn guard_directory_or_absent(
    path: &Path,
) -> Result<MaintenanceFilesystemGuard, MaintenanceSstateAdapterError> {
    if path.exists() {
        return guard_directory(path);
    }
    let parent = path
        .parent()
        .ok_or_else(|| MaintenanceSstateAdapterError::UnsafePath(path.into()))?;
    Ok(MaintenanceFilesystemGuard::Absent {
        path: path.into(),
        parent: filesystem_identity(parent, true)?,
    })
}

fn revalidate_filesystem_guard(
    guard: &MaintenanceFilesystemGuard,
) -> Result<(), MaintenanceSstateAdapterError> {
    match guard {
        MaintenanceFilesystemGuard::Existing(expected) => {
            let current = filesystem_identity(&expected.path, expected.directory)?;
            if &current != expected {
                return Err(MaintenanceSstateAdapterError::StaleIdentity(
                    expected.path.clone(),
                ));
            }
        }
        MaintenanceFilesystemGuard::Absent { path, parent } => {
            if path.exists() || filesystem_identity(&parent.path, true)? != *parent {
                return Err(MaintenanceSstateAdapterError::StaleIdentity(path.clone()));
            }
        }
    }
    Ok(())
}

fn require_writable(
    path: &Path,
    metadata: &fs::Metadata,
) -> Result<(), MaintenanceSstateAdapterError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o222 == 0 {
            return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
        }
    }
    #[cfg(not(unix))]
    if metadata.permissions().readonly() {
        return Err(MaintenanceSstateAdapterError::UnsafePath(path.into()));
    }
    Ok(())
}

fn inspect_pr_service_file(
    request: &PrServiceRequest,
) -> Result<PrServiceFileGuard, MaintenanceSstateAdapterError> {
    match request.operation {
        PrServiceOperation::Export => {
            let parent = request
                .file
                .parent()
                .ok_or_else(|| MaintenanceSstateAdapterError::UnsafePath(request.file.clone()))?;
            let parent_identity = filesystem_identity(parent, true)?;
            let parent_metadata = fs::metadata(parent)
                .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(parent.into()))?;
            require_writable(parent, &parent_metadata)?;
            let existing = if request.file.exists() {
                let identity = filesystem_identity(&request.file, false)?;
                let metadata = fs::metadata(&request.file)
                    .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(request.file.clone()))?;
                require_writable(&request.file, &metadata)?;
                Some(identity)
            } else {
                None
            };
            Ok(PrServiceFileGuard::Export {
                parent: parent_identity,
                existing,
            })
        }
        PrServiceOperation::Import => {
            let identity = filesystem_identity(&request.file, false)?;
            fs::File::open(&request.file)
                .map_err(|_| MaintenanceSstateAdapterError::UnsafePath(request.file.clone()))?;
            Ok(PrServiceFileGuard::Import(identity))
        }
    }
}

fn revalidate_pr_service_file(
    request: &PrServiceRequest,
    expected: &PrServiceFileGuard,
) -> Result<(), MaintenanceSstateAdapterError> {
    let current = inspect_pr_service_file(request)?;
    if &current != expected {
        return Err(MaintenanceSstateAdapterError::StaleIdentity(
            request.file.clone(),
        ));
    }
    Ok(())
}

fn indexed_arguments(executable: &Path, arguments: &[String]) -> Vec<String> {
    std::iter::once(format!("0: {}", executable.display()))
        .chain(
            arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| format!("{}: {argument}", index + 1)),
        )
        .collect()
}
