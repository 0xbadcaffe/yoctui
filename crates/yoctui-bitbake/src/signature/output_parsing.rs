fn parse_signature_dump(
    identity: &SignatureIdentity,
    output: &str,
) -> Result<(SignatureRecord, Vec<String>), SignatureAdapterError> {
    let mut base_hash = None;
    let mut task_hash = None;
    let mut variables = Vec::new();
    let mut task_dependencies = Vec::new();
    let mut dependency_hashes = BTreeMap::new();
    let mut limitations = Vec::new();
    let mut current_variable: Option<(String, String)> = None;
    let mut recognized = 0usize;

    for line in output.lines() {
        if let Some(value) = line.strip_prefix("basehash: ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            base_hash = valid_hash(value).then(|| value.to_owned());
            recognized += 1;
        } else if let Some(value) = line.strip_prefix("Computed task hash is ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            task_hash = valid_hash(value).then(|| value.to_owned());
            recognized += 1;
        } else if let Some(rest) = line.strip_prefix("Variable ") {
            if let Some((name, value)) = rest.split_once(" value is ") {
                finish_variable(&mut current_variable, &mut variables, &mut limitations);
                if valid_field_name(name) {
                    current_variable = Some((name.to_owned(), value.to_owned()));
                } else {
                    push_limitation(
                        &mut limitations,
                        "an invalid signature variable name was dropped".into(),
                    );
                }
                recognized += 1;
            } else if let Some((_, value)) = current_variable.as_mut() {
                append_variable_line(value, line, &mut limitations);
            }
        } else if let Some(value) = line.strip_prefix("Tasks this task depends on: ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            match parse_quoted_list(value) {
                Some(dependencies) => task_dependencies = dependencies,
                None => push_limitation(
                    &mut limitations,
                    "task dependency list could not be parsed".into(),
                ),
            }
            recognized += 1;
        } else if let Some(value) = line.strip_prefix("Hash for dependent task ") {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            if let Some((dependency, hash)) = value.rsplit_once(" is ")
                && valid_field_name(dependency)
                && valid_hash(hash)
            {
                dependency_hashes.insert(dependency.to_owned(), hash.to_owned());
            } else {
                push_limitation(
                    &mut limitations,
                    "a dependent task hash could not be parsed".into(),
                );
            }
            recognized += 1;
        } else if is_dump_header(line) {
            finish_variable(&mut current_variable, &mut variables, &mut limitations);
            recognized += 1;
        } else if let Some((_, value)) = current_variable.as_mut() {
            append_variable_line(value, line, &mut limitations);
        } else if !line.trim().is_empty() {
            push_limitation(
                &mut limitations,
                "unrecognized bitbake-dumpsig output was omitted".into(),
            );
        }
    }
    finish_variable(&mut current_variable, &mut variables, &mut limitations);
    if recognized == 0 {
        return Err(SignatureAdapterError::Malformed(
            "no recognized bitbake-dumpsig records".into(),
        ));
    }
    if variables.len() > MAX_SIGNATURE_VARIABLES {
        variables.sort();
        variables.truncate(MAX_SIGNATURE_VARIABLES);
        push_limitation(
            &mut limitations,
            format!("signature variables were limited to {MAX_SIGNATURE_VARIABLES} entries"),
        );
    }
    let mut dependencies = task_dependencies
        .into_iter()
        .map(|dependency| {
            dependency_hashes
                .get(&dependency)
                .map_or(dependency.clone(), |hash| format!("{dependency}={hash}"))
        })
        .collect::<Vec<_>>();
    for (dependency, hash) in dependency_hashes {
        if !dependencies
            .iter()
            .any(|value| value == &dependency || value.starts_with(&format!("{dependency}=")))
        {
            dependencies.push(format!("{dependency}={hash}"));
        }
    }
    dependencies.sort();
    dependencies.dedup();
    if dependencies.len() > MAX_SIGNATURE_DEPENDENCIES {
        dependencies.truncate(MAX_SIGNATURE_DEPENDENCIES);
        push_limitation(
            &mut limitations,
            format!("signature dependencies were limited to {MAX_SIGNATURE_DEPENDENCIES} entries"),
        );
    }
    if identity.hash.is_some() && task_hash != identity.hash {
        push_limitation(
            &mut limitations,
            "computed task hash did not match the signature filename".into(),
        );
    }
    Ok((
        SignatureRecord {
            identity: identity.clone(),
            base_hash,
            task_hash,
            variables,
            dependencies,
        },
        limitations,
    ))
}

