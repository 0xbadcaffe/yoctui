//! Sdk render.
use super::*;

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

pub(crate) fn sdk_popup(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    tone: DialogTone,
    content: String,
    preferred_height: u16,
) {
    let popup = dialog_popup_rect(area, 92, preferred_height);
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(content)
            .block(dialog_block(app, title, tone))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn sdk_build_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &yoctui_model::SdkBuildPreview,
    area: Rect,
) {
    let action = match preview.action {
        SdkBuildAction::Populate(SdkKind::Standard) => "populate standard SDK",
        SdkBuildAction::Populate(SdkKind::Extensible) => "populate extensible SDK",
        SdkBuildAction::Test(SdkKind::Standard) => "run testsdk",
        SdkBuildAction::Test(SdkKind::Extensible) => "run testsdkext",
    };
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK build",
        DialogTone::Confirmation,
        format!(
            "Action: {action}\nMachine: {}\nDistro: {}\nImage target: {}\nBitBake task: {}\n\nEnter starts the managed BitBake build.\nEsc closes without starting.",
            preview.machine,
            preview.distro,
            preview.image,
            preview.request.task.as_deref().unwrap_or("unavailable"),
        ),
        12,
    );
}

pub(crate) fn sdk_publish_dialog(
    frame: &mut Frame,
    app: &App,
    draft: &SdkPublishDraft,
    area: Rect,
) {
    let installer = app.selected_sdk_artifact().map_or_else(
        || "unavailable".into(),
        |artifact| artifact.identity.path.display().to_string(),
    );
    let validation = app
        .sdk_tool_capability
        .publish_executable()
        .and_then(|executable| {
            app.selected_sdk_artifact()
                .ok_or("an installer selection is required")
                .and_then(|artifact| {
                    SdkPublishPreview::new(
                        executable,
                        artifact.identity.clone(),
                        std::path::PathBuf::from(&draft.destination),
                    )
                    .map(|_| ())
                })
        })
        .map_or_else(
            |message| format!("✕ Validation: {message}"),
            |()| "✓ Validation: ready for exact preview".into(),
        );
    sdk_popup(
        frame,
        app,
        area,
        "Publish SDK installer",
        DialogTone::Standard,
        format!(
            "Tool: {}\nInstaller [read-only]: {installer}\nDestination [editing]: {}_\n{validation}\n\nDestination must be an absolute canonical empty directory.\nEnter validates and opens the exact argument preview.\nEsc closes without publishing.",
            app.sdk_tool_capability
                .publish_executable()
                .map_or_else(|_| "unavailable".into(), |path| path.display().to_string()),
            draft.destination,
        ),
        13,
    );
}

pub(crate) fn indexed_path_vector(paths: &[std::path::PathBuf]) -> String {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| format!("[{index}] {}", path.display()))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn sdk_publish_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &SdkPublishPreview,
    area: Rect,
) {
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK publication",
        DialogTone::Confirmation,
        format!(
            "Installer: {}\nDestination: {}\n\nExact indexed shell-free argument vector:\n{}\n\nNo overwrite policy is guessed.\nEnter starts the managed publication job.\nEsc closes without publishing.",
            preview.request.artifact.path.display(),
            preview.request.destination.display(),
            indexed_path_vector(&preview.argv),
        ),
        18,
    );
}

