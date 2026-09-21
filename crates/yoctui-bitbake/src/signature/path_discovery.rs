async fn canonical_build_dir(build_dir: &Path) -> Result<PathBuf, SignatureAdapterError> {
    tokio::fs::canonicalize(build_dir)
        .await
        .map_err(|_| SignatureAdapterError::BuildDirectory(build_dir.to_owned()))
}

async fn validate_identity_path(
    canonical_build_dir: &Path,
    identity: &SignatureIdentity,
) -> Result<PathBuf, SignatureAdapterError> {
    identity
        .validate()
        .map_err(|message| SignatureAdapterError::InvalidRequest(message.into()))?;
    let path = identity
        .path
        .as_deref()
        .ok_or(SignatureAdapterError::MissingPath)?;
    let canonical = validate_signature_path(canonical_build_dir, path).await?;
    let discovered = identity_from_path(&identity.target, canonical.clone())?;
    if discovered.hash != identity.hash {
        return Err(SignatureAdapterError::InvalidRequest(format!(
            "signature hash does not match {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

async fn validate_signature_path(
    canonical_build_dir: &Path,
    path: &Path,
) -> Result<PathBuf, SignatureAdapterError> {
    if !path.is_absolute() {
        return Err(SignatureAdapterError::PathEscape(path.to_owned()));
    }
    let link_metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| SignatureAdapterError::InvalidFile(path.to_owned()))?;
    if link_metadata.file_type().is_symlink() || !link_metadata.is_file() {
        return Err(SignatureAdapterError::InvalidFile(path.to_owned()));
    }
    let canonical = tokio::fs::canonicalize(path)
        .await
        .map_err(|_| SignatureAdapterError::InvalidFile(path.to_owned()))?;
    if !canonical.starts_with(canonical_build_dir) {
        return Err(SignatureAdapterError::PathEscape(canonical));
    }
    Ok(canonical)
}

fn identity_from_path(
    target: &SignatureTarget,
    path: PathBuf,
) -> Result<SignatureIdentity, SignatureAdapterError> {
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str());
    if parent != Some(target.recipe.as_str()) {
        return Err(SignatureAdapterError::InvalidRequest(format!(
            "signature path does not belong to recipe {}: {}",
            target.recipe,
            path.display()
        )));
    }
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| SignatureAdapterError::InvalidFile(path.clone()))?;
    let hash = signature_hash_from_name(name, &target.task).ok_or_else(|| {
        SignatureAdapterError::InvalidRequest(format!(
            "signature path does not match task {}: {}",
            target.task,
            path.display()
        ))
    })?;
    let identity = SignatureIdentity {
        target: target.clone(),
        hash,
        path: Some(path),
    };
    identity
        .validate()
        .map_err(|message| SignatureAdapterError::InvalidRequest(message.into()))?;
    Ok(identity)
}

fn signature_hash_from_name(name: &str, task: &str) -> Option<Option<String>> {
    for kind in ["sigdata", "siginfo"] {
        let marker = format!(".{task}.{kind}");
        let Some((_, suffix)) = name.split_once(&marker) else {
            continue;
        };
        if suffix.is_empty() {
            return Some(None);
        }
        let hash = suffix.strip_prefix('.')?;
        if hash.is_empty() || hash.contains('.') || hash.chars().any(char::is_whitespace) {
            return None;
        }
        return Some(Some(hash.to_owned()));
    }
    None
}

fn discover_signature_paths(
    root: &Path,
    target: &SignatureTarget,
) -> Result<(Vec<PathBuf>, bool), SignatureAdapterError> {
    if !root.is_dir() {
        return Ok((Vec::new(), false));
    }
    let mut directories = vec![root.to_owned()];
    let mut paths = Vec::new();
    let mut visited = 0usize;
    while let Some(directory) = directories.pop() {
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| SignatureAdapterError::Io(error.to_string()))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
        entries.sort_by_key(std::fs::DirEntry::path);
        let mut child_directories = Vec::new();
        for entry in entries {
            visited += 1;
            if visited > MAX_SIGNATURE_SCAN_ENTRIES {
                paths.sort();
                return Ok((paths, true));
            }
            let file_type = entry
                .file_type()
                .map_err(|error| SignatureAdapterError::Io(error.to_string()))?;
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                child_directories.push(path);
            } else if file_type.is_file()
                && path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    == Some(target.recipe.as_str())
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| signature_hash_from_name(name, &target.task).is_some())
            {
                paths.push(path);
            }
        }
        child_directories.reverse();
        directories.extend(child_directories);
    }
    paths.sort();
    Ok((paths, false))
}

struct BoundedOutput {
    bytes: Vec<u8>,
    truncated: bool,
}
