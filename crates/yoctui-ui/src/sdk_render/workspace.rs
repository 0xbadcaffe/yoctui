pub(crate) fn sdk_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unavailable", String::as_str);
    let distro = app
        .workspace
        .variables
        .get("DISTRO")
        .map_or("unavailable", String::as_str);
    let target = app.build.target.as_deref().unwrap_or("not selected");
    let root = sdk_inventory_root(app);
    let filtered = app.filtered_sdk_artifacts();
    let filtered_selection = filtered
        .iter()
        .position(|artifact| app.sdk_artifact_selection.as_ref() == Some(&artifact.identity));
    let mut lines = vec![
        Line::from(format!(
            "MACHINE {machine} | DISTRO {distro} | image {target}"
        )),
        Line::from(format!("SDK_DEPLOY {root}")),
    ];
    lines.push(search_line(
        app,
        &app.sdk_artifact_query,
        app.sdk_artifact_searching,
        filtered_selection,
        filtered.len(),
        SearchNavigation::Results,
        SearchExit::Done,
        area.width.saturating_sub(2),
    ));
    lines.push(Line::from(
        "  Kind       SDK type     Size       Modified     Published  Artifact",
    ));
    match &app.sdk_artifacts {
        SdkArtifactInventoryState::NotLoaded => {
            lines.push(Line::from(
                "SDK artifacts are not loaded. Press R to scan SDK_DEPLOY.",
            ));
        }
        SdkArtifactInventoryState::Loading { request } => {
            lines.push(Line::from(format!(
                "Loading SDK artifacts (generation {})…",
                request.generation
            )));
        }
        SdkArtifactInventoryState::AvailableEmpty { .. } => {
            lines.push(Line::from(
                "No SDK artifacts were found in the authoritative SDK_DEPLOY root.",
            ));
        }
        SdkArtifactInventoryState::Failed { message, .. } => {
            lines.push(Line::from(format!("SDK artifact scan failed: {message}")));
        }
        SdkArtifactInventoryState::Available { .. } | SdkArtifactInventoryState::Partial { .. } => {
            let artifacts = app.filtered_sdk_artifacts();
            if artifacts.is_empty() {
                lines.push(Line::from("No SDK artifacts match the active search."));
            } else {
                let selected_index = artifacts
                    .iter()
                    .position(|artifact| {
                        app.sdk_artifact_selection.as_ref() == Some(&artifact.identity)
                    })
                    .unwrap_or(0);
                let capacity = usize::from(area.height.saturating_sub(7)).max(1);
                let start = selected_index
                    .saturating_sub(capacity / 2)
                    .min(artifacts.len().saturating_sub(capacity));
                for artifact in artifacts.into_iter().skip(start).take(capacity) {
                    let selected = app.sdk_artifact_selection.as_ref() == Some(&artifact.identity);
                    let published = artifact.published.map_or("unavailable", |published| {
                        if published { "yes" } else { "no" }
                    });
                    let file = artifact.identity.path.file_name().map_or_else(
                        || "unavailable".into(),
                        |name| name.to_string_lossy().into_owned(),
                    );
                    lines.push(
                        Line::from(format!(
                            "{} {:<10} {:<12} {:<10} {:<12} {:<10} {}",
                            if selected { "▶" } else { " " },
                            sdk_kind_label(artifact.kind),
                            sdk_type_label(artifact.sdk_kind),
                            artifact.identity.size_bytes,
                            artifact.identity.modified_unix_seconds,
                            published,
                            file,
                        ))
                        .style(selected_style(app, selected)),
                    );
                }
            }
            if let SdkArtifactInventoryState::Partial { limitations, .. } = &app.sdk_artifacts {
                lines.push(Line::from(format!(
                    "Partial SDK inventory: {} limitation(s); see Inspector.",
                    limitations.len()
                )));
            }
        }
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(app, "SDK", app.focus == FocusTarget::Workspace))
            .wrap(Wrap { trim: false }),
        area,
    );
}

pub(crate) fn sdk_capability_text(app: &App) -> String {
    match &app.sdk_tool_capability {
        SdkToolCapability::NotInspected => "not inspected".into(),
        SdkToolCapability::Failed { message } => format!("inspection failed: {message}"),
        SdkToolCapability::Available {
            publish,
            find_sysroot,
            run_native,
        } => format!(
            "oe-publish-sdk: {}\noe-find-native-sysroot: {}\noe-run-native: {}",
            publish
                .as_ref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            find_sysroot
                .as_ref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
            run_native
                .as_ref()
                .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        ),
    }
}

