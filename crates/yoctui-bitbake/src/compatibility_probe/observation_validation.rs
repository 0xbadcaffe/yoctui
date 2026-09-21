fn set_observation(
    values: &Option<BTreeSet<String>>,
    value: &str,
    kind: CapabilityEvidenceKind,
    subject: &str,
) -> CapabilityProbeObservation {
    let Some(values) = values else {
        return observation(
            CapabilityProbeStatus::Inconclusive,
            kind,
            subject,
            format!("{subject} inventory was not probed for {value}"),
            Vec::new(),
        );
    };
    let present = values.contains(value);
    observation(
        if present {
            CapabilityProbeStatus::Positive
        } else {
            CapabilityProbeStatus::Negative
        },
        kind,
        subject,
        if present {
            format!("Connected environment reports {value}")
        } else {
            format!("Connected environment does not report {value}")
        },
        Vec::new(),
    )
}

fn observation(
    status: CapabilityProbeStatus,
    kind: CapabilityEvidenceKind,
    subject: impl Into<String>,
    detail: String,
    argv: Vec<String>,
) -> CapabilityProbeObservation {
    CapabilityProbeObservation {
        status,
        evidence: CapabilityEvidence {
            kind,
            outcome: match status {
                CapabilityProbeStatus::Positive => CapabilityEvidenceOutcome::Positive,
                CapabilityProbeStatus::Negative => CapabilityEvidenceOutcome::Negative,
                CapabilityProbeStatus::Inconclusive => CapabilityEvidenceOutcome::Inconclusive,
            },
            subject: bounded(&subject.into()),
            detail: bounded(&detail),
            argv: argv.into_iter().map(|value| bounded(&value)).collect(),
        },
    }
}

fn validate_tools(
    identity: &AuthoritativeValue<Vec<ToolIdentity>>,
    tools: &BTreeMap<CapabilityToolId, PathBuf>,
) -> Result<(), CapabilityProbeContextError> {
    if tools.is_empty() {
        return Ok(());
    }
    let Some(authoritative) = identity.value() else {
        return Err(CapabilityProbeContextError::EnvironmentMismatch);
    };
    for (tool, path) in tools {
        if !valid_absolute_path(path) {
            return Err(CapabilityProbeContextError::UnsafeTool(path.clone()));
        }
        if !authoritative
            .iter()
            .any(|identity| identity.id == tool.executable_name() && identity.executable == *path)
        {
            return Err(CapabilityProbeContextError::EnvironmentMismatch);
        }
    }
    Ok(())
}

fn canonical_directory(path: &Path) -> Option<PathBuf> {
    if !valid_absolute_path(path) {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return None;
    }
    let canonical = fs::canonicalize(path).ok()?;
    (canonical == path).then_some(canonical)
}

fn safe_executable(path: &Path) -> Option<PathBuf> {
    if !valid_absolute_path(path) {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok()?;
    let is_symlink = metadata.file_type().is_symlink();
    let executable_metadata = if is_symlink {
        let target = fs::read_link(path).ok()?;
        if target.is_absolute()
            || target
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return None;
        }
        let parent = path.parent()?;
        if fs::canonicalize(parent).ok()? != parent {
            return None;
        }
        let canonical_target = fs::canonicalize(parent.join(target)).ok()?;
        if canonical_target.parent()? != parent {
            return None;
        }
        fs::metadata(canonical_target).ok()?
    } else {
        metadata
    };
    if !executable_metadata.is_file() {
        return None;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if executable_metadata.permissions().mode() & 0o111 == 0 {
            return None;
        }
    }
    if is_symlink {
        Some(path.to_owned())
    } else {
        let canonical = fs::canonicalize(path).ok()?;
        (canonical == path).then_some(canonical)
    }
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().as_encoded_bytes().len() <= MAX_PROBE_TEXT_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}

fn valid_environment_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROBE_TEXT_BYTES
        && !value.chars().any(|character| character == '\0')
}

fn valid_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_PROBE_TEXT_BYTES
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

fn bounded(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control() || *character == '\t')
        .take(MAX_PROBE_TEXT_BYTES)
        .collect()
}