pub(crate) fn sdk_native_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &SdkNativeDialog,
    area: Rect,
) {
    let draft = &dialog.draft;
    let arguments = if dialog.arguments_input.is_empty() {
        "none".into()
    } else {
        dialog.arguments_input.clone()
    };
    let validation = app
        .sdk_tool_capability
        .executable_for(draft.mode)
        .and_then(|executable| {
            yoctui_model::SdkNativePreview::new(yoctui_model::SdkNativeRequest {
                executable,
                mode: draft.mode,
                extracted_root: (!draft.extracted_root.is_empty())
                    .then(|| std::path::PathBuf::from(&draft.extracted_root)),
                recipe: draft.recipe.clone(),
                tool: (draft.mode == SdkNativeMode::RunNative).then(|| draft.tool.clone()),
                arguments: draft.arguments.clone(),
            })
            .map(|_| ())
        })
        .map_or_else(
            |message| format!("✕ Validation: {message}"),
            |()| "✓ Validation: ready for exact preview".into(),
        );
    let row = |field: SdkNativeField, label: &str, value: &str| {
        let marker = if dialog.selected_field == field {
            "▶"
        } else {
            " "
        };
        let editing = if dialog.selected_field == field && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        format!("{marker} {label}: {value}{editing}")
    };
    let validation = dialog
        .validation_error
        .as_ref()
        .map_or(validation, |message| format!("✕ Validation: {message}"));
    sdk_popup(
        frame,
        app,
        area,
        "SDK native tool",
        DialogTone::Standard,
        format!(
            "{}\nExecutable: {}\n{}\n{}\n{}\n{}\n{validation}\n\n↑/↓ select · Enter edit/cycle · ←/→ mode · p preview · Esc close",
            row(SdkNativeField::Mode, "Mode", &format!("{:?}", draft.mode)),
            app.sdk_tool_capability
                .executable_for(draft.mode)
                .map_or_else(|_| "unavailable".into(), |path| path.display().to_string()),
            row(
                SdkNativeField::Workspace,
                "Workspace",
                if draft.extracted_root.is_empty() {
                    "active build"
                } else {
                    &draft.extracted_root
                },
            ),
            row(
                SdkNativeField::Recipe,
                "Recipe",
                if draft.recipe.is_empty() {
                    "unavailable"
                } else {
                    &draft.recipe
                },
            ),
            row(
                SdkNativeField::Tool,
                "Tool",
                if draft.mode == SdkNativeMode::FindSysroot {
                    "not applicable"
                } else if draft.tool.is_empty() {
                    "unavailable"
                } else {
                    &draft.tool
                },
            ),
            row(SdkNativeField::Arguments, "Arguments", &arguments),
        ),
        18,
    );
}

pub(crate) fn sdk_native_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &SdkNativePreview,
    area: Rect,
) {
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK native tool",
        DialogTone::Confirmation,
        format!(
            "Mode: {:?}\nWorkspace: {}\nRecipe: {}\nTool: {}\n\nExact indexed shell-free argument vector:\n{}\n\nEnvironment changes apply only to the managed child.\nEnter starts the operation.\nEsc closes without starting.",
            preview.request.mode,
            preview
                .request
                .extracted_root
                .as_ref()
                .map_or("active build".into(), |path| path.display().to_string()),
            preview.request.recipe,
            preview.request.tool.as_deref().unwrap_or("not applicable"),
            indexed_path_vector(&preview.argv),
        ),
        19,
    );
}

pub(crate) fn sdk_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: SdkSessionId,
    area: Rect,
) {
    let operation = app
        .sdk_session(id)
        .map_or("unavailable", |session| match &session.operation {
            SdkOperation::Publish(_) => "publication",
            SdkOperation::Native(_) => "native tool",
        });
    sdk_popup(
        frame,
        app,
        area,
        "Confirm SDK cancellation",
        DialogTone::Confirmation,
        format!(
            "Cancel managed SDK {operation} operation #{}?\n\nRetained output and the terminal cancellation result remain in SDK history.\nEnter requests cancellation.\nEsc keeps the operation running.",
            id.0
        ),
        9,
    );
}

pub(crate) fn qemu_launch_field_label(field: QemuLaunchField) -> &'static str {
    match field {
        QemuLaunchField::Machine => "Machine",
        QemuLaunchField::Image => "Image",
        QemuLaunchField::Kernel => "Kernel",
        QemuLaunchField::Rootfs => "Root filesystem",
        QemuLaunchField::Networking => "Networking",
        QemuLaunchField::Memory => "Memory MiB",
        QemuLaunchField::Display => "Display",
        QemuLaunchField::Serial => "Serial",
        QemuLaunchField::ExtraArguments => "Extra arguments",
    }
}

pub(crate) fn image_console_field_label(field: ImageConsoleField) -> &'static str {
    match field {
        ImageConsoleField::Mode => "Mode",
        ImageConsoleField::Image => "Selected image",
        ImageConsoleField::Networking => "QEMU networking",
        ImageConsoleField::Memory => "QEMU memory MiB",
        ImageConsoleField::Host => "SSH host",
        ImageConsoleField::User => "SSH user",
        ImageConsoleField::Port => "SSH port",
        ImageConsoleField::IdentityFile => "SSH identity file",
    }
}

