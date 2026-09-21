fn validate_environment(
    environment: &CompatibilityEnvironmentIdentity,
) -> Result<(), CompatibilityProtocolError> {
    validate_detected(&environment.build_directory, |path| {
        valid_path(path, "build directory")
    })?;
    validate_detected(&environment.bitbake_version, |value| {
        valid_text(value, "BitBake version")
    })?;
    validate_detected(&environment.machine, |value| {
        valid_id(value)
            .then_some(())
            .ok_or(CompatibilityProtocolError::InvalidText("machine"))
    })?;
    for release in [&environment.oe_core, &environment.poky] {
        validate_detected(release, |release| {
            if release.name.is_none() && release.version.is_none() {
                return Err(CompatibilityProtocolError::InvalidText("release"));
            }
            for value in [release.name.as_deref(), release.version.as_deref()]
                .into_iter()
                .flatten()
            {
                valid_text(value, "release")?;
            }
            Ok(())
        })?;
    }
    validate_detected(&environment.distro, |distro| {
        valid_token(&distro.name, "distro")?;
        if let Some(version) = &distro.version {
            valid_text(version, "distro version")?;
        }
        Ok(())
    })?;
    validate_detected(&environment.source_roots, |roots| {
        valid_collection(roots, "source roots")?;
        for root in roots {
            valid_text(&root.kind, "source root kind")?;
            valid_path(&root.path, "source root")?;
        }
        Ok(())
    })?;
    validate_detected(&environment.layer_series, |layers| {
        valid_collection(layers, "layer series")?;
        for layer in layers {
            if !valid_id(&layer.layer) {
                return Err(CompatibilityProtocolError::InvalidText("layer"));
            }
            valid_path(&layer.root, "layer root")?;
            valid_collection(&layer.compatible_series, "compatible series")?;
            if layer
                .compatible_series
                .iter()
                .any(|series| !valid_id(series))
            {
                return Err(CompatibilityProtocolError::InvalidText("compatible series"));
            }
        }
        Ok(())
    })?;
    validate_detected(&environment.available_tools, |tools| {
        valid_collection(tools, "available tools")?;
        for tool in tools {
            if !valid_id(&tool.id) {
                return Err(CompatibilityProtocolError::InvalidText("tool id"));
            }
            valid_path(&tool.executable, "tool executable")?;
            if let Some(version) = &tool.version {
                valid_text(version, "tool version")?;
            }
        }
        Ok(())
    })?;
    validate_detected(&environment.backend, |backend| {
        if !valid_id(&backend.name) {
            return Err(CompatibilityProtocolError::InvalidText("backend"));
        }
        if let Some(version) = &backend.version {
            valid_text(version, "backend version")?;
        }
        Ok(())
    })?;
    validate_detected(&environment.protocol, |protocol| {
        if !valid_id(&protocol.name) {
            return Err(CompatibilityProtocolError::InvalidText("protocol"));
        }
        valid_text(&protocol.version, "protocol version")
    })?;
    Ok(())
}

fn validate_detected<T>(
    value: &CompatibilityDetected<T>,
    validate: impl FnOnce(&T) -> Result<(), CompatibilityProtocolError>,
) -> Result<(), CompatibilityProtocolError> {
    match value {
        CompatibilityDetected::Unknown => Ok(()),
        CompatibilityDetected::Detected { value, authority } => {
            if *authority == CompatibilityIdentityAuthority::Unknown {
                return Err(CompatibilityProtocolError::UnknownIdentityAuthority);
            }
            validate(value)
        }
    }
}

fn validate_capability(
    capability: &CompatibilityCapabilityData,
) -> Result<(), CompatibilityProtocolError> {
    if capability.evidence.len() > MAX_COMPATIBILITY_EVIDENCE {
        return Err(CompatibilityProtocolError::Oversized("capability evidence"));
    }
    for evidence in &capability.evidence {
        valid_text(&evidence.subject, "evidence subject")?;
        valid_text(&evidence.detail, "evidence detail")?;
        if evidence.argv.len() > MAX_COMPATIBILITY_ARGV {
            return Err(CompatibilityProtocolError::Oversized("evidence argv"));
        }
        for argument in &evidence.argv {
            valid_text(argument, "evidence argv")?;
        }
    }
    if let Some(implementation) = &capability.implementation
        && (!valid_id(&implementation.id) || !valid_id(&implementation.kind))
    {
        return Err(CompatibilityProtocolError::InvalidText("implementation"));
    }
    let has_positive = capability.evidence.iter().any(|evidence| {
        evidence.outcome == CompatibilityEvidenceOutcome::Positive
            && evidence.kind != CompatibilityEvidenceKind::Unknown
    });
    let has_negative = capability.evidence.iter().any(|evidence| {
        evidence.outcome == CompatibilityEvidenceOutcome::Negative
            && evidence.kind != CompatibilityEvidenceKind::Unknown
    });
    match &capability.state {
        CompatibilityStateData::Available => {
            if !has_positive || capability.implementation.is_none() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::AvailableWithLimitations {
            reason,
            limitations,
        } => {
            validate_reason(reason)?;
            valid_collection(limitations, "limitations")?;
            for limitation in limitations {
                valid_text(limitation, "limitation")?;
            }
            if !has_positive || capability.implementation.is_none() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::Unavailable { reason } => {
            validate_reason(reason)?;
            if !has_negative || capability.implementation.is_some() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::Unknown { reason }
        | CompatibilityStateData::Unsupported { reason } => {
            validate_reason(reason)?;
            if capability.implementation.is_some() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
        CompatibilityStateData::UnknownWireState => {
            if capability.implementation.is_some() {
                return Err(CompatibilityProtocolError::EvidenceMismatch(
                    capability.id.clone(),
                ));
            }
        }
    }
    Ok(())
}

fn validate_reason(reason: &CompatibilityReasonData) -> Result<(), CompatibilityProtocolError> {
    if !valid_id(&reason.code) {
        return Err(CompatibilityProtocolError::InvalidText("reason code"));
    }
    valid_text(&reason.message, "reason message")?;
    if let Some(requirement) = &reason.requirement {
        valid_text(requirement, "reason requirement")?;
    }
    Ok(())
}

fn valid_collection<T>(
    values: &[T],
    field: &'static str,
) -> Result<(), CompatibilityProtocolError> {
    if values.is_empty() || values.len() > MAX_COMPATIBILITY_ITEMS {
        return Err(CompatibilityProtocolError::Oversized(field));
    }
    Ok(())
}

fn valid_text(value: &str, field: &'static str) -> Result<(), CompatibilityProtocolError> {
    if value.is_empty()
        || value.len() > MAX_COMPATIBILITY_TEXT_BYTES
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        return Err(CompatibilityProtocolError::InvalidText(field));
    }
    Ok(())
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}

fn valid_token(value: &str, field: &'static str) -> Result<(), CompatibilityProtocolError> {
    valid_text(value, field)?;
    if value.chars().any(char::is_whitespace) {
        return Err(CompatibilityProtocolError::InvalidText(field));
    }
    Ok(())
}

fn valid_path(value: &str, field: &'static str) -> Result<(), CompatibilityProtocolError> {
    let path = Path::new(value);
    if !path.is_absolute()
        || path == Path::new("/")
        || value.len() > MAX_COMPATIBILITY_TEXT_BYTES
        || !path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
    {
        return Err(CompatibilityProtocolError::InvalidPath(field));
    }
    Ok(())
}
