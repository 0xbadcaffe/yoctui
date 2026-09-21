fn parse_report(
    identity: QaReportIdentity,
    file: &ExactFile,
    bytes: &[u8],
    outer_limitations: &mut Vec<String>,
) -> Result<Option<QaReport>, QaReportAdapterError> {
    let mut limitations = Vec::new();
    let mut metadata = Vec::new();
    let findings = match identity.format {
        QaReportFormat::Json => parse_json_findings(file, bytes, &mut metadata, &mut limitations)?,
        QaReportFormat::Xml => parse_xml_findings(file, bytes, &mut limitations)?,
        QaReportFormat::Text => parse_text_findings(file, bytes, false, &mut limitations)?,
        QaReportFormat::BitBakeLog => parse_text_findings(file, bytes, true, &mut limitations)?,
    };
    if findings.is_empty() && !limitations.is_empty() {
        for limitation in &limitations {
            push_limitation(outer_limitations, limitation.clone());
        }
        return Ok(None);
    }
    Ok(Some(QaReport {
        identity,
        findings,
        metadata,
        limitations,
    }))
}

fn parse_json_findings(
    file: &ExactFile,
    bytes: &[u8],
    metadata: &mut Vec<QaMetadata>,
    limitations: &mut Vec<String>,
) -> Result<Vec<QaFinding>, QaReportAdapterError> {
    let values = if file
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().ends_with(".jsonl"))
    {
        let text = std::str::from_utf8(bytes)
            .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
        let mut values = Vec::new();
        for (index, line) in text.lines().enumerate() {
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(line) {
                Ok(value) => values.push(value),
                Err(error) => push_limitation(
                    limitations,
                    format!("JSONL record {} was malformed: {error}", index + 1),
                ),
            }
            if values.len() >= MAX_QA_FINDINGS {
                push_limitation(
                    limitations,
                    format!("JSONL input reached the {MAX_QA_FINDINGS}-record bound"),
                );
                break;
            }
        }
        values
    } else {
        let value: Value = serde_json::from_slice(bytes)
            .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
        match value {
            Value::Array(values) => values,
            Value::Object(mut object) => {
                if let Some(Value::Object(report_metadata)) = object.remove("metadata") {
                    *metadata = scalar_metadata(&report_metadata, limitations);
                }
                match object.remove("findings") {
                    Some(Value::Array(values)) => values,
                    _ if object.contains_key("status") && object.contains_key("message") => {
                        vec![Value::Object(object)]
                    }
                    _ => {
                        push_limitation(
                            limitations,
                            "JSON report contained no documented findings array".into(),
                        );
                        Vec::new()
                    }
                }
            }
            _ => {
                push_limitation(
                    limitations,
                    "JSON report root was not an object or array".into(),
                );
                Vec::new()
            }
        }
    };
    let omitted = values.len().saturating_sub(MAX_QA_FINDINGS);
    let mut findings = Vec::new();
    for value in values.into_iter().take(MAX_QA_FINDINGS) {
        let Some(object) = value.as_object() else {
            push_limitation(limitations, "a non-object JSON finding was ignored".into());
            continue;
        };
        match finding_from_object(file, object, limitations) {
            Some(finding) => findings.push(finding),
            None => push_limitation(limitations, "a malformed JSON finding was ignored".into()),
        }
    }
    if omitted > 0 {
        push_limitation(
            limitations,
            format!("{omitted} JSON findings were omitted at the {MAX_QA_FINDINGS}-record bound"),
        );
    }
    Ok(findings)
}

fn finding_from_object(
    file: &ExactFile,
    object: &Map<String, Value>,
    limitations: &mut Vec<String>,
) -> Option<QaFinding> {
    if object
        .get("check")
        .and_then(Value::as_str)
        .is_some_and(|value| value != file.producer.0)
    {
        push_limitation(
            limitations,
            "a finding for a different check identity was ignored".into(),
        );
        return None;
    }
    let message = bounded_json_string(object, "message", limitations)?;
    let raw_status = bounded_json_string(object, "status", limitations)?;
    let status = finding_status(&raw_status);
    if status == QaFindingStatus::Unknown && !raw_status.eq_ignore_ascii_case("unknown") {
        push_limitation(
            limitations,
            format!("unrecognized QA status was preserved as unknown: {raw_status}"),
        );
    }
    let source = object
        .get("source")
        .and_then(|value| parse_source(value, limitations));
    let record = serde_json::to_vec(object).ok()?;
    let identity = QaFindingIdentity::new(file.producer.clone(), fingerprint(&record)).ok()?;
    let mut metadata = object
        .get("metadata")
        .and_then(Value::as_object)
        .map(|values| scalar_metadata(values, limitations))
        .unwrap_or_default();
    metadata.sort();
    metadata.dedup();
    Some(QaFinding {
        identity,
        status,
        severity: bounded_json_string(object, "severity", limitations),
        message,
        scope: file.scope.clone(),
        task: file.task.clone(),
        test_name: bounded_json_string(object, "test_name", limitations)
            .or_else(|| file.test_name.clone()),
        source,
        rule: bounded_json_string(object, "rule", limitations),
        suggestion: bounded_json_string(object, "suggestion", limitations),
        metadata,
    })
    .filter(QaFinding::is_valid)
}

fn parse_source(value: &Value, limitations: &mut Vec<String>) -> Option<QaSourceLocation> {
    let object = value.as_object()?;
    let path = object.get("path")?.as_str().map(PathBuf::from)?;
    let line = object
        .get("line")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    let column = object
        .get("column")
        .and_then(Value::as_u64)
        .and_then(|value| u32::try_from(value).ok());
    if validate_regular_file(&path).is_err() {
        push_limitation(
            limitations,
            format!(
                "unsafe or missing QA source was ignored: {}",
                path.display()
            ),
        );
        return None;
    }
    QaSourceLocation::new(path, line, column).ok()
}
