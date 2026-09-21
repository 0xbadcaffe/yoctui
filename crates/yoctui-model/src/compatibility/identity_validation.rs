#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum EnvironmentIdentityError {
    #[error("invalid authoritative environment identity field: {0}")]
    InvalidField(&'static str),
    #[error("invalid authority {authority:?} for environment identity field {field}")]
    InvalidAuthority {
        field: &'static str,
        authority: IdentityAuthority,
    },
    #[error("too many entries in environment identity field {field}: {count} > {limit}")]
    TooManyEntries {
        field: &'static str,
        count: usize,
        limit: usize,
    },
    #[error("conflicting duplicate environment identity for {field}: {key}")]
    ConflictingDuplicate { field: &'static str, key: String },
}

fn validate_detected<T>(
    field: &'static str,
    detected: &AuthoritativeValue<T>,
    authorities: &[IdentityAuthority],
    valid: impl FnOnce(&T) -> bool,
) -> Result<(), EnvironmentIdentityError> {
    let AuthoritativeValue::Detected { value, authority } = detected else {
        return Ok(());
    };
    if !authorities.contains(authority) {
        return Err(EnvironmentIdentityError::InvalidAuthority {
            field,
            authority: *authority,
        });
    }
    if !valid(value) {
        return Err(EnvironmentIdentityError::InvalidField(field));
    }
    Ok(())
}

fn normalize_source_roots(
    value: &mut AuthoritativeValue<Vec<SourceRootIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "source_roots",
        value,
        &[
            IdentityAuthority::BitBakeDatastore,
            IdentityAuthority::ConfiguredLayerMetadata,
            IdentityAuthority::InitializedEnvironment,
        ],
        |roots| {
            !roots.is_empty()
                && roots.len() <= MAX_ENVIRONMENT_SOURCE_ROOTS
                && roots.iter().all(|root| {
                    valid_absolute_path(&root.path)
                        && match &root.kind {
                            SourceRootKind::Other(label) => valid_token(label),
                            _ => true,
                        }
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: roots, .. } = value else {
        return Ok(());
    };
    roots.sort();
    roots.dedup();
    Ok(())
}

fn normalize_layer_series(
    value: &mut AuthoritativeValue<Vec<LayerSeriesIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "layer_series",
        value,
        &[IdentityAuthority::ConfiguredLayerMetadata],
        |layers| {
            !layers.is_empty()
                && layers.len() <= MAX_ENVIRONMENT_LAYER_SERIES
                && layers.iter().all(|layer| {
                    valid_token(&layer.layer)
                        && valid_absolute_path(&layer.root)
                        && !layer.compatible_series.is_empty()
                        && layer.compatible_series.len() <= MAX_LAYER_COMPATIBLE_SERIES
                        && layer
                            .compatible_series
                            .iter()
                            .all(|series| valid_token(series))
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: layers, .. } = value else {
        return Ok(());
    };
    for layer in layers.iter_mut() {
        layer.compatible_series.sort();
        layer.compatible_series.dedup();
    }
    reject_conflicts(
        "layer_series",
        layers,
        |layer| layer.layer.clone(),
        |layer| (layer.root.clone(), layer.compatible_series.clone()),
    )?;
    layers.sort();
    layers.dedup();
    Ok(())
}

fn normalize_tools(
    value: &mut AuthoritativeValue<Vec<ToolIdentity>>,
) -> Result<(), EnvironmentIdentityError> {
    validate_detected(
        "available_tools",
        value,
        &[
            IdentityAuthority::ExecutableProbe,
            IdentityAuthority::InitializedEnvironment,
        ],
        |tools| {
            !tools.is_empty()
                && tools.len() <= MAX_ENVIRONMENT_TOOLS
                && tools.iter().all(|tool| {
                    valid_token(&tool.id)
                        && valid_absolute_path(&tool.executable)
                        && tool.version.as_deref().is_none_or(valid_text)
                })
        },
    )?;
    let AuthoritativeValue::Detected { value: tools, .. } = value else {
        return Ok(());
    };
    reject_conflicts(
        "available_tools",
        tools,
        |tool| tool.id.clone(),
        |tool| (tool.executable.clone(), tool.version.clone()),
    )?;
    tools.sort();
    tools.dedup();
    Ok(())
}

fn reject_conflicts<T, V: PartialEq>(
    field: &'static str,
    values: &[T],
    key: impl Fn(&T) -> String,
    identity: impl Fn(&T) -> V,
) -> Result<(), EnvironmentIdentityError> {
    let mut seen = BTreeMap::new();
    for value in values {
        let key = key(value);
        let identity = identity(value);
        if seen.get(&key).is_some_and(|previous| previous != &identity) {
            return Err(EnvironmentIdentityError::ConflictingDuplicate { field, key });
        }
        seen.insert(key, identity);
    }
    Ok(())
}

fn valid_release(value: &ReleaseIdentity) -> bool {
    (value.name.is_some() || value.version.is_some())
        && value.name.as_deref().is_none_or(valid_text)
        && value.version.as_deref().is_none_or(valid_text)
}

fn valid_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn valid_token(value: &str) -> bool {
    valid_text(value) && !value.chars().any(char::is_whitespace)
}

fn valid_absolute_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().as_encoded_bytes().len() <= MAX_ENVIRONMENT_IDENTITY_PATH_BYTES
        && path
            .components()
            .all(|component| matches!(component, Component::RootDir | Component::Normal(_)))
}
