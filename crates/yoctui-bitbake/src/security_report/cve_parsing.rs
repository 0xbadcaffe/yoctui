fn looks_like_cve(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(looks_like_cve),
        Value::Object(object) => {
            ["cves", "CVE", "cve", "issues", "issue"]
                .iter()
                .any(|key| object.contains_key(*key))
                || cve_id(object).is_some()
                || object.get("package").is_some_and(|value| value.is_array())
        }
        _ => false,
    }
}

#[derive(Debug, Clone, Default)]
struct CveContext {
    recipe: Option<String>,
    package: Option<String>,
    product: Option<String>,
    version: Option<String>,
}

fn parse_cve_json(identity: SecurityReportIdentity, value: &Value) -> CveReport {
    let mut findings = Vec::new();
    let mut limitations = Vec::new();
    let mut metadata = Vec::new();
    if let Value::Object(object) = value {
        metadata = scalar_metadata(
            object,
            &["package", "packages", "products", "cves", "issues"],
        );
    }
    collect_cve_findings(
        value,
        &CveContext::default(),
        0,
        &mut findings,
        &mut limitations,
    );
    if findings.is_empty() {
        push_limitation(
            &mut limitations,
            "CVE JSON contained no supported findings".into(),
        );
    }
    CveReport {
        identity,
        scope: None,
        findings,
        metadata,
        limitations,
    }
}

fn collect_cve_findings(
    value: &Value,
    inherited: &CveContext,
    depth: usize,
    findings: &mut Vec<CveFinding>,
    limitations: &mut Vec<String>,
) {
    if depth > MAX_PARSE_DEPTH {
        push_limitation(
            limitations,
            format!("CVE JSON exceeded the {MAX_PARSE_DEPTH}-level nesting bound"),
        );
        return;
    }
    match value {
        Value::Array(values) => {
            for value in values.iter().take(MAX_SECURITY_FINDINGS) {
                collect_cve_findings(value, inherited, depth + 1, findings, limitations);
            }
            if values.len() > MAX_SECURITY_FINDINGS {
                push_limitation(
                    limitations,
                    format!(
                        "{} CVE values were omitted at the {MAX_SECURITY_FINDINGS}-record bound",
                        values.len() - MAX_SECURITY_FINDINGS
                    ),
                );
            }
        }
        Value::Object(object) => {
            let context = cve_context(object, inherited, limitations);
            if cve_id(object).is_some() {
                if findings.len() >= MAX_SECURITY_FINDINGS {
                    push_limitation(
                        limitations,
                        format!("CVE findings reached the {MAX_SECURITY_FINDINGS}-record bound"),
                    );
                    return;
                }
                match cve_finding(object, &context, limitations) {
                    Some(finding) => findings.push(finding),
                    None => {
                        push_limitation(limitations, "a malformed CVE finding was ignored".into())
                    }
                }
            }
            for key in [
                "package", "packages", "products", "cves", "CVEs", "issues", "issue",
            ] {
                if let Some(child) = object.get(key) {
                    collect_cve_findings(child, &context, depth + 1, findings, limitations);
                }
            }
        }
        _ => {}
    }
}

fn cve_context(
    object: &Map<String, Value>,
    inherited: &CveContext,
    limitations: &mut Vec<String>,
) -> CveContext {
    let name = bounded_json_string(object, &["name"], limitations);
    CveContext {
        recipe: bounded_json_string(object, &["recipe", "pn"], limitations)
            .or_else(|| inherited.recipe.clone())
            .or_else(|| name.clone()),
        package: bounded_json_string(object, &["package", "package_name"], limitations)
            .or_else(|| inherited.package.clone())
            .or(name),
        product: bounded_json_string(object, &["product"], limitations)
            .or_else(|| inherited.product.clone()),
        version: bounded_json_string(object, &["version", "package_version"], limitations)
            .or_else(|| inherited.version.clone()),
    }
}

fn cve_id(object: &Map<String, Value>) -> Option<String> {
    ["id", "cve", "CVE"]
        .iter()
        .find_map(|key| object.get(*key).and_then(Value::as_str))
        .map(str::to_owned)
        .filter(|value| value.starts_with("CVE-"))
}

