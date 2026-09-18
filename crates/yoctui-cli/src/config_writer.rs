//! Config writer.
use super::*;

pub(crate) fn bbmask_assignment(value: &str) -> Result<String> {
    if value.contains(['\n', '\r']) {
        anyhow::bail!("BBMASK must be entered on one line");
    }
    Ok(format!(
        "BBMASK = \"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

pub(crate) async fn write_bbmask(build_dir: &Path, value: String) -> Result<()> {
    let path = build_dir.join("conf").join("local.conf");
    tokio::task::spawn_blocking(move || -> Result<()> {
        let assignment = bbmask_assignment(&value)?;
        let mut content = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&assignment);
        content.push('\n');
        fs::write(&path, content).with_context(|| format!("could not write {}", path.display()))
    })
    .await
    .context("BBMASK write task failed")?
}

pub(crate) fn is_exact_config_assignment(line: &str, name: &str) -> bool {
    let line = line.trim_end_matches('\r').trim_start();
    if line.starts_with('#') {
        return false;
    }
    let Some(remainder) = line.strip_prefix(name) else {
        return false;
    };
    if !remainder.chars().next().is_some_and(char::is_whitespace) {
        return false;
    }
    let expression = remainder.trim_start();
    ["??=", "?=", ":=", "+=", "=+", ".=", "=.", "="]
        .iter()
        .any(|operator| expression.starts_with(operator))
}

pub(crate) fn replace_config_assignment(content: &str, name: &str, assignment: &str) -> String {
    let mut output = String::with_capacity(content.len().max(assignment.len()) + 1);
    let mut replaced = false;
    for segment in content.split_inclusive('\n') {
        let ending = if segment.ends_with("\r\n") {
            "\r\n"
        } else if segment.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let body = segment
            .strip_suffix(ending)
            .expect("the detected line ending is a suffix");
        if is_exact_config_assignment(body, name) {
            if !replaced {
                output.push_str(assignment);
                output.push_str(ending);
                replaced = true;
            }
        } else {
            output.push_str(segment);
        }
    }
    if replaced {
        return output;
    }

    let ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(ending);
    }
    output.push_str(assignment);
    output.push_str(ending);
    output
}

pub(crate) fn write_config_assignment_atomic(
    build_dir: &Path,
    request: &ConfigEditRequest,
) -> Result<()> {
    validate_config_edit_request(request, build_dir).map_err(anyhow::Error::msg)?;
    let destination = &request.destination;
    let metadata = fs::metadata(destination)
        .with_context(|| format!("could not inspect {}", destination.display()))?;
    if !metadata.is_file() {
        anyhow::bail!("{} is not a regular file", destination.display());
    }
    let content = fs::read_to_string(destination)
        .with_context(|| format!("could not read {}", destination.display()))?;
    let updated = replace_config_assignment(&content, &request.identity.name, &request.assignment);
    let parent = destination
        .parent()
        .context("configuration destination has no parent directory")?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .context("configuration destination file name is not valid UTF-8")?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temporary = None;
    for attempt in 0..16 {
        let candidate = parent.join(format!(
            ".{file_name}.yoctui-{}-{nonce}-{attempt}.tmp",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "could not create temporary configuration file in {}",
                        parent.display()
                    )
                });
            }
        }
    }
    let (temporary_path, mut temporary_file) =
        temporary.context("could not allocate a unique temporary configuration file")?;
    let write_result = (|| -> Result<()> {
        temporary_file
            .set_permissions(metadata.permissions())
            .with_context(|| {
                format!(
                    "could not preserve permissions on {}",
                    temporary_path.display()
                )
            })?;
        temporary_file
            .write_all(updated.as_bytes())
            .with_context(|| format!("could not write {}", temporary_path.display()))?;
        temporary_file
            .sync_all()
            .with_context(|| format!("could not sync {}", temporary_path.display()))?;
        drop(temporary_file);
        fs::rename(&temporary_path, destination)
            .with_context(|| format!("could not atomically replace {}", destination.display()))?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

pub(crate) async fn write_config_assignment(
    build_dir: &Path,
    request: ConfigEditRequest,
) -> Result<()> {
    let build_dir = build_dir.to_path_buf();
    tokio::task::spawn_blocking(move || write_config_assignment_atomic(&build_dir, &request))
        .await
        .context("configuration write task failed")?
}

pub(crate) fn finish_config_edit_refresh(
    app: &mut App,
    identity: VariableIdentity,
    result: std::result::Result<VariableValue, yoctui_bitbake::BackendError>,
) {
    match result {
        Ok(variable) => {
            let _ = update(
                app,
                config_variable_loaded_action(identity.clone(), variable),
            );
            let _ = update(app, Action::ConfigEditRefreshSucceeded { identity });
        }
        Err(error) => {
            let _ = update(
                app,
                Action::ConfigEditRefreshFailed {
                    identity,
                    message: error.to_string(),
                },
            );
        }
    }
}

pub(crate) async fn execute_config_edit_write(
    backend: &mut dyn BitBakeBackend,
    app: &mut App,
    build_dir: &Path,
    request: ConfigEditRequest,
) {
    let identity = request.identity.clone();
    match write_config_assignment(build_dir, request).await {
        Ok(()) => {
            if let Some(Effect::GetVariable(identity)) = compatibility_workspace_action(
                app,
                Action::ConfigEditWriteSucceeded {
                    identity: identity.clone(),
                },
            ) {
                let result = backend
                    .get_variable(identity.name.clone(), identity.recipe.clone())
                    .await;
                finish_config_edit_refresh(app, identity, result);
            }
        }
        Err(error) => {
            let _ = update(
                app,
                Action::ConfigEditWriteFailed {
                    identity,
                    message: error.to_string(),
                },
            );
        }
    }
}
