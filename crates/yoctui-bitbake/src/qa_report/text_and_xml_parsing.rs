fn parse_text_findings(
    file: &ExactFile,
    bytes: &[u8],
    bitbake_log: bool,
    limitations: &mut Vec<String>,
) -> Result<Vec<QaFinding>, QaReportAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
    let mut findings = Vec::new();
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parsed = if bitbake_log {
            parse_bitbake_line(file, line)
        } else {
            parse_tab_line(file, line)
        };
        match parsed {
            Some(finding) => findings.push(finding),
            None => push_limitation(
                limitations,
                format!("unsupported QA text record {} was ignored", index + 1),
            ),
        }
        if findings.len() >= MAX_QA_FINDINGS {
            push_limitation(
                limitations,
                format!("QA text reached the {MAX_QA_FINDINGS}-record bound"),
            );
            break;
        }
    }
    Ok(findings)
}

fn parse_tab_line(file: &ExactFile, line: &str) -> Option<QaFinding> {
    let mut fields = line.split('\t');
    let status_text = fields.next()?;
    let message = fields.next()?.to_owned();
    if !bounded_text(&message) {
        return None;
    }
    let mut severity = None;
    let mut rule = None;
    let mut suggestion = None;
    let mut metadata = Vec::new();
    for field in fields.take(MAX_QA_METADATA) {
        let (key, value) = field.split_once('=')?;
        match key {
            "severity" if bounded_text(value) => severity = Some(value.to_owned()),
            "rule" if bounded_text(value) => rule = Some(value.to_owned()),
            "suggestion" if bounded_text(value) => suggestion = Some(value.to_owned()),
            _ => {
                if let Ok(value) = QaMetadata::new(key.to_owned(), value.to_owned()) {
                    metadata.push(value);
                }
            }
        }
    }
    finding_from_text(
        file,
        finding_status(status_text),
        severity,
        message,
        rule,
        suggestion,
        metadata,
        line,
    )
}

fn parse_bitbake_line(file: &ExactFile, line: &str) -> Option<QaFinding> {
    let (status, severity, rest) = if let Some(rest) = line.strip_prefix("ERROR: QA Issue: ") {
        (QaFindingStatus::Failed, Some("error".into()), rest)
    } else {
        let rest = line.strip_prefix("WARNING: QA Issue: ")?;
        (QaFindingStatus::Warning, Some("warning".into()), rest)
    };
    let (message, rule) = rest
        .strip_suffix(']')
        .and_then(|value| value.rsplit_once(" ["))
        .map_or((rest.to_owned(), None), |(message, rule)| {
            (
                message.to_owned(),
                bounded_text(rule).then(|| rule.to_owned()),
            )
        });
    finding_from_text(
        file,
        status,
        severity,
        message,
        rule,
        None,
        Vec::new(),
        line,
    )
}

#[allow(clippy::too_many_arguments)]
fn finding_from_text(
    file: &ExactFile,
    status: QaFindingStatus,
    severity: Option<String>,
    message: String,
    rule: Option<String>,
    suggestion: Option<String>,
    metadata: Vec<QaMetadata>,
    raw: &str,
) -> Option<QaFinding> {
    if !bounded_text(&message) {
        return None;
    }
    let identity =
        QaFindingIdentity::new(file.producer.clone(), fingerprint(raw.as_bytes())).ok()?;
    Some(QaFinding {
        identity,
        status,
        severity,
        message,
        scope: file.scope.clone(),
        task: file.task.clone(),
        test_name: file.test_name.clone(),
        source: None,
        rule,
        suggestion,
        metadata,
    })
    .filter(QaFinding::is_valid)
}

fn parse_xml_findings(
    file: &ExactFile,
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<Vec<QaFinding>, QaReportAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| QaReportAdapterError::MalformedReport(file.path.clone()))?;
    if !text.trim_start().starts_with("<qa-report") {
        push_limitation(
            limitations,
            "XML root was not the documented qa-report envelope".into(),
        );
        return Ok(Vec::new());
    }
    let mut findings = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("<finding ") {
        remaining = &remaining[start + "<finding ".len()..];
        let Some(end) = remaining.find("/>") else {
            push_limitation(limitations, "unterminated XML finding was ignored".into());
            break;
        };
        let attributes = &remaining[..end];
        remaining = &remaining[end + 2..];
        let values = parse_xml_attributes(attributes);
        let Some(status) = values.get("status") else {
            push_limitation(limitations, "XML finding without status was ignored".into());
            continue;
        };
        let Some(message) = values.get("message").filter(|value| bounded_text(value)) else {
            push_limitation(
                limitations,
                "XML finding without message was ignored".into(),
            );
            continue;
        };
        let raw = format!("{status}\0{message}\0{attributes}");
        if let Some(finding) = finding_from_text(
            file,
            finding_status(status),
            values.get("severity").cloned(),
            message.clone(),
            values.get("rule").cloned(),
            values.get("suggestion").cloned(),
            Vec::new(),
            &raw,
        ) {
            findings.push(finding);
        }
        if findings.len() >= MAX_QA_FINDINGS {
            push_limitation(
                limitations,
                format!("QA XML reached the {MAX_QA_FINDINGS}-record bound"),
            );
            break;
        }
    }
    if findings.is_empty() {
        push_limitation(
            limitations,
            "XML report contained no supported self-closing finding records".into(),
        );
    }
    Ok(findings)
}

fn parse_xml_attributes(value: &str) -> BTreeMap<String, String> {
    let mut output = BTreeMap::new();
    let mut remaining = value.trim();
    while let Some((key, tail)) = remaining.split_once('=') {
        let key = key.trim();
        let Some(tail) = tail.strip_prefix('"') else {
            break;
        };
        let Some(end) = tail.find('"') else {
            break;
        };
        let decoded = decode_xml(&tail[..end]);
        if bounded_token(key) && bounded_text(&decoded) {
            output.insert(key.to_owned(), decoded);
        }
        remaining = tail[end + 1..].trim_start();
    }
    output
}

fn decode_xml(value: &str) -> String {
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}
