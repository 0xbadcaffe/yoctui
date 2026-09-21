#[derive(Debug)]
struct PackageDataContext {
    build_dir: PathBuf,
    pkgdata_dir: PathBuf,
    tool: PathBuf,
    compatibility: DaemonCompatibilitySnapshot,
}

impl PackageDataContext {
    fn command(
        &self,
        capability: CapabilityId,
        implementation: &str,
        subcommand: &str,
        arguments: impl IntoIterator<Item = OsString>,
    ) -> Result<PackageDataCommandSpec, PackageDataAdapterError> {
        let record = self
            .compatibility
            .snapshot
            .capability(capability)
            .ok_or_else(|| PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: "the capability record is missing".into(),
            })?;
        if !record.state.is_enabled() {
            return Err(PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: record
                    .state
                    .reason()
                    .map(|reason| reason.message.clone())
                    .unwrap_or_else(|| "no positive capability evidence is available".into()),
            });
        }
        let selected = self
            .compatibility
            .implementations
            .get(&capability)
            .ok_or_else(|| PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: "no compatible implementation was selected".into(),
            })?;
        if selected.id != implementation {
            return Err(PackageDataAdapterError::CapabilityUnavailable {
                capability,
                reason: format!("selected implementation {} is incompatible", selected.id),
            });
        }
        Ok(PackageDataCommandSpec::new(
            &self.tool,
            &self.pkgdata_dir,
            subcommand,
            arguments,
        ))
    }
}

fn validate_inventory_request(
    request: PackageInventoryRequest,
) -> Result<(), PackageDataAdapterError> {
    if request.generation == 0 {
        Err(PackageDataAdapterError::InvalidRequest(
            "inventory generation must be nonzero".into(),
        ))
    } else {
        Ok(())
    }
}

fn validate_detail_request(request: &PackageDetailRequest) -> Result<(), PackageDataAdapterError> {
    if request.generation == 0 {
        return Err(PackageDataAdapterError::InvalidRequest(
            "detail generation must be nonzero".into(),
        ));
    }
    request
        .identity
        .validate()
        .map_err(|message| PackageDataAdapterError::InvalidRequest(message.into()))
}

fn unavailable_summary(identity: PackageIdentity) -> (PackageIdentity, PackageSummary) {
    (
        identity.clone(),
        PackageSummary {
            identity,
            recipe: PackageField::Unavailable,
            provider: PackageField::Unavailable,
            version: PackageField::Unavailable,
            installed_size_bytes: PackageField::Unavailable,
            license: PackageField::Unavailable,
            image_membership: PackageField::Unavailable,
        },
    )
}

fn summary_has_no_information(summary: &PackageSummary) -> bool {
    matches!(summary.recipe, PackageField::Unavailable)
        && matches!(summary.version, PackageField::Unavailable)
        && matches!(summary.installed_size_bytes, PackageField::Unavailable)
        && matches!(summary.license, PackageField::Unavailable)
}

fn parse_package_list(
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<Vec<PackageIdentity>, PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PackageDataAdapterError::Malformed("inventory is not UTF-8".into()))?;
    let mut identities = BTreeSet::new();
    for (index, line) in text.lines().enumerate() {
        if index >= MAX_PACKAGE_OUTPUT_LINES {
            push_limitation(
                limitations,
                format!("package inventory was limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
            );
            break;
        }
        let identity = PackageIdentity::new(line.trim());
        if identity.validate().is_err() {
            push_limitation(
                limitations,
                format!(
                    "invalid package inventory record was omitted at line {}",
                    index + 1
                ),
            );
            continue;
        }
        identities.insert(identity);
        if identities.len() == MAX_PACKAGE_RECORDS {
            if text
                .lines()
                .skip(index + 1)
                .any(|line| !line.trim().is_empty())
            {
                push_limitation(
                    limitations,
                    format!("package inventory was limited to {MAX_PACKAGE_RECORDS} records"),
                );
            }
            break;
        }
    }
    Ok(identities.into_iter().collect())
}

fn parse_package_info(
    bytes: &[u8],
    summaries: &mut BTreeMap<PackageIdentity, PackageSummary>,
    limitations: &mut Vec<String>,
) -> Result<(), PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        PackageDataAdapterError::Malformed("package information is not UTF-8".into())
    })?;
    for (index, line) in text.lines().take(MAX_PACKAGE_OUTPUT_LINES).enumerate() {
        let Some((fields, remainder)) = split_prefix_tokens(line, 5) else {
            push_limitation(
                limitations,
                format!(
                    "malformed package-info record was omitted at line {}",
                    index + 1
                ),
            );
            continue;
        };
        let identity = PackageIdentity::new(fields[0]);
        let Some(summary) = summaries.get_mut(&identity) else {
            push_limitation(
                limitations,
                format!(
                    "unexpected package-info identity {} was omitted",
                    identity.name
                ),
            );
            continue;
        };
        summary.version = nonempty_field(fields[1]);
        summary.recipe = nonempty_field(fields[2]);
        summary.installed_size_bytes = fields[4]
            .parse::<u64>()
            .map(PackageField::Available)
            .unwrap_or(PackageField::Unavailable);
        if matches!(summary.installed_size_bytes, PackageField::Unavailable) {
            push_limitation(
                limitations,
                format!("installed size was unavailable for {}", identity.name),
            );
        }
        let license = remainder.trim();
        let license = license
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(license);
        summary.license = nonempty_field(license);
    }
    if text.lines().count() > MAX_PACKAGE_OUTPUT_LINES {
        push_limitation(
            limitations,
            format!("package information was limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
        );
    }
    Ok(())
}

