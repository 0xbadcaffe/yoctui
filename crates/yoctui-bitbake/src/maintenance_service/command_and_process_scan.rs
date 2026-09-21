pub fn pr_service_command(
    session: MaintenanceSessionId,
    capability_request: u64,
    snapshot: &MaintenanceCapabilitySnapshot,
    operation_id: u64,
    request: PrServiceRequest,
) -> Result<
    (MaintenanceOperationPreview, MaintenanceSstateCommandSpec),
    MaintenanceServiceAdapterError,
> {
    MaintenanceSstateCommandSpec::pr_service(
        session,
        capability_request,
        snapshot,
        operation_id,
        request,
    )
    .map_err(Into::into)
}

fn discover_pr_service_tool(
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
        let candidate = directory.join("bitbake-prserv-tool");
        match executable_identity(&candidate) {
            Ok(executable) => {
                return MaintenanceToolCapability::Available {
                    tool: MaintenanceTool::PrServiceTool,
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
        tool: MaintenanceTool::PrServiceTool,
        reason: "bitbake-prserv-tool is unavailable in the configured child search path".into(),
    }
}

fn executable_identity(
    path: &Path,
) -> Result<MaintenanceFileIdentity, MaintenanceServiceAdapterError> {
    if path.file_name().and_then(|name| name.to_str()) != Some("bitbake-prserv-tool") {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
        }
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    MaintenanceFileIdentity::new(
        canonical,
        metadata.len(),
        metadata
            .modified()
            .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?,
    )
    .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))
}

fn canonical_directory(path: &Path) -> Result<PathBuf, MaintenanceServiceAdapterError> {
    if !path.is_absolute() || path == Path::new("/") {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|_| MaintenanceServiceAdapterError::UnsafePath(path.into()))?;
    if canonical != path {
        return Err(MaintenanceServiceAdapterError::UnsafePath(path.into()));
    }
    Ok(canonical)
}

#[derive(Debug)]
struct ProcessScan {
    processes: BTreeMap<ServiceKind, Vec<ServiceProcessEvidence>>,
    limitations: Vec<String>,
}

fn scan_processes(process_root: &Path) -> Result<ProcessScan, MaintenanceServiceAdapterError> {
    let process_root = canonical_directory(process_root)?;
    let directory = fs::read_dir(&process_root)
        .map_err(|error| MaintenanceServiceAdapterError::ProcessInspection(error.to_string()))?;
    let mut entries = Vec::new();
    let mut limitations = Vec::new();
    for entry in directory.take(MAX_PROCESS_ENTRIES + 1) {
        match entry {
            Ok(entry) => entries.push(entry),
            Err(error) => push_limitation(
                &mut limitations,
                format!("one process entry could not be inspected: {error}"),
            ),
        }
    }
    if entries.len() > MAX_PROCESS_ENTRIES {
        entries.truncate(MAX_PROCESS_ENTRIES);
        push_limitation(
            &mut limitations,
            "process inspection reached the entry limit".into(),
        );
    }
    entries.sort_by_key(fs::DirEntry::file_name);
    let mut processes = BTreeMap::<ServiceKind, Vec<ServiceProcessEvidence>>::new();
    for entry in entries {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse().ok())
        else {
            continue;
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => metadata,
            _ => continue,
        };
        let _ = metadata;
        let Some(executable) = read_process_name(&path) else {
            continue;
        };
        let kind = match executable.as_str() {
            "bitbake-prserv" => ServiceKind::Pr,
            "bitbake-hashserv" => ServiceKind::Hash,
            "bitbake-worker" => ServiceKind::Worker,
            _ => continue,
        };
        let evidence = ServiceProcessEvidence::new(pid, executable)
            .map_err(|message| MaintenanceServiceAdapterError::InvalidInput(message.into()))?;
        processes.entry(kind).or_default().push(evidence);
    }
    for values in processes.values_mut() {
        values.sort();
        values.dedup();
        values.truncate(MAX_MAINTENANCE_OUTPUT);
    }
    Ok(ProcessScan {
        processes,
        limitations,
    })
}

fn read_process_name(process_path: &Path) -> Option<String> {
    let comm = read_limited(&process_path.join("comm"), MAX_PROCESS_NAME_BYTES).ok()?;
    let comm = String::from_utf8_lossy(&comm).trim().to_string();
    if !comm.is_empty() {
        return Some(comm);
    }
    let command = read_limited(&process_path.join("cmdline"), MAX_PROCESS_NAME_BYTES).ok()?;
    let first = command.split(|byte| *byte == 0).next()?;
    Path::new(std::str::from_utf8(first).ok()?)
        .file_name()
        .and_then(|name| name.to_str())
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
