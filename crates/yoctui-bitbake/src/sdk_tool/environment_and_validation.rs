fn native_arguments(request: &SdkNativeRequest) -> Vec<OsString> {
    let mut arguments = vec![OsString::from(&request.recipe)];
    if let Some(tool) = &request.tool {
        arguments.push(OsString::from(tool));
    }
    arguments.extend(request.arguments.iter().map(OsString::from));
    arguments
}

fn validate_workspace_roots(
    workspace_roots: &[PathBuf],
) -> Result<Vec<PathBuf>, SdkToolAdapterError> {
    if workspace_roots.is_empty() || workspace_roots.len() > MAX_SDK_TOOL_ROOTS {
        return Err(SdkToolAdapterError::InvalidRequest(format!(
            "SDK tool roots must contain between 1 and {MAX_SDK_TOOL_ROOTS} entries"
        )));
    }
    let mut roots = Vec::new();
    for root in workspace_roots {
        roots.push(validate_exact_directory(
            root,
            SdkToolAdapterError::UnsafeWorkspaceRoot(root.clone()),
        )?);
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn discover_tool(roots: &[PathBuf], name: &str) -> Result<Option<PathBuf>, SdkToolAdapterError> {
    let mut found = BTreeSet::new();
    for root in roots {
        for candidate in [root.join("scripts").join(name), root.join(name)] {
            match fs::symlink_metadata(&candidate) {
                Ok(_) => {
                    found.insert(validate_executable(&candidate)?);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => return Err(SdkToolAdapterError::UnsafeTool(candidate)),
            }
        }
    }
    Ok(found.pop_first())
}

fn validate_named_tool(
    path: &Path,
    expected_name: &str,
    roots: &[PathBuf],
) -> Result<PathBuf, SdkToolAdapterError> {
    if path.file_name() != Some(OsStr::new(expected_name)) {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    let canonical = validate_executable(path)?;
    if !roots
        .iter()
        .any(|root| canonical.starts_with(root) && canonical != *root)
    {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    Ok(canonical)
}

fn validate_executable(path: &Path) -> Result<PathBuf, SdkToolAdapterError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| SdkToolAdapterError::UnsafeTool(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| SdkToolAdapterError::UnsafeTool(path.into()))?;
    if canonical != path {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(SdkToolAdapterError::UnsafeTool(path.into()));
        }
    }
    Ok(canonical)
}

fn validate_exact_directory(
    path: &Path,
    error: SdkToolAdapterError,
) -> Result<PathBuf, SdkToolAdapterError> {
    if !path.is_absolute()
        || path == Path::new("/")
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(error);
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| error.clone())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error);
    }
    let canonical = fs::canonicalize(path).map_err(|_| error.clone())?;
    if canonical != path {
        return Err(error);
    }
    Ok(canonical)
}