fn cve_finding(
    object: &Map<String, Value>,
    context: &CveContext,
    limitations: &mut Vec<String>,
) -> Option<CveFinding> {
    let cve = cve_id(object)?;
    let recipe = bounded_json_string(object, &["recipe", "pn"], limitations)
        .or_else(|| context.recipe.clone())?;
    let package = bounded_json_string(object, &["package", "package_name"], limitations)
        .or_else(|| context.package.clone());
    let identity = CveFindingIdentity::new(cve, recipe, package).ok()?;
    let raw_status = bounded_json_string(object, &["status", "state"], limitations);
    let status = raw_status
        .as_deref()
        .map(cve_status)
        .unwrap_or(CveStatus::Unknown);
    if status == CveStatus::Unknown && raw_status.is_some() {
        push_limitation(
            limitations,
            format!(
                "unrecognized CVE status was preserved as Unknown: {}",
                raw_status.unwrap_or_default()
            ),
        );
    }
    let mapping = object
        .get("mapping")
        .and_then(Value::as_object)
        .map(|mapping| scalar_metadata(mapping, &[]))
        .unwrap_or_default();
    Some(CveFinding {
        identity,
        status,
        product: bounded_json_string(object, &["product"], limitations)
            .or_else(|| context.product.clone()),
        version: bounded_json_string(object, &["version", "package_version"], limitations)
            .or_else(|| context.version.clone()),
        severity: bounded_json_string(object, &["severity"], limitations),
        score: bounded_json_string(object, &["score", "cvss_score"], limitations),
        vector: bounded_json_string(object, &["vector", "cvss_vector"], limitations),
        advisory_url: bounded_json_string(object, &["link", "url", "advisory"], limitations)
            .filter(|value| value.starts_with("https://")),
        summary: bounded_json_string(object, &["summary", "description"], limitations),
        mapping,
    })
}

fn parse_cve_text(
    identity: SecurityReportIdentity,
    bytes: &[u8],
    outer_limitations: &mut Vec<String>,
) -> Result<ParseReportOutcome, SecurityReportAdapterError> {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => {
            push_limitation(
                outer_limitations,
                format!("CVE text was not UTF-8: {error}"),
            );
            return Ok(ParseReportOutcome::Malformed);
        }
    };
    let mut findings = Vec::new();
    let mut limitations = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = if line.contains('\t') {
            line.split('\t').collect::<Vec<_>>()
        } else {
            line.split_whitespace().collect::<Vec<_>>()
        };
        if fields.first().is_some_and(|field| {
            field.eq_ignore_ascii_case("recipe") || field.eq_ignore_ascii_case("pn")
        }) {
            continue;
        }
        let Some(cve_index) = fields.iter().position(|field| field.starts_with("CVE-")) else {
            push_limitation(
                &mut limitations,
                format!("CVE text line {} was ignored", index + 1),
            );
            continue;
        };
        if cve_index == 0 || fields.len() <= cve_index + 1 {
            push_limitation(
                &mut limitations,
                format!("malformed CVE text line {} was ignored", index + 1),
            );
            continue;
        }
        let recipe = fields[0].to_owned();
        let package = (cve_index > 2).then(|| fields[1].to_owned());
        let version = (cve_index > 1).then(|| fields[cve_index - 1].to_owned());
        let Ok(finding_identity) =
            CveFindingIdentity::new(fields[cve_index].to_owned(), recipe, package)
        else {
            push_limitation(
                &mut limitations,
                format!("invalid CVE identity on text line {}", index + 1),
            );
            continue;
        };
        findings.push(CveFinding {
            identity: finding_identity,
            status: cve_status(fields[cve_index + 1]),
            product: None,
            version,
            severity: fields.get(cve_index + 2).map(|value| (*value).to_owned()),
            score: fields.get(cve_index + 3).map(|value| (*value).to_owned()),
            vector: fields.get(cve_index + 4).map(|value| (*value).to_owned()),
            advisory_url: fields
                .get(cve_index + 5)
                .filter(|value| value.starts_with("https://"))
                .map(|value| (*value).to_owned()),
            summary: None,
            mapping: Vec::new(),
        });
        if findings.len() >= MAX_SECURITY_FINDINGS {
            push_limitation(
                &mut limitations,
                format!("CVE text reached the {MAX_SECURITY_FINDINGS}-record bound"),
            );
            break;
        }
    }
    Ok(ParseReportOutcome::Report(Box::new(SecurityReport::Cve(
        CveReport {
            identity,
            scope: None,
            findings,
            metadata: Vec::new(),
            limitations,
        },
    ))))
}

fn cve_status(value: &str) -> CveStatus {
    let normalized = value.trim().to_ascii_lowercase().replace([' ', '_'], "-");
    match normalized.as_str() {
        "vulnerable" | "unpatched" | "version-in-range" => CveStatus::Vulnerable,
        "patched" | "fixed" | "fix-file-included" => CveStatus::Patched,
        "ignored" => CveStatus::Ignored,
        "not-affected" | "not-applicable" | "version-not-in-range" => CveStatus::NotAffected,
        _ => CveStatus::Unknown,
    }
}
