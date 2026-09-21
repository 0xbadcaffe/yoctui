fn parse_spdx(identity: SecurityReportIdentity, value: &Value) -> SpdxDocument {
    let mut limitations = Vec::new();
    let Some(object) = value.as_object() else {
        return SpdxDocument {
            identity,
            scope: None,
            kind: SpdxArtifactKind::Json,
            spdx_version: None,
            name: None,
            namespace: None,
            data_license: None,
            creators: Vec::new(),
            components: Vec::new(),
            file_count: None,
            relationship_count: None,
            checksums: Vec::new(),
            limitations: vec![
                "unsupported SPDX JSON root was retained as an exact artifact".into(),
            ],
        };
    };
    let spdx_version =
        bounded_json_string(object, &["spdxVersion", "specVersion"], &mut limitations);
    if spdx_version.is_none() {
        push_limitation(
            &mut limitations,
            "unsupported SPDX schema was retained as an exact artifact".into(),
        );
    }
    let creators = object
        .get("creationInfo")
        .and_then(Value::as_object)
        .and_then(|creation| creation.get("creators"))
        .and_then(Value::as_array)
        .map(|values| bounded_string_array(values, &mut limitations))
        .unwrap_or_default();
    let components = object
        .get("packages")
        .or_else(|| object.get("components"))
        .and_then(Value::as_array)
        .map(|values| spdx_components(values, &mut limitations))
        .unwrap_or_default();
    let file_count = object
        .get("files")
        .and_then(Value::as_array)
        .map(|values| values.len() as u64);
    let relationship_count = object
        .get("relationships")
        .and_then(Value::as_array)
        .map(|values| values.len() as u64);
    let checksums = object
        .get("checksums")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| {
                    let object = value.as_object()?;
                    let algorithm = object.get("algorithm")?.as_str()?.to_owned();
                    let checksum = object.get("checksumValue")?.as_str()?.to_owned();
                    SecurityMetadata::new(algorithm, checksum).ok()
                })
                .take(MAX_SECURITY_METADATA)
                .collect()
        })
        .unwrap_or_default();
    SpdxDocument {
        identity,
        scope: None,
        kind: SpdxArtifactKind::Json,
        spdx_version,
        name: bounded_json_string(object, &["name"], &mut limitations),
        namespace: bounded_json_string(
            object,
            &["documentNamespace", "namespace"],
            &mut limitations,
        ),
        data_license: bounded_json_string(object, &["dataLicense"], &mut limitations),
        creators,
        components,
        file_count,
        relationship_count,
        checksums,
        limitations,
    }
}

fn spdx_components(values: &[Value], limitations: &mut Vec<String>) -> Vec<SpdxComponent> {
    let mut components = Vec::new();
    for value in values.iter().take(MAX_SECURITY_COMPONENTS) {
        let Some(object) = value.as_object() else {
            push_limitation(limitations, "a malformed SPDX component was ignored".into());
            continue;
        };
        let identity = bounded_json_string(object, &["SPDXID", "spdxId", "id"], limitations);
        let name = bounded_json_string(object, &["name"], limitations);
        let Some((identity, name)) = identity.zip(name) else {
            push_limitation(limitations, "a malformed SPDX component was ignored".into());
            continue;
        };
        let component = SpdxComponent {
            identity,
            name,
            version: bounded_json_string(object, &["versionInfo", "version"], limitations),
            supplier: bounded_json_string(object, &["supplier"], limitations),
            license: bounded_json_string(
                object,
                &["licenseConcluded", "licenseDeclared", "license"],
                limitations,
            ),
        };
        if component.is_valid() {
            components.push(component);
        } else {
            push_limitation(limitations, "an invalid SPDX component was ignored".into());
        }
    }
    if values.len() > MAX_SECURITY_COMPONENTS {
        push_limitation(
            limitations,
            format!(
                "{} SPDX components were omitted at the {MAX_SECURITY_COMPONENTS}-record bound",
                values.len() - MAX_SECURITY_COMPONENTS
            ),
        );
    }
    components
}

fn bounded_string_array(values: &[Value], limitations: &mut Vec<String>) -> Vec<String> {
    let mut output = Vec::new();
    for value in values.iter().take(MAX_SECURITY_METADATA) {
        if let Some(value) = value.as_str().filter(|value| valid_text(value)) {
            output.push(value.to_owned());
        } else {
            push_limitation(limitations, "an invalid SPDX text value was ignored".into());
        }
    }
    output
}

fn bounded_json_string(
    object: &Map<String, Value>,
    keys: &[&str],
    limitations: &mut Vec<String>,
) -> Option<String> {
    let value = keys
        .iter()
        .find_map(|key| object.get(*key))
        .and_then(|value| match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })?;
    if valid_text(&value) {
        Some(value)
    } else {
        push_limitation(
            limitations,
            format!(
                "an oversized or invalid Security field was ignored (maximum {MAX_SECURITY_TEXT_BYTES} bytes)"
            ),
        );
        None
    }
}

fn scalar_metadata(object: &Map<String, Value>, excluded: &[&str]) -> Vec<SecurityMetadata> {
    let mut metadata = BTreeMap::new();
    for (key, value) in object {
        if excluded.contains(&key.as_str()) {
            continue;
        }
        let value = match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        };
        if let Some(value) = value
            && let Ok(value) = SecurityMetadata::new(key.clone(), value)
        {
            metadata.insert(value.key.clone(), value);
        }
        if metadata.len() >= MAX_SECURITY_METADATA {
            break;
        }
    }
    metadata.into_values().collect()
}

fn valid_text(value: &str) -> bool {
    yoctui_utils::is_bounded_plain_text(value, MAX_SECURITY_TEXT_BYTES)
}

fn push_limitation(limitations: &mut Vec<String>, value: String) {
    if limitations.len() < MAX_SECURITY_LIMITATIONS && valid_text(&value) {
        limitations.push(value);
    }
}

fn check_control(
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<(), SecurityReportAdapterError> {
    if cancellation.is_cancelled() {
        Err(SecurityReportAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(SecurityReportAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}