pub(crate) fn image_console_field_value(
    dialog: &ImageConsoleDialog,
    field: ImageConsoleField,
) -> String {
    match field {
        ImageConsoleField::Mode => dialog.draft.mode.label().into(),
        ImageConsoleField::Image => dialog.draft.image.path.display().to_string(),
        ImageConsoleField::Networking => format!("{:?}", dialog.draft.networking),
        ImageConsoleField::Memory => dialog.draft.memory_mib.clone(),
        ImageConsoleField::Host => {
            if dialog.draft.host.is_empty() {
                "required".into()
            } else {
                dialog.draft.host.clone()
            }
        }
        ImageConsoleField::User => dialog.draft.user.clone(),
        ImageConsoleField::Port => dialog.draft.port.clone(),
        ImageConsoleField::IdentityFile => {
            if dialog.draft.identity_file.is_empty() {
                "default SSH configuration".into()
            } else {
                dialog.draft.identity_file.clone()
            }
        }
    }
}

pub(crate) fn image_console_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &ImageConsoleDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 104, 20);
    clear_popup(frame, app, popup);
    let shell = dialog_shell(app, "Image Console", DialogTone::Standard);
    let block = shell.clone().block();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let regions = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(inner);
    let purpose = match dialog.draft.mode {
        ImageConsoleMode::Qemu => format!(
            "Boots {} with runqemu in a daemon-owned PTY. nographic + serialstdio are enforced.",
            dialog.draft.image.image
        ),
        ImageConsoleMode::Ssh => format!(
            "Connects to an already-running target. OpenSSH host-key policy stays enabled. {}",
            app.ssh_client_capability.status_text()
        ),
    };
    frame.render_widget(
        Paragraph::new(purpose).wrap(Wrap { trim: true }),
        regions[0],
    );
    let rows = dialog.fields().iter().copied().map(|field| {
        let selected = field == dialog.selected_field;
        Row::new([
            format!(
                "{} {}{}",
                if selected { "▶" } else { " " },
                image_console_field_label(field),
                if field.is_read_only() {
                    " [read-only]"
                } else {
                    ""
                }
            ),
            image_console_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    frame.render_widget(
        Table::new(rows, [Constraint::Length(26), Constraint::Min(1)])
            .header(Row::new(["Field", "Value"]).style(Style::default().bold())),
        regions[1],
    );
    let validation = dialog.validation_error.as_deref().map_or_else(
        || match dialog.draft.mode {
            ImageConsoleMode::Qemu => format!("runqemu: {}", qemu_capability_text(app)),
            ImageConsoleMode::Ssh => {
                "Password prompts stay inside the PTY; credentials are never stored.".into()
            }
        },
        |message| format!("Cannot launch: {message}"),
    );
    frame.render_widget(
        Paragraph::new(validation).wrap(Wrap { trim: true }),
        regions[2],
    );
    frame.render_widget(
        Paragraph::new(shell.controls(
            Some(("Enter", "Launch")),
            &[("↑/↓", "Field"), ("←/→", "Choice"), ("Esc", "Cancel")],
        )),
        regions[3],
    );
}

pub(crate) fn qemu_launch_field_value(dialog: &QemuLaunchDialog, field: QemuLaunchField) -> String {
    match field {
        QemuLaunchField::Machine => dialog.draft.machine.clone(),
        QemuLaunchField::Image => dialog.draft.image.path.display().to_string(),
        QemuLaunchField::Kernel => {
            if dialog.draft.kernel.is_empty() {
                "not set".into()
            } else {
                dialog.draft.kernel.clone()
            }
        }
        QemuLaunchField::Rootfs => {
            if dialog.draft.rootfs.is_empty() {
                "not set".into()
            } else {
                dialog.draft.rootfs.clone()
            }
        }
        QemuLaunchField::Networking => match dialog.draft.networking {
            QemuNetworkingMode::Slirp => "slirp",
            QemuNetworkingMode::Tap => "tap",
            QemuNetworkingMode::None => "none",
        }
        .into(),
        QemuLaunchField::Memory => dialog.draft.memory_mib.clone(),
        QemuLaunchField::Display => match dialog.draft.display {
            QemuDisplayMode::Graphical => "graphical",
            QemuDisplayMode::Nographic => "nographic",
        }
        .into(),
        QemuLaunchField::Serial => match dialog.draft.serial {
            QemuSerialMode::Stdio => "stdio",
            QemuSerialMode::Telnet => "telnet",
            QemuSerialMode::None => "none",
        }
        .into(),
        QemuLaunchField::ExtraArguments => {
            if dialog.draft.extra_arguments.is_empty() {
                "none".into()
            } else {
                dialog.draft.extra_arguments.clone()
            }
        }
    }
}

pub(crate) fn qemu_launch_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &QemuLaunchDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let shell = dialog_shell(app, "Launch runqemu", DialogTone::Standard);
    let block = shell.clone().block();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let regions = Layout::vertical(if dialog.validation_error.is_some() {
        vec![
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(4), Constraint::Length(1)]
    })
    .split(inner);
    let fields = [
        QemuLaunchField::Machine,
        QemuLaunchField::Image,
        QemuLaunchField::Kernel,
        QemuLaunchField::Rootfs,
        QemuLaunchField::Networking,
        QemuLaunchField::Memory,
        QemuLaunchField::Display,
        QemuLaunchField::Serial,
        QemuLaunchField::ExtraArguments,
    ];
    let rows = fields.into_iter().map(|field| {
        let selected = dialog.selected_field == field;
        let suffix = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([
            format!(
                "{} {}{}",
                if selected { "▶" } else { " " },
                qemu_launch_field_label(field),
                suffix
            ),
            qemu_launch_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    frame.render_widget(
        Table::new(rows, [Constraint::Length(23), Constraint::Min(1)]).header(
            Row::new(["Field", "Value"]).style(
                ThemePalette::for_app(app)
                    .role(ThemePalette::for_app(app).table_header, Modifier::BOLD),
            ),
        ),
        regions[0],
    );
    if let Some(message) = &dialog.validation_error {
        frame.render_widget(
            Paragraph::new(shell.validation(Some(message))).wrap(Wrap { trim: false }),
            regions[1],
        );
    }
    let controls = *regions.last().expect("dialog has controls");
    frame.render_widget(
        Paragraph::new(shell.controls(
            Some(("p", "Preview")),
            &[
                ("↑/↓", "Field"),
                ("←/→", "Choice"),
                ("Enter", "Edit"),
                ("Esc", "Close"),
            ],
        )),
        controls,
    );
}

pub(crate) fn qemu_launch_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &QemuLaunchPreview,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 18);
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!("Machine: {}", preview.request.machine)),
        Line::from(format!("Image: {}", preview.request.image.image)),
        Line::from(format!(
            "Artifact: {}",
            preview.request.image.path.display()
        )),
        Line::from("Exact argument vector (one argument per line):"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}] {}", argument.display()))),
    );
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Enter confirms launch. Esc closes without launch.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Confirm managed runqemu launch",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn qemu_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: QemuSessionId,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 72, 7);
    clear_popup(frame, app, popup);
    let detail = app.qemu_session(id).map_or_else(
        || format!("Session {} is no longer available.", id.0),
        |session| {
            format!(
                "Cancel managed session {}?\nImage: {}\nArtifact: {}",
                id.0,
                session.request.image.image,
                session.request.image.path.display()
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(dialog_block(
            app,
            "Confirm runqemu cancellation",
            DialogTone::Confirmation,
        ))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_field_value(dialog: &WicCreateDialog, field: WicCreateField) -> String {
    match field {
        WicCreateField::Machine => dialog.draft.machine.clone(),
        WicCreateField::Image => dialog.draft.image.clone(),
        WicCreateField::Kickstart => dialog.draft.kickstart.name.clone(),
        WicCreateField::OutputDirectory => dialog.draft.output_directory.clone(),
        WicCreateField::GenerateBmap => if dialog.draft.generate_bmap {
            "yes"
        } else {
            "no"
        }
        .into(),
        WicCreateField::Compression => match dialog.draft.compression {
            WicCompression::None => "none",
            WicCompression::Gzip => "gzip",
            WicCompression::Bzip2 => "bzip2",
            WicCompression::Xz => "xz",
        }
        .into(),
    }
}

pub(crate) fn wic_partition_summary(kickstart: &WicKickstart) -> String {
    if kickstart.partitions.is_empty() {
        return "none reported".into();
    }
    kickstart
        .partitions
        .iter()
        .enumerate()
        .map(|(index, partition)| {
            format!(
                "{}: mount={} fs={} source={} size={} MiB align={} KiB",
                index + 1,
                partition.mount_point.as_deref().unwrap_or("unavailable"),
                partition.filesystem.as_deref().unwrap_or("unavailable"),
                partition.source_plugin.as_deref().unwrap_or("unavailable"),
                partition
                    .size_mib
                    .map_or_else(|| "dynamic".into(), |value| value.to_string()),
                partition
                    .alignment_kib
                    .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn wic_limitations(kickstart: &WicKickstart) -> String {
    if kickstart.limitations.is_empty() {
        "none".into()
    } else {
        kickstart.limitations.join("\n")
    }
}

pub(crate) fn wic_create_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &WicCreateDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let shell = dialog_shell(app, "Create Wic", DialogTone::Standard);
    let block = shell.clone().block();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let regions = Layout::vertical(if dialog.validation_error.is_some() {
        vec![
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(4), Constraint::Length(1)]
    })
    .split(inner);
    let fields = [
        WicCreateField::Machine,
        WicCreateField::Image,
        WicCreateField::Kickstart,
        WicCreateField::OutputDirectory,
        WicCreateField::GenerateBmap,
        WicCreateField::Compression,
    ];
    let labels = [
        "Machine",
        "Image",
        "Kickstart",
        "Output directory",
        "Generate bmap",
        "Compression",
    ];
    let rows = fields.into_iter().zip(labels).map(|(field, label)| {
        let selected = dialog.selected_field == field;
        let marker = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([
            format!("{} {label}{marker}", if selected { "▶" } else { " " }),
            wic_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    frame.render_widget(
        Table::new(rows, [Constraint::Length(27), Constraint::Min(1)]).header(
            Row::new(["Field", "Value"]).style(
                ThemePalette::for_app(app)
                    .role(ThemePalette::for_app(app).table_header, Modifier::BOLD),
            ),
        ),
        regions[0],
    );
    if let Some(message) = &dialog.validation_error {
        frame.render_widget(
            Paragraph::new(shell.validation(Some(message))).wrap(Wrap { trim: false }),
            regions[1],
        );
    }
    let controls = *regions.last().expect("dialog has controls");
    frame.render_widget(
        Paragraph::new(shell.controls(
            Some(("p", "Preview")),
            &[
                ("↑/↓", "Field"),
                ("←/→", "Choice"),
                ("Enter", "Edit"),
                ("Esc", "Close"),
            ],
        )),
        controls,
    );
}

pub(crate) fn wic_create_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &WicCreatePreview,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, area.height.saturating_sub(2).min(30));
    clear_popup(frame, app, popup);
    let partitions = wic_partition_summary(&preview.kickstart);
    let limitations = wic_limitations(&preview.kickstart);
    let argv = preview
        .argv
        .iter()
        .enumerate()
        .map(|(index, argument)| format!("[{index}]={}", argument.display()))
        .collect::<Vec<_>>()
        .join("  ");
    let source_line_count = preview.kickstart.source.lines().count();
    let source_limit = if popup.height <= 22 { 2 } else { 6 };
    let mut lines = vec![
        Line::from("Confirm managed Wic creation"),
        Line::from(format!(
            "Machine: {} | Image: {}",
            preview.request.machine, preview.request.image
        )),
        Line::from(format!(
            "Kickstart: {} | Output: {}",
            preview.request.kickstart.name,
            preview.request.output_directory.display()
        )),
        Line::from(""),
        Line::from(format!(
            "Kickstart source (showing {} of {} lines):",
            source_line_count.min(source_limit),
            source_line_count
        )),
    ];
    let mut source = source_preview(
        &preview
            .kickstart
            .source
            .lines()
            .take(source_limit)
            .collect::<Vec<_>>()
            .join("\n"),
        preview
            .kickstart
            .identity
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("kickstart.wks"),
        app,
    );
    lines.append(&mut source.lines);
    lines.extend([
        Line::from(""),
        Line::from(format!("Partitions: {partitions}")),
        Line::from(format!("Limitations: {limitations}")),
        Line::from(""),
        Line::from(format!("Exact argument vector: {argv}")),
        Line::from(""),
        Line::from("Enter confirms creation. Esc closes."),
    ]);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(dialog_block(
                app,
                "Confirm managed Wic creation",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_device_lines(
    app: &App,
    devices: &[WicDevice],
    available_lines: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let selection = devices
        .iter()
        .position(|device| app.wic_device_selection.as_ref() == Some(&device.identity));
    let capacity = (available_lines / 3).max(1);
    let viewport = yoctui_model::centered_viewport_range(selection, devices.len(), capacity);
    for device in &devices[viewport] {
        let selected = app.wic_device_selection.as_ref() == Some(&device.identity);
        let style = selected_style(app, selected);
        let mounts = if device.descendant_mounts.is_empty() {
            "none".into()
        } else {
            device
                .descendant_mounts
                .iter()
                .map(|mount| mount.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        lines.push(
            Line::from(format!(
                "{} {} | {} | {} | major:minor {}",
                if selected { "▶" } else { " " },
                device.identity.path.display(),
                format_bytes(device.identity.size_bytes),
                device.identity.size_bytes,
                device.identity.major_minor,
            ))
            .style(style),
        );
        lines.push(
            Line::from(format!(
                "  model={} | serial={} | transport={} | removable={} | writable={} | read-only={} | mounts={}",
                device.identity.model.as_deref().unwrap_or("unavailable"),
                device.identity.serial.as_deref().unwrap_or("unavailable"),
                device.identity.transport.as_deref().unwrap_or("unavailable"),
                device.removable,
                device.writable,
                device.read_only,
                mounts,
            ))
            .style(style),
        );
        if let Some(reason) = &device.unavailable_reason {
            lines.push(Line::from(format!("  unavailable: {reason}")).style(style));
        }
    }
    lines
}

pub(crate) fn wic_device_picker(
    frame: &mut Frame,
    app: &App,
    dialog: &WicDevicePickerDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 110, area.height.saturating_sub(2).min(30));
    let device_lines = usize::from(popup.height.saturating_sub(9)).max(1);
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!(
            "Image: {} | {} bytes | modified {}s",
            dialog.request.image.path.display(),
            dialog.request.image.size_bytes,
            dialog.request.image.modified_unix_seconds,
        )),
        Line::from(
            "Only removable, writable whole devices without mounted descendants are eligible.",
        ),
        Line::from(""),
    ];
    match &app.wic_devices {
        WicDeviceInventoryState::Loading { request } if request == &dialog.request => {
            lines.push(Line::from("Discovering removable whole devices…"));
        }
        WicDeviceInventoryState::Available { request, devices } if request == &dialog.request => {
            if devices.is_empty() {
                lines.push(Line::from(
                    "No eligible removable whole devices were found.",
                ));
            } else {
                lines.extend(wic_device_lines(app, devices, device_lines));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Discovery limitations: none"));
        }
        WicDeviceInventoryState::Partial {
            request,
            devices,
            limitations,
        } if request == &dialog.request => {
            if devices.is_empty() {
                lines.push(Line::from(
                    "No eligible removable whole devices were found.",
                ));
            } else {
                lines.extend(wic_device_lines(app, devices, device_lines));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Discovery limitations: {}",
                limitations.join("; ")
            )));
        }
        WicDeviceInventoryState::Failed { request, message } if request == &dialog.request => {
            lines.push(Line::from(format!("Device discovery failed: {message}")));
        }
        _ => lines.push(Line::from(
            "This device inventory is stale; close and start a new discovery.",
        )),
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "↑/↓ selects. Enter opens the required phrase dialog. Esc closes.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Select protected Wic write device",
                DialogTone::Standard,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_write_phrase_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &WicWritePhraseDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 15);
    clear_popup(frame, app, popup);
    let expected = format!("WRITE {}", dialog.device.path.display());
    let mut lines = vec![
        Line::from(format!(
            "Image: {} | {} bytes",
            dialog.request.image.path.display(),
            dialog.request.image.size_bytes,
        )),
        Line::from(format!(
            "Device: {} | major:minor {} | {} bytes",
            dialog.device.path.display(),
            dialog.device.major_minor,
            dialog.device.size_bytes,
        )),
        Line::from(format!(
            "Model: {} | Serial: {} | Transport: {}",
            dialog.device.model.as_deref().unwrap_or("unavailable"),
            dialog.device.serial.as_deref().unwrap_or("unavailable"),
            dialog.device.transport.as_deref().unwrap_or("unavailable"),
        )),
        Line::from(""),
        Line::from(format!("Required phrase: {expected}")),
        Line::from(format!("Input: {}_", dialog.input)),
    ];
    if let Some(error) = &dialog.validation_error {
        let palette = ThemePalette::for_app(app);
        lines.push(
            Line::from(format!("✕ Validation: {error}"))
                .style(palette.role(palette.error, Modifier::BOLD)),
        );
    }
    lines.extend([
        Line::from(""),
        Line::from(
            "The phrase alone does not write. Enter opens a separate exact command preview.",
        ),
        Line::from("Esc closes without writing."),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Confirm protected Wic device identity",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_write_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &WicWritePreview,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 110, area.height.saturating_sub(2).min(25));
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from("DESTRUCTIVE OPERATION: this overwrites the selected whole device."),
        Line::from(""),
        Line::from(format!(
            "Image: {} | {} bytes | modified {}s",
            preview.request.image.path.display(),
            preview.request.image.size_bytes,
            preview.request.image.modified_unix_seconds,
        )),
        Line::from(format!(
            "Device: {} | major:minor {} | {} bytes",
            preview.request.device.path.display(),
            preview.request.device.major_minor,
            preview.request.device.size_bytes,
        )),
        Line::from(format!(
            "Model: {} | Serial: {} | Transport: {}",
            preview
                .request
                .device
                .model
                .as_deref()
                .unwrap_or("unavailable"),
            preview
                .request
                .device
                .serial
                .as_deref()
                .unwrap_or("unavailable"),
            preview
                .request
                .device
                .transport
                .as_deref()
                .unwrap_or("unavailable"),
        )),
        Line::from(""),
        Line::from("Exact argument vector:"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}]={}", argument.display()))),
    );
    lines.extend([
        Line::from(""),
        Line::from("Enter starts WRITE DEVICE. Esc closes without writing."),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Final protected Wic device-write preview",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: WicSessionId,
    incomplete_device_warning: bool,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 84, 10);
    clear_popup(frame, app, popup);
    let detail = app.wic_session(id).map_or_else(
        || format!("Wic operation {} is unavailable.", id.0),
        |session| match &session.operation {
            WicOperation::Create(request) => format!(
                "Cancel Wic creation {}?\nImage: {}\nOutput: {}",
                id.0,
                request.image,
                request.output_directory.display()
            ),
            WicOperation::Write(request) => format!(
                "Cancel Wic device write {}?\nImage: {}\nDevice: {}",
                id.0,
                request.image.path.display(),
                request.device.path.display()
            ),
        },
    );
    let warning = if incomplete_device_warning {
        "\nWARNING: stopping a device write can leave the target incomplete and unusable."
    } else {
        ""
    };
    let title = if incomplete_device_warning {
        "Confirm Wic device-write cancellation"
    } else {
        "Confirm Wic cancellation"
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}{warning}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(dialog_block(
            app,
            title,
            if incomplete_device_warning {
                DialogTone::Destructive
            } else {
                DialogTone::Confirmation
            },
        ))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn platform_workspace(
    frame: &mut Frame,
    app: &App,
    workbench: &PlatformWorkbench,
    area: Rect,
    fallback_title: &str,
) {
    let title = workbench
        .inventory()
        .map_or(fallback_title, |inventory| inventory.component.label());
    let block = pane_block(app, title, app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let selection = match workbench.view {
        yoctui_model::PlatformView::Configuration => workbench.config_selection,
        yoctui_model::PlatformView::DeviceTrees => workbench.device_tree_selection,
    };
    let tabs = Line::from(vec![
        Span::styled(
            " 1 Configuration ",
            if workbench.view == yoctui_model::PlatformView::Configuration {
                selected_style(app, true)
            } else {
                Style::default()
            },
        ),
        Span::raw(" │ "),
        Span::styled(
            " 2 Device trees ",
            if workbench.view == yoctui_model::PlatformView::DeviceTrees {
                selected_style(app, true)
            } else {
                Style::default()
            },
        ),
        Span::raw("  Tab switches"),
    ]);
    let mut lines = vec![tabs];
    match &workbench.inventory {
        PlatformInventoryState::NotLoaded => {
            lines.push(Line::from("Not inspected. Press r to scan."))
        }
        PlatformInventoryState::Loading => lines.push(Line::from(
            "Inspecting provider, configuration, and device trees…",
        )),
        PlatformInventoryState::Failed(message) => {
            lines.push(Line::from(format!("Inspection failed: {message}")))
        }
        PlatformInventoryState::Available(inventory) => {
            lines.push(Line::from(format!(
                "Target {} | provider {} | menuconfig {} | dtc {}",
                inventory.target,
                inventory
                    .provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                if inventory
                    .tasks
                    .iter()
                    .any(|task| task == "menuconfig" || task == "do_menuconfig")
                {
                    "available"
                } else {
                    "unavailable"
                },
                inventory
                    .dtc
                    .as_ref()
                    .map_or("unavailable", |_| "available"),
            )));
            lines.push(Line::from("Kind         Size       File"));
            let files = workbench.visible_files().collect::<Vec<_>>();
            if files.is_empty() {
                lines.push(Line::from(match workbench.view {
                    yoctui_model::PlatformView::Configuration => {
                        "No .config file was found in the reported source/build roots."
                    }
                    yoctui_model::PlatformView::DeviceTrees => {
                        "No DTS, DTSI, DTB, or DTBO artifact was found."
                    }
                }));
            }
            let capacity = usize::from(inner.height.saturating_sub(4)).max(1);
            let viewport =
                yoctui_model::centered_viewport_range(Some(selection), files.len(), capacity);
            for (offset, file) in files[viewport.clone()].iter().enumerate() {
                let index = viewport.start + offset;
                lines.push(Line::styled(
                    format!(
                        "{:<12} {:>9}  {}",
                        file.kind.label(),
                        file.size_bytes,
                        file.path.display()
                    ),
                    selected_style(app, index == selection),
                ));
            }
            if let Some(limitation) = inventory.limitations.first() {
                lines.push(Line::styled(
                    format!("Limited: {limitation}"),
                    ThemePalette::for_app(app)
                        .role(ThemePalette::for_app(app).warning, Modifier::empty()),
                ));
            }
        }
    }
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(crate) fn platform_inspector_text(workbench: &PlatformWorkbench, title: &str) -> String {
    match &workbench.inventory {
        PlatformInventoryState::NotLoaded => format!("{title} has not been inspected."),
        PlatformInventoryState::Loading => format!("{title} inspection is loading."),
        PlatformInventoryState::Failed(message) => format!("{title} inspection failed: {message}"),
        PlatformInventoryState::Available(inventory) => {
            let selected = workbench.selected_file();
            format!(
                "Target: {}\nProvider: {}\nView: {}\nRoots: {}\nFiles: {}\nmenuconfig: {}\ndtc: {}\n\nSelected: {}\nKind: {}\nSize: {} bytes\n\nText sources open in the in-app explorer/editor. DTB and DTBO files are binary and can be decompiled to a new .yoctui.dts file.",
                inventory.target,
                inventory
                    .provider
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                workbench.view.label(),
                inventory.roots.len(),
                inventory.files.len(),
                if inventory
                    .tasks
                    .iter()
                    .any(|task| task == "menuconfig" || task == "do_menuconfig")
                {
                    "available"
                } else {
                    "unavailable"
                },
                inventory
                    .dtc
                    .as_ref()
                    .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
                selected.map_or_else(|| "none".into(), |file| file.path.display().to_string()),
                selected.map_or("unavailable", |file| file.kind.label()),
                selected.map_or(0, |file| file.size_bytes),
            )
        }
    }
}
