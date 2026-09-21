fn canonical_descendant(root: &Path, path: &Path) -> Result<PathBuf, SdkArtifactAdapterError> {
    let canonical =
        fs::canonicalize(path).map_err(|error| SdkArtifactAdapterError::Io(error.to_string()))?;
    if canonical != path || !canonical.starts_with(root) || canonical == root {
        return Err(SdkArtifactAdapterError::Io(format!(
            "SDK entry escaped or was not canonical: {}",
            path_label(path)
        )));
    }
    Ok(canonical)
}

fn check_scan_control(
    cancellation: &SdkArtifactCancellation,
    deadline: Instant,
) -> Result<(), SdkArtifactAdapterError> {
    if cancellation.is_cancelled() {
        Err(SdkArtifactAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(SdkArtifactAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

fn valid_record_name(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            !name.is_empty()
                && name.len() <= MAX_SDK_NAME_BYTES
                && !matches!(name, "." | "..")
                && !name.chars().any(char::is_control)
        })
}

fn classify_record(path: &Path) -> Option<SdkArtifactKind> {
    let name = path.file_name()?.to_str()?.to_ascii_lowercase();
    let suffix = checksum_suffix(&name);
    if let Some(suffix) = suffix {
        return (name.len() > suffix.len()).then_some(SdkArtifactKind::Checksum);
    }
    if name.ends_with(".manifest") {
        return (name.len() > ".manifest".len()).then_some(SdkArtifactKind::Manifest);
    }
    if name.ends_with(".sh") {
        return (name.len() > ".sh".len()).then_some(SdkArtifactKind::Installer);
    }
    Some(SdkArtifactKind::Other)
}

fn checksum_suffix(name: &str) -> Option<&'static str> {
    [".sha256sum", ".sha512", ".sha256", ".md5sum", ".md5"]
        .into_iter()
        .find(|suffix| name.ends_with(suffix))
}

fn associate_records(artifacts: &mut [SdkArtifact], limitations: &mut Vec<String>) {
    let checksums = artifacts
        .iter()
        .filter(|artifact| artifact.kind == SdkArtifactKind::Checksum)
        .map(|artifact| artifact.identity.path.clone())
        .collect::<Vec<_>>();
    let manifests = artifacts
        .iter()
        .filter(|artifact| artifact.kind == SdkArtifactKind::Manifest)
        .map(|artifact| artifact.identity.path.clone())
        .collect::<Vec<_>>();

    for artifact in artifacts
        .iter_mut()
        .filter(|artifact| artifact.kind == SdkArtifactKind::Installer)
    {
        let installer = &artifact.identity.path;
        artifact.checksums = associated_paths(
            installer,
            &checksums,
            SdkArtifactKind::Checksum,
            limitations,
        );
        artifact.manifests = associated_paths(
            installer,
            &manifests,
            SdkArtifactKind::Manifest,
            limitations,
        );
    }
}

fn associated_paths(
    installer: &Path,
    candidates: &[PathBuf],
    kind: SdkArtifactKind,
    limitations: &mut Vec<String>,
) -> Vec<PathBuf> {
    let installer_name = installer
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let installer_stem = installer
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut associated = candidates
        .iter()
        .filter(|candidate| candidate.parent() == installer.parent())
        .filter(|candidate| {
            let Some(name) = candidate.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            let base = match kind {
                SdkArtifactKind::Checksum => checksum_suffix(name)
                    .and_then(|suffix| name.strip_suffix(suffix))
                    .unwrap_or_default(),
                SdkArtifactKind::Manifest => manifest_base(name),
                _ => "",
            };
            base == installer_name || base == installer_stem
        })
        .cloned()
        .collect::<Vec<_>>();
    associated.sort();
    associated.dedup();
    if associated.len() > MAX_SDK_ASSOCIATIONS {
        let omitted = associated.len() - MAX_SDK_ASSOCIATIONS;
        associated.truncate(MAX_SDK_ASSOCIATIONS);
        push_limitation(
            limitations,
            format!(
                "{omitted} {:?} associations were omitted for {}",
                kind,
                path_label(installer)
            ),
        );
    }
    associated
}

fn manifest_base(name: &str) -> &str {
    for suffix in [".host.manifest", ".target.manifest", ".manifest"] {
        if let Some(base) = name.strip_suffix(suffix) {
            return base;
        }
    }
    ""
}

fn path_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| name.len() <= MAX_SDK_NAME_BYTES && !name.chars().any(char::is_control))
        .unwrap_or("<unavailable>")
        .to_owned()
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitations.len() < yoctui_model::MAX_SDK_LIMITATIONS {
        limitations.push(limitation);
    } else if limitations.len() == yoctui_model::MAX_SDK_LIMITATIONS {
        limitations.pop();
        limitations.push("additional SDK scan limitations were omitted".into());
    }
}
