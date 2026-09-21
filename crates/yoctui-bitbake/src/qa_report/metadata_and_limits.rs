fn bounded_json_string(
    object: &Map<String, Value>,
    key: &str,
    limitations: &mut Vec<String>,
) -> Option<String> {
    let value = match object.get(key)? {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        Value::Bool(value) => value.to_string(),
        _ => return None,
    };
    if bounded_text(&value) {
        Some(value)
    } else {
        push_limitation(
            limitations,
            format!("invalid or oversized QA field was ignored: {key}"),
        );
        None
    }
}

fn scalar_metadata(object: &Map<String, Value>, limitations: &mut Vec<String>) -> Vec<QaMetadata> {
    let mut output = BTreeMap::new();
    for (key, value) in object.iter().take(MAX_QA_METADATA) {
        let value = match value {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        };
        if let Some(value) = value {
            if let Ok(metadata) = QaMetadata::new(key.clone(), value) {
                output.insert(metadata.key.clone(), metadata);
            } else {
                push_limitation(limitations, "invalid QA metadata was ignored".into());
            }
        }
    }
    output.into_values().collect()
}

fn finding_status(value: &str) -> QaFindingStatus {
    match value.trim().to_ascii_lowercase().as_str() {
        "pass" | "passed" | "ok" => QaFindingStatus::Passed,
        "warn" | "warning" => QaFindingStatus::Warning,
        "fail" | "failed" | "error" => QaFindingStatus::Failed,
        "skip" | "skipped" => QaFindingStatus::Skipped,
        _ => QaFindingStatus::Unknown,
    }
}

fn bounded_text(value: &str) -> bool {
    yoctui_utils::is_bounded_plain_text(value, MAX_QA_TEXT_BYTES)
}

fn bounded_token(value: &str) -> bool {
    yoctui_utils::is_bounded_identifier(value, 256)
}

fn push_limitation(limitations: &mut Vec<String>, value: String) {
    if limitations.len() < MAX_QA_LIMITATIONS && bounded_text(&value) {
        limitations.push(value);
    }
}