pub(crate) fn sdk_association_text(paths: &[std::path::PathBuf]) -> String {
    if paths.is_empty() {
        "none".into()
    } else {
        paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

pub(crate) fn background_status_label(status: BackgroundJobStatus) -> &'static str {
    match status {
        BackgroundJobStatus::Queued => "queued",
        BackgroundJobStatus::Starting => "starting",
        BackgroundJobStatus::Running => "running",
        BackgroundJobStatus::Cancelling => "cancelling",
        BackgroundJobStatus::Succeeded => "succeeded",
        BackgroundJobStatus::Failed => "failed",
        BackgroundJobStatus::Cancelled => "cancelled",
        BackgroundJobStatus::Lost => "lost",
    }
}

pub(crate) fn sdk_session_operation_text(operation: &SdkOperation) -> String {
    match operation {
        SdkOperation::Publish(request) => format!(
            "publish\nInstaller: {}\nDestination: {}",
            request.artifact.path.display(),
            request.destination.display()
        ),
        SdkOperation::Native(request) => format!(
            "{:?}\nWorkspace: {}\nRecipe: {}\nTool: {}\nArguments: {}",
            request.mode,
            request
                .extracted_root
                .as_ref()
                .map_or("active build", |path| path
                    .to_str()
                    .unwrap_or("unavailable")),
            request.recipe,
            request.tool.as_deref().unwrap_or("unavailable"),
            if request.arguments.is_empty() {
                "none".into()
            } else {
                request.arguments.join(" ")
            }
        ),
    }
}

pub(crate) fn sdk_session_text(app: &App) -> String {
    let Some(session) = app.latest_sdk_session() else {
        return "Managed SDK operation\nNo publication or native-tool operation has been started."
            .into();
    };
    let Some(job) = app.background_jobs.get(session.background_job_id) else {
        return format!(
            "Managed SDK operation {}\nLifecycle record unavailable.",
            session.id.0
        );
    };
    let mut retained = job.output.iter().rev().take(80).collect::<Vec<_>>();
    retained.reverse();
    let output = if retained.is_empty() {
        "none".into()
    } else {
        retained
            .into_iter()
            .map(|entry| {
                let source = match entry.source {
                    BackgroundJobOutputSource::Backend => "backend",
                    BackgroundJobOutputSource::Stdout => "stdout",
                    BackgroundJobOutputSource::Stderr => "stderr",
                };
                format!(
                    "[{source}] {}{}",
                    entry.message,
                    if entry.truncated { " [truncated]" } else { "" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let result = job
        .result
        .as_ref()
        .map_or_else(|| "none".into(), |result| result.summary.clone());
    let result_artifacts = job.result.as_ref().map_or_else(
        || "none".into(),
        |result| {
            if result.artifacts.is_empty() {
                "none".into()
            } else {
                result
                    .artifacts
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        },
    );
    let error = job.error.as_ref().map_or_else(
        || session.error_detail.as_deref().unwrap_or("none").into(),
        |error| {
            error.detail.as_ref().map_or_else(
                || error.summary.clone(),
                |detail| format!("{}: {detail}", error.summary),
            )
        },
    );
    let history = app
        .sdk_sessions
        .iter()
        .rev()
        .take(8)
        .filter_map(|record| {
            app.background_jobs
                .get(record.background_job_id)
                .map(|job| {
                    format!(
                        "#{} {} {}",
                        record.id.0,
                        match &record.operation {
                            SdkOperation::Publish(_) => "publish",
                            SdkOperation::Native(_) => "native",
                        },
                        background_status_label(job.status)
                    )
                })
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Managed SDK operation {}\nStatus: {}\n{}\nQueued: {}\nStarted: {}\nFinished: {}\nExit code: {}\nResult: {}\nResult artifacts:\n{}\nError: {}\nRetained output: {} entries (showing latest {})\nDropped output: {} entries\nWarnings: {}  Errors: {}\n\nOutput:\n{}\n\nRecent SDK history ({} retained):\n{}",
        session.id.0,
        background_status_label(job.status),
        sdk_session_operation_text(&session.operation),
        timestamp_text(job.queued_at),
        job.started_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        job.finished_at
            .map(timestamp_text)
            .unwrap_or_else(|| "unavailable".into()),
        session
            .exit_code
            .map_or_else(|| "unavailable".into(), |code| code.to_string()),
        result,
        result_artifacts,
        error,
        job.output.len(),
        job.output.len().min(80),
        job.dropped_output_entries,
        job.warnings,
        job.errors,
        output,
        app.sdk_sessions.len(),
        if history.is_empty() { "none" } else { &history },
    )
}

pub(crate) fn sdk_inspector_text(app: &App) -> String {
    let limitations = match &app.sdk_artifacts {
        SdkArtifactInventoryState::Partial { limitations, .. } => limitations
            .iter()
            .map(|limitation| format!("! {limitation}"))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => "none".into(),
    };
    let artifact = app.selected_sdk_artifact().map_or_else(
        || {
            "No SDK artifact selected.\nPress R to scan SDK_DEPLOY or adjust the active search."
                .into()
        },
        |artifact| {
            format!(
                "Path: {}\nKind: {}\nSDK type: {}\nMachine: {}\nHost tuple: {}\nTarget tuple: {}\nSize: {} bytes\nModified: {}s since Unix epoch\nPublished: {}\n\nChecksums:\n{}\n\nManifests:\n{}",
                artifact.identity.path.display(),
                sdk_kind_label(artifact.kind),
                sdk_type_label(artifact.sdk_kind),
                artifact.machine.as_deref().unwrap_or("unavailable"),
                artifact.host_tuple.as_deref().unwrap_or("unavailable"),
                artifact.target_tuple.as_deref().unwrap_or("unavailable"),
                artifact.identity.size_bytes,
                artifact.identity.modified_unix_seconds,
                artifact.published.map_or("unavailable", |published| if published {
                    "yes"
                } else {
                    "no"
                }),
                sdk_association_text(&artifact.checksums),
                sdk_association_text(&artifact.manifests),
            )
        },
    );
    format!(
        "SDK tool capability\n{}\n\n{}\n\nSelected artifact\n{}\n\nScan limitations\n{}",
        sdk_capability_text(app),
        sdk_session_text(app),
        artifact,
        limitations,
    )
}
