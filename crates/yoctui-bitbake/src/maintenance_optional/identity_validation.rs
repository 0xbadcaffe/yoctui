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