fn validate_installer(
    identity: &SdkArtifactIdentity,
    deploy_root: &Path,
) -> Result<(), SdkToolAdapterError> {
    identity
        .validate()
        .map_err(|_| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    if !identity.path.starts_with(deploy_root)
        || identity.path == deploy_root
        || identity.path.extension() != Some(OsStr::new("sh"))
    {
        return Err(SdkToolAdapterError::UnsafeInstaller(identity.path.clone()));
    }
    let metadata = fs::symlink_metadata(&identity.path)
        .map_err(|_| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SdkToolAdapterError::UnsafeInstaller(identity.path.clone()));
    }
    let canonical = fs::canonicalize(&identity.path)
        .map_err(|_| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    let modified = modified_seconds(&metadata)
        .ok_or_else(|| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    if canonical != identity.path
        || metadata.len() != identity.size_bytes
        || modified != identity.modified_unix_seconds
    {
        return Err(SdkToolAdapterError::UnsafeInstaller(identity.path.clone()));
    }
    Ok(())
}

fn validate_empty_destination(path: &Path) -> Result<PathBuf, SdkToolAdapterError> {
    let destination =
        validate_exact_directory(path, SdkToolAdapterError::UnsafeDestination(path.into()))?;
    let mut entries = fs::read_dir(&destination)
        .map_err(|_| SdkToolAdapterError::UnsafeDestination(path.into()))?;
    if entries.next().is_some() {
        return Err(SdkToolAdapterError::UnsafeDestination(path.into()));
    }
    Ok(destination)
}

fn find_environment_setup(root: &Path) -> Result<SdkEnvironmentSetupIdentity, SdkToolAdapterError> {
    let mut matches = Vec::new();
    let entries =
        fs::read_dir(root).map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
    for entry in entries {
        let entry = entry.map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("environment-setup-") {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SdkToolAdapterError::UnsafeExtractedRoot(root.into()));
        }
        let canonical = fs::canonicalize(&path)
            .map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        if canonical != path || canonical.parent() != Some(root) {
            return Err(SdkToolAdapterError::UnsafeExtractedRoot(root.into()));
        }
        let modified_unix_seconds = modified_seconds(&metadata)
            .ok_or_else(|| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        matches.push(SdkEnvironmentSetupIdentity {
            path: canonical,
            size_bytes: metadata.len(),
            modified_unix_seconds,
        });
    }
    matches.sort_by(|left, right| left.path.cmp(&right.path));
    if matches.len() != 1 {
        return Err(SdkToolAdapterError::InvalidEnvironment(format!(
            "expected exactly one environment-setup-* file, found {}",
            matches.len()
        )));
    }
    Ok(matches.remove(0))
}

fn modified_seconds(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn parse_environment_setup(
    path: &Path,
) -> Result<BTreeMap<OsString, OsString>, SdkToolAdapterError> {
    let metadata = fs::metadata(path)
        .map_err(|error| SdkToolAdapterError::InvalidEnvironment(error.to_string()))?;
    if metadata.len() > MAX_SDK_ENVIRONMENT_BYTES {
        return Err(SdkToolAdapterError::InvalidEnvironment(format!(
            "environment setup exceeds {MAX_SDK_ENVIRONMENT_BYTES} bytes"
        )));
    }
    let bytes = fs::read(path)
        .map_err(|error| SdkToolAdapterError::InvalidEnvironment(error.to_string()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        SdkToolAdapterError::InvalidEnvironment("environment setup is not UTF-8".into())
    })?;
    let mut values = BTreeMap::<String, String>::new();
    if let Ok(path) = std::env::var("PATH")
        && path.len() <= MAX_SDK_ENVIRONMENT_VALUE_BYTES
        && path.is_ascii()
        && !path.chars().any(char::is_control)
    {
        values.insert("PATH".into(), path);
    }
    let mut exported = 0_usize;
    for (index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let assignment = line.strip_prefix("export ").ok_or_else(|| {
            SdkToolAdapterError::InvalidEnvironment(format!(
                "unsupported environment statement on line {}",
                index + 1
            ))
        })?;
        let (name, raw_value) = assignment.split_once('=').ok_or_else(|| {
            SdkToolAdapterError::InvalidEnvironment(format!(
                "malformed environment assignment on line {}",
                index + 1
            ))
        })?;
        if !valid_environment_name(name) || exported >= MAX_SDK_ENVIRONMENT_VARIABLES {
            return Err(SdkToolAdapterError::InvalidEnvironment(format!(
                "invalid or excessive environment variable on line {}",
                index + 1
            )));
        }
        let value = parse_environment_value(raw_value.trim(), &values).map_err(|message| {
            SdkToolAdapterError::InvalidEnvironment(format!("{message} on line {}", index + 1))
        })?;
        if value.len() > MAX_SDK_ENVIRONMENT_VALUE_BYTES || value.chars().any(char::is_control) {
            return Err(SdkToolAdapterError::InvalidEnvironment(format!(
                "environment value exceeds its bound on line {}",
                index + 1
            )));
        }
        values.insert(name.into(), value);
        exported = exported.saturating_add(1);
    }
    if exported == 0 {
        return Err(SdkToolAdapterError::InvalidEnvironment(
            "environment setup contained no supported exports".into(),
        ));
    }
    Ok(values
        .into_iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect())
}

fn valid_environment_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn parse_environment_value(
    value: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, &'static str> {
    if !value.is_ascii() {
        return Err("non-ASCII environment values are unsupported");
    }
    if value.is_empty() {
        return Ok(String::new());
    }
    let (value, expand) = if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        (&value[1..value.len() - 1], false)
    } else if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        (&value[1..value.len() - 1], true)
    } else {
        if value.chars().any(|character| {
            character.is_whitespace()
                || matches!(character, ';' | '|' | '&' | '<' | '>' | '`' | '\\')
        }) {
            return Err("unsafe unquoted environment value");
        }
        (value, true)
    };
    if !expand {
        return Ok(value.into());
    }
    expand_environment_value(value, variables)
}

fn expand_environment_value(
    value: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, &'static str> {
    let characters = value.as_bytes();
    let mut expanded = String::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] != b'$' {
            expanded.push(characters[index] as char);
            index += 1;
            continue;
        }
        index += 1;
        let (name, next) = if characters.get(index) == Some(&b'{') {
            let start = index + 1;
            let Some(end_offset) = characters[start..].iter().position(|byte| *byte == b'}') else {
                return Err("unterminated environment expansion");
            };
            let end = start + end_offset;
            (&value[start..end], end + 1)
        } else {
            let start = index;
            while index < characters.len()
                && (characters[index] == b'_'
                    || (characters[index] as char).is_ascii_alphanumeric())
            {
                index += 1;
            }
            if start == index {
                return Err("unsupported environment expansion");
            }
            (&value[start..index], index)
        };
        if !valid_environment_name(name) {
            return Err("invalid environment expansion");
        }
        let replacement = variables
            .get(name)
            .ok_or("environment expansion referenced an unavailable variable")?;
        expanded.push_str(replacement);
        index = next;
    }
    Ok(expanded)
}