fn split_prefix_tokens(line: &str, count: usize) -> Option<(Vec<&str>, &str)> {
    let mut tokens = Vec::with_capacity(count);
    let mut cursor = 0;
    while tokens.len() < count {
        cursor += line[cursor..].find(|character: char| !character.is_whitespace())?;
        let end = line[cursor..]
            .find(char::is_whitespace)
            .map_or(line.len(), |offset| cursor + offset);
        tokens.push(&line[cursor..end]);
        cursor = end;
    }
    Some((tokens, line[cursor..].trim_start()))
}

fn nonempty_field(value: &str) -> PackageField<String> {
    if value.is_empty() {
        PackageField::Unavailable
    } else {
        PackageField::Available(value.into())
    }
}

fn parse_package_files(
    identity: &PackageIdentity,
    bytes: &[u8],
    limitations: &mut Vec<String>,
) -> Result<Vec<PathBuf>, PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PackageDataAdapterError::Malformed("package file list is not UTF-8".into()))?;
    let mut lines = text.lines();
    if lines.next().map(str::trim) != Some(format!("{}:", identity.name).as_str()) {
        return Err(PackageDataAdapterError::Malformed(format!(
            "file list identity does not match {}",
            identity.name
        )));
    }
    let mut files = Vec::new();
    for (index, line) in lines.enumerate() {
        if index >= MAX_PACKAGE_OUTPUT_LINES {
            push_limitation(
                limitations,
                format!("package file list was limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
            );
            break;
        }
        let path = PathBuf::from(line.trim());
        if !path.is_absolute() || path == Path::new("/") {
            push_limitation(
                limitations,
                format!(
                    "invalid package file path was omitted at line {}",
                    index + 2
                ),
            );
            continue;
        }
        files.push(path);
    }
    Ok(files)
}

fn parse_runtime_dependencies(
    bytes: &[u8],
    dependencies: &mut BTreeMap<PackageIdentity, Vec<PackageIdentity>>,
    limitations: &mut Vec<String>,
) -> Result<(), PackageDataAdapterError> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        PackageDataAdapterError::Malformed("runtime dependencies are not UTF-8".into())
    })?;
    for (index, line) in text.lines().take(MAX_PACKAGE_OUTPUT_LINES).enumerate() {
        let mut fields = line.split_whitespace();
        let Some(name) = fields.next() else {
            continue;
        };
        let identity = PackageIdentity::new(name);
        if identity.validate().is_err() {
            push_limitation(
                limitations,
                format!("invalid dependency owner was omitted at line {}", index + 1),
            );
            continue;
        }
        let mut values = BTreeSet::new();
        let mut in_constraint = false;
        for token in fields {
            if token.starts_with('(') {
                in_constraint = !token.ends_with(')');
                continue;
            }
            if in_constraint {
                in_constraint = !token.ends_with(')');
                continue;
            }
            if token == "|" || token.contains(['<', '>', '=']) {
                continue;
            }
            let dependency = PackageIdentity::new(token);
            if dependency.validate().is_ok() {
                values.insert(dependency);
            } else {
                push_limitation(
                    limitations,
                    format!(
                        "invalid runtime dependency was omitted at line {}",
                        index + 1
                    ),
                );
            }
        }
        dependencies.insert(identity, values.into_iter().collect());
    }
    if text.lines().count() > MAX_PACKAGE_OUTPUT_LINES {
        push_limitation(
            limitations,
            format!("runtime dependencies were limited to {MAX_PACKAGE_OUTPUT_LINES} lines"),
        );
    }
    Ok(())
}

fn append_normalization_limitations(
    limitations: &mut Vec<String>,
    report: &yoctui_model::PackageNormalizationReport,
) {
    if report.invalid_records > 0 {
        push_limitation(
            limitations,
            format!(
                "{} invalid package records were omitted",
                report.invalid_records
            ),
        );
    }
    if report.invalid_fields > 0 {
        push_limitation(
            limitations,
            format!(
                "{} invalid package fields were unavailable",
                report.invalid_fields
            ),
        );
    }
    if report.truncated_records > 0
        || report.truncated_files > 0
        || report.truncated_dependencies > 0
        || report.truncated_image_memberships > 0
    {
        push_limitation(
            limitations,
            "one or more package data collections reached their hard limit".into(),
        );
    }
}