fn is_dump_header(line: &str) -> bool {
    line.starts_with("basehash_ignore_vars:")
        || line.starts_with("taskhash_ignore_tasks:")
        || line.starts_with("Task dependencies:")
        || line.starts_with("List of dependencies for variable ")
        || line.starts_with("This task depends on the checksums of files:")
        || line.starts_with("Computed base hash is ")
        || line == "Unable to compute base hash"
        || line == "Unable to compute task hash"
        || line.starts_with("Tainted (by forced/invalidated task):")
}

fn finish_variable(
    current: &mut Option<(String, String)>,
    variables: &mut Vec<SignatureValue>,
    limitations: &mut Vec<String>,
) {
    let Some((name, mut value)) = current.take() else {
        return;
    };
    if value.len() > MAX_SIGNATURE_OUTPUT_BYTES {
        value.truncate(MAX_SIGNATURE_OUTPUT_BYTES);
        push_limitation(
            limitations,
            format!("signature variable {name} was truncated"),
        );
    }
    variables.push(SignatureValue {
        name,
        value: Some(value),
    });
}

fn append_variable_line(value: &mut String, line: &str, limitations: &mut Vec<String>) {
    if value.len() >= MAX_SIGNATURE_OUTPUT_BYTES {
        push_limitation(
            limitations,
            "a multiline signature variable was truncated".into(),
        );
        return;
    }
    value.push('\n');
    let remaining = MAX_SIGNATURE_OUTPUT_BYTES.saturating_sub(value.len());
    if line.len() <= remaining {
        value.push_str(line);
    } else {
        let boundary = line
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(line.len()))
            .take_while(|index| *index <= remaining)
            .last()
            .unwrap_or(0);
        value.push_str(&line[..boundary]);
    }
}

fn valid_hash(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

fn valid_field_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 1024
        && !value.chars().any(char::is_control)
        && !value.contains('\n')
}

fn parse_quoted_list(value: &str) -> Option<Vec<String>> {
    let value = value.trim();
    if !value.starts_with('[') || !value.ends_with(']') {
        return None;
    }
    let mut values = Vec::new();
    let mut chars = value[1..value.len() - 1].chars().peekable();
    while let Some(character) = chars.next() {
        if character.is_whitespace() || character == ',' {
            continue;
        }
        if character != '\'' {
            return None;
        }
        let mut item = String::new();
        loop {
            match chars.next()? {
                '\\' => item.push(chars.next()?),
                '\'' => break,
                character if character.is_control() => return None,
                character => item.push(character),
            }
        }
        if !valid_field_name(&item) {
            return None;
        }
        values.push(item);
        if values.len() > MAX_SIGNATURE_DEPENDENCIES {
            break;
        }
    }
    Some(values)
}

fn parse_diffsigs_output(output: &str) -> (Vec<SignatureDifference>, Vec<String>) {
    let mut differences = Vec::new();
    let mut limitations = Vec::new();
    let mut represented_lines = BTreeSet::new();
    for (index, line) in output.lines().enumerate() {
        if let Some(value) = line.strip_prefix("basehash changed from ")
            && let Some((left, right)) = value.split_once(" to ")
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::BaseHash,
                key: "base_hash".into(),
                left: Some(left.to_owned()),
                right: Some(right.to_owned()),
            });
            represented_lines.insert(index);
        } else if let Some(value) = line.strip_prefix("Variable ")
            && let Some((name, values)) = value.split_once(" value changed from '")
            && let Some((left, right)) = values.split_once("' to '")
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::ChangedValue,
                key: name.to_owned(),
                left: Some(left.to_owned()),
                right: Some(right.trim_end_matches('\'').to_owned()),
            });
            represented_lines.insert(index);
        } else if let Some(name) = line
            .strip_prefix("Dependency on variable ")
            .and_then(|value| value.strip_suffix(" was added"))
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::Dependency,
                key: name.to_owned(),
                left: None,
                right: Some("present".into()),
            });
            represented_lines.insert(index);
        } else if let Some(name) = line
            .strip_prefix("Dependency on Variable ")
            .and_then(|value| value.strip_suffix(" was removed"))
        {
            differences.push(SignatureDifference {
                category: SignatureDifferenceCategory::Dependency,
                key: name.to_owned(),
                left: Some("present".into()),
                right: None,
            });
            represented_lines.insert(index);
        }
    }
    if output
        .lines()
        .enumerate()
        .any(|(index, line)| !line.trim().is_empty() && !represented_lines.contains(&index))
    {
        push_limitation(
            &mut limitations,
            "some recursive bitbake-diffsigs details were omitted; exact top-level dumps remain available"
                .into(),
        );
    }
    (differences, limitations)
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    yoctui_utils::push_unique_bounded(limitations, limitation, MAX_SIGNATURE_LIMITATIONS);
}
