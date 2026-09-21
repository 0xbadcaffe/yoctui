fn parse_report(
    identity: SecurityReportIdentity,
    bytes: &[u8],
    limitations: &mut Vec<String>,
    cancellation: &SecurityReportCancellation,
    deadline: Instant,
) -> Result<ParseReportOutcome, SecurityReportAdapterError> {
    check_control(cancellation, deadline)?;
    let name = identity
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.ends_with(".spdx.tar.zst")
        || name.ends_with(".spdx.tar.gz")
        || name.ends_with(".spdx.zip")
    {
        return Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Spdx(
            SpdxDocument {
                identity,
                scope: None,
                kind: SpdxArtifactKind::Archive,
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
                    "SPDX archive is retained as an exact artifact; archive contents are not parsed"
                        .into(),
                ],
            },
        ))));
    }
    if name.ends_with(".cve") || name.ends_with(".cve.txt") || name.ends_with(".cve.log") {
        return parse_cve_text(identity, bytes, limitations);
    }
    if name.ends_with(".manifest") {
        return parse_package_manifest(identity, bytes, limitations);
    }
    let value: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(error) => {
            push_limitation(
                limitations,
                format!("JSON report could not be parsed: {error}"),
            );
            return Ok(ParseReportOutcome::Malformed);
        }
    };
    if looks_like_cyclonedx(&value) || name.contains("cyclonedx") || name.contains("cdx") {
        return Ok(ParseReportOutcome::Report(Box::new(
            SecurityReport::CycloneDx(parse_cyclonedx(identity, &value)),
        )));
    }
    if looks_like_spdx(&value) || name.contains("spdx") {
        return Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Spdx(
            parse_spdx(identity, &value),
        ))));
    }
    if looks_like_cve(&value) || name.contains("cve") {
        return Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Cve(
            parse_cve_json(identity, &value),
        ))));
    }
    Ok(ParseReportOutcome::Unsupported)
}

fn looks_like_spdx(value: &Value) -> bool {
    value.get("spdxVersion").is_some()
        || value.get("SPDXID").is_some()
        || value.get("documentNamespace").is_some()
        || value.get("@context").is_some()
}

fn looks_like_cyclonedx(value: &Value) -> bool {
    value
        .get("bomFormat")
        .and_then(Value::as_str)
        .is_some_and(|value| value.eq_ignore_ascii_case("CycloneDX"))
        || (value.get("specVersion").is_some() && value.get("components").is_some())
}

fn parse_package_manifest(
    identity: SecurityReportIdentity,
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<ParseReportOutcome, SecurityReportAdapterError> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            push_limitation(
                limitations,
                format!("package manifest is not UTF-8: {error}"),
            );
            return Ok(ParseReportOutcome::Malformed);
        }
    };
    let mut components = Vec::new();
    let mut document_limitations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if components.len() == MAX_SECURITY_COMPONENTS {
            push_limitation(
                &mut document_limitations,
                format!(
                    "package manifest rows after {} were omitted at the component bound",
                    index + 1
                ),
            );
            break;
        }
        let fields = line.split_whitespace().collect::<Vec<_>>();
        let Some(name) = fields.first().copied() else {
            continue;
        };
        if name.len() > MAX_SECURITY_TEXT_BYTES || name.chars().any(char::is_control) {
            push_limitation(
                &mut document_limitations,
                format!("invalid package manifest row {} was ignored", index + 1),
            );
            continue;
        }
        // Yocto image manifests commonly use `package arch version`; older
        // outputs may contain only `package version`.
        let version = fields
            .get(2)
            .or_else(|| fields.get(1))
            .map(|value| (*value).to_owned());
        components.push(SpdxComponent {
            identity: name.to_owned(),
            name: name.to_owned(),
            version,
            supplier: None,
            license: None,
        });
    }
    if components.is_empty() {
        push_limitation(
            limitations,
            "package manifest contained no usable package rows".into(),
        );
        return Ok(ParseReportOutcome::Malformed);
    }
    Ok(ParseReportOutcome::Report(Box::new(
        SecurityReport::PackageManifest(PackageManifestDocument {
            identity,
            scope: None,
            components,
            limitations: document_limitations,
        }),
    )))
}

fn parse_cyclonedx(identity: SecurityReportIdentity, value: &Value) -> CycloneDxDocument {
    let mut limitations = Vec::new();
    let Some(object) = value.as_object() else {
        return CycloneDxDocument {
            identity,
            scope: None,
            spec_version: None,
            serial_number: None,
            version: None,
            components: Vec::new(),
            dependency_count: None,
            limitations: vec![
                "unsupported CycloneDX JSON root was retained as an exact artifact".into(),
            ],
        };
    };
    let components = object
        .get("components")
        .and_then(Value::as_array)
        .map(|values| cyclonedx_components(values, &mut limitations))
        .unwrap_or_default();
    if components.is_empty() {
        push_limitation(
            &mut limitations,
            "CycloneDX document contained no supported components".into(),
        );
    }
    CycloneDxDocument {
        identity,
        scope: None,
        spec_version: bounded_json_string(object, &["specVersion"], &mut limitations),
        serial_number: bounded_json_string(object, &["serialNumber"], &mut limitations),
        version: object.get("version").and_then(Value::as_u64),
        components,
        dependency_count: object
            .get("dependencies")
            .and_then(Value::as_array)
            .map(|values| values.len() as u64),
        limitations,
    }
}

fn cyclonedx_components(values: &[Value], limitations: &mut Vec<String>) -> Vec<SpdxComponent> {
    let mut components = Vec::new();
    for value in values.iter().take(MAX_SECURITY_COMPONENTS) {
        let Some(object) = value.as_object() else {
            push_limitation(
                limitations,
                "a malformed CycloneDX component was ignored".into(),
            );
            continue;
        };
        let Some(name) = object.get("name").and_then(Value::as_str) else {
            push_limitation(
                limitations,
                "a CycloneDX component without a name was ignored".into(),
            );
            continue;
        };
        let identity = object
            .get("bom-ref")
            .or_else(|| object.get("purl"))
            .and_then(Value::as_str)
            .unwrap_or(name);
        let supplier = object.get("supplier").and_then(|value| {
            value
                .as_object()
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
                .or_else(|| value.as_str())
        });
        let license = object
            .get("licenses")
            .and_then(Value::as_array)
            .and_then(|licenses| {
                licenses.first()?.as_object().and_then(|entry| {
                    entry.get("expression").and_then(Value::as_str).or_else(|| {
                        entry
                            .get("license")?
                            .as_object()?
                            .get("id")
                            .or_else(|| entry.get("license")?.as_object()?.get("name"))
                            .and_then(Value::as_str)
                    })
                })
            });
        components.push(SpdxComponent {
            identity: identity.to_owned(),
            name: name.to_owned(),
            version: object
                .get("version")
                .and_then(Value::as_str)
                .map(str::to_owned),
            supplier: supplier.map(str::to_owned),
            license: license.map(str::to_owned),
        });
    }
    if values.len() > MAX_SECURITY_COMPONENTS {
        push_limitation(
            limitations,
            format!(
                "{} CycloneDX components were omitted at the component bound",
                values.len() - MAX_SECURITY_COMPONENTS
            ),
        );
    }
    components
}
