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
