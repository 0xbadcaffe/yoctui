//! Emulation inspector.
use super::*;

pub(crate) fn qemu_capability_text(app: &App) -> String {
    match &app.qemu_capability {
        QemuCapability::NotInspected => "not inspected".into(),
        QemuCapability::MissingTool => "missing runqemu executable".into(),
        QemuCapability::MissingCompatibleImage => "no compatible deployed image".into(),
        QemuCapability::Failed { message } => format!("inspection failed: {message}"),
        QemuCapability::Available {
            executable,
            compatible_images,
        } => format!(
            "available: {}\nCompatible images: {}",
            executable.display(),
            compatible_images.len()
        ),
    }
}

pub(crate) fn qemu_session_text(app: &App) -> String {
    let Some(session) = app.latest_qemu_session() else {
        return "Managed runqemu session\nNo session has been launched.".into();
    };
    let Some(job) = app.background_jobs.get(session.background_job_id) else {
        return format!(
            "Managed runqemu session {}\nLifecycle record unavailable.",
            session.id.0
        );
    };
    let status = match job.status {
        BackgroundJobStatus::Queued => "queued",
        BackgroundJobStatus::Starting => "starting",
        BackgroundJobStatus::Running => "running",
        BackgroundJobStatus::Cancelling => "cancelling",
        BackgroundJobStatus::Succeeded => "succeeded",
        BackgroundJobStatus::Failed => "failed",
        BackgroundJobStatus::Cancelled => "cancelled",
        BackgroundJobStatus::Lost => "lost",
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
                    yoctui_model::BackgroundJobOutputSource::Backend => "backend",
                    yoctui_model::BackgroundJobOutputSource::Stdout => "stdout",
                    yoctui_model::BackgroundJobOutputSource::Stderr => "stderr",
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
    let error = job.error.as_ref().map_or_else(
        || session.error_detail.as_deref().unwrap_or("none").into(),
        |error| {
            error.detail.as_ref().map_or_else(
                || error.summary.clone(),
                |detail| format!("{}: {detail}", error.summary),
            )
        },
    );
    format!(
        "Managed runqemu session {}\nStatus: {}\nMachine: {}\nImage: {}\nArtifact: {}\nNetworking: {:?}\nDisplay: {:?}\nSerial: {:?}\nMemory: {} MiB\nQueued: {}\nStarted: {}\nFinished: {}\nExit code: {}\nResult: {}\nError: {}\nRetained output: {} entries (showing latest {})\nDropped output: {} entries\n\nOutput:\n{}",
        session.id.0,
        status,
        session.request.machine,
        session.request.image.image,
        session.request.image.path.display(),
        session.request.networking,
        session.request.display,
        session.request.serial,
        session.request.memory_mib,
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
        error,
        job.output.len(),
        job.output.len().min(80),
        job.dropped_output_entries,
        output,
    )
}

pub(crate) fn wic_inspector_text(app: &App) -> String {
    let capability = match &app.wic_capability {
        WicCapability::NotInspected => "not inspected".into(),
        WicCapability::MissingTool => "missing wic executable".into(),
        WicCapability::MissingKickstarts { .. } => "no kickstarts available".into(),
        WicCapability::Failed { message } => format!("inspection failed: {message}"),
        WicCapability::Available {
            executable,
            kickstarts,
            image_targets,
        } => format!(
            "available: {}\nKickstarts: {}\nImages: {}",
            executable.display(),
            kickstarts.len(),
            image_targets.len()
        ),
    };
    let readiness = app
        .wic_create_unavailable_reason()
        .unwrap_or_else(|| "ready for selected image (W)".into());
    let write_readiness = app.wic_device_write_unavailable_reason().map_or_else(
        || {
            app.selected_wic_write_image().map_or_else(
                |message| message,
                |image| {
                    format!(
                        "ready for protected write of {} ({} bytes) (D)",
                        image.path.display(),
                        image.size_bytes
                    )
                },
            )
        },
        |message| format!("disabled: {message}"),
    );
    if app.latest_wic_session().is_none()
        && matches!(app.wic_outputs, WicOutputInventoryState::NotLoaded)
        && matches!(app.wic_devices, WicDeviceInventoryState::NotLoaded)
        && !matches!(app.wic_capability, WicCapability::Available { .. })
    {
        return format!(
            "Wic: {capability} | Create disabled | Device write: {write_readiness} | Outputs and protected devices not loaded"
        );
    }
    let selected_kickstart = {
        let selected_identity = match app.active_dialog() {
            Some(Dialog::WicCreate(dialog)) => Some(&dialog.draft.kickstart),
            Some(Dialog::WicCreateConfirmation(preview)) => Some(&preview.request.kickstart),
            _ => app
                .latest_wic_session()
                .and_then(|session| match &session.operation {
                    WicOperation::Create(request) => Some(&request.kickstart),
                    WicOperation::Write(_) => None,
                }),
        };
        match &app.wic_capability {
            WicCapability::Available { kickstarts, .. } => selected_identity
                .and_then(|identity| {
                    kickstarts
                        .iter()
                        .find(|kickstart| kickstart.identity == *identity)
                })
                .or_else(|| kickstarts.first()),
            _ => None,
        }
    };
    let session = app.latest_wic_session().map_or_else(
        || "Managed Wic operation\nNo operation has been started.".into(),
        |session| {
            let Some(job) = app.background_jobs.get(session.background_job_id) else {
                return format!("Managed Wic operation {}\nLifecycle unavailable.", session.id.0);
            };
            let request = match &session.operation {
                WicOperation::Create(request) => format!(
                    "create image={} kickstart={} output={}",
                    request.image,
                    request.kickstart.name,
                    request.output_directory.display()
                ),
                WicOperation::Write(request) => format!(
                    "write\nimage={} ({} bytes, modified {}s)\ndevice={} major:minor={} capacity={} bytes model={} serial={} transport={}",
                    request.image.path.display(),
                    request.image.size_bytes,
                    request.image.modified_unix_seconds,
                    request.device.path.display(),
                    request.device.major_minor,
                    request.device.size_bytes,
                    request.device.model.as_deref().unwrap_or("unavailable"),
                    request.device.serial.as_deref().unwrap_or("unavailable"),
                    request.device.transport.as_deref().unwrap_or("unavailable"),
                ),
            };
            let output = job
                .output
                .iter()
                .rev()
                .take(40)
                .map(|entry| {
                    let source = match entry.source {
                        yoctui_model::BackgroundJobOutputSource::Backend => "backend",
                        yoctui_model::BackgroundJobOutputSource::Stdout => "stdout",
                        yoctui_model::BackgroundJobOutputSource::Stderr => "stderr",
                    };
                    format!(
                        "[{source}] {}{}",
                        entry.message,
                        if entry.truncated { " [truncated]" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            let result = job
                .result
                .as_ref()
                .map_or_else(|| "none".into(), |result| result.summary.clone());
            let error = job.error.as_ref().map_or_else(
                || session.error_detail.as_deref().unwrap_or("none").into(),
                |error| {
                    error.detail.as_ref().map_or_else(
                        || error.summary.clone(),
                        |detail| format!("{}: {detail}", error.summary),
                    )
                },
            );
            format!(
                "Managed Wic operation {}\nStatus: {}\nRequest: {}\nQueued: {}\nStarted: {}\nFinished: {}\nExit: {}\nResult: {}\nError: {}\nRetained output: {} entries / {} bytes (showing latest {})\nDropped output: {} entries\nWarnings: {} | Errors: {}\nHost telemetry: CPU {} | Disk available {}\nOutput:\n{}",
                session.id.0,
                match job.status {
                    BackgroundJobStatus::Queued => "queued",
                    BackgroundJobStatus::Starting => "starting",
                    BackgroundJobStatus::Running => "running",
                    BackgroundJobStatus::Cancelling => "cancelling",
                    BackgroundJobStatus::Succeeded => "succeeded",
                    BackgroundJobStatus::Failed => "failed",
                    BackgroundJobStatus::Cancelled => "cancelled",
                    BackgroundJobStatus::Lost => "lost",
                },
                request,
                timestamp_text(job.queued_at),
                job.started_at
                    .map(timestamp_text)
                    .unwrap_or_else(|| "unavailable".into()),
                job.finished_at
                    .map(timestamp_text)
                    .unwrap_or_else(|| "unavailable".into()),
                session
                    .exit_code
                    .map_or_else(|| "unavailable".into(), |value| value.to_string()),
                result,
                error,
                job.output.len(),
                job.retained_output_bytes,
                job.output.len().min(40),
                job.dropped_output_entries,
                job.warnings,
                job.errors,
                app.host_telemetry
                    .cpu_utilization_percent
                    .map_or_else(|| "unavailable".into(), |value| format!("{value}%")),
                app.host_telemetry
                    .disk_available_bytes
                    .map_or_else(|| "unavailable".into(), format_bytes),
                if output.is_empty() { "none" } else { &output },
            )
        },
    );
    let outputs = match &app.wic_outputs {
        WicOutputInventoryState::NotLoaded => "not loaded".into(),
        WicOutputInventoryState::Loading { request } => format!(
            "loading generation {} beneath {}",
            request.generation,
            request.output_directory.display()
        ),
        WicOutputInventoryState::Failed { request, message } => format!(
            "failed generation {} beneath {}: {message}",
            request.generation,
            request.output_directory.display()
        ),
        WicOutputInventoryState::Available { request, outputs }
        | WicOutputInventoryState::Partial {
            request, outputs, ..
        } => {
            let rows = if outputs.is_empty() {
                "none generated".into()
            } else {
                outputs
                    .iter()
                    .map(|output| {
                        let selected = app.wic_output_selection.as_ref() == Some(&output.identity);
                        format!(
                            "{} {:?} {} ({} bytes, {}s)",
                            if selected { "▶" } else { " " },
                            output.kind,
                            output.identity.path.display(),
                            output.identity.size_bytes,
                            output.identity.modified_unix_seconds,
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!(
                "generation {} beneath {}\n{}",
                request.generation,
                request.output_directory.display(),
                rows
            )
        }
    };
    let limitations = match &app.wic_outputs {
        WicOutputInventoryState::Partial { limitations, .. } => limitations.join("\n"),
        _ => "none".into(),
    };
    let devices = match &app.wic_devices {
        WicDeviceInventoryState::NotLoaded => "not loaded".into(),
        WicDeviceInventoryState::Loading { request } => format!(
            "loading generation {} for {}",
            request.generation,
            request.image.path.display()
        ),
        WicDeviceInventoryState::Failed { request, message } => format!(
            "failed generation {} for {}: {message}",
            request.generation,
            request.image.path.display()
        ),
        WicDeviceInventoryState::Available { request, devices }
        | WicDeviceInventoryState::Partial {
            request, devices, ..
        } => {
            let rows = if devices.is_empty() {
                "no eligible removable whole devices".into()
            } else {
                devices
                    .iter()
                    .map(|device| {
                        let selected =
                            app.wic_device_selection.as_ref() == Some(&device.identity);
                        let mounts = if device.descendant_mounts.is_empty() {
                            "none".into()
                        } else {
                            device
                                .descendant_mounts
                                .iter()
                                .map(|mount| mount.display().to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        };
                        format!(
                            "{} {} major:minor={} capacity={} bytes model={} serial={} transport={} removable={} writable={} read-only={} mounts={} unavailable={}",
                            if selected { "▶" } else { " " },
                            device.identity.path.display(),
                            device.identity.major_minor,
                            device.identity.size_bytes,
                            device.identity.model.as_deref().unwrap_or("unavailable"),
                            device.identity.serial.as_deref().unwrap_or("unavailable"),
                            device.identity.transport.as_deref().unwrap_or("unavailable"),
                            device.removable,
                            device.writable,
                            device.read_only,
                            mounts,
                            device.unavailable_reason.as_deref().unwrap_or("none"),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            format!(
                "generation {} for {}\n{}",
                request.generation,
                request.image.path.display(),
                rows
            )
        }
    };
    let device_limitations = match &app.wic_devices {
        WicDeviceInventoryState::Partial { limitations, .. } => limitations.join("\n"),
        _ => "none".into(),
    };
    let kickstart = selected_kickstart.map_or_else(
        || "Selected kickstart\nunavailable".into(),
        |kickstart| {
            format!(
                "Selected kickstart\nName: {}\nPath: {}\nSource (bounded to {} bytes):\n{}\nPartitions:\n{}\nLimitations:\n{}",
                kickstart.identity.name,
                kickstart
                    .identity
                    .path
                    .as_ref()
                    .map_or_else(|| "canned name".into(), |path| path.display().to_string()),
                yoctui_model::MAX_WIC_SOURCE_BYTES,
                if kickstart.source.is_empty() {
                    "empty"
                } else {
                    &kickstart.source
                },
                wic_partition_summary(kickstart),
                wic_limitations(kickstart),
            )
        },
    );
    format!(
        "Wic capability\n{capability}\nCreate: {readiness}\nDevice write: {write_readiness}\n\n{session}\n\nGenerated outputs\n{outputs}\nLimitations: {limitations}\n\nProtected device inventory\n{devices}\nLimitations: {device_limitations}\n\n{kickstart}"
    )
}
