pub(crate) fn next_qemu_session_id(app: &mut App) -> QemuSessionId {
    app.qemu_session_generation = app.qemu_session_generation.wrapping_add(1);
    if app.qemu_session_generation == 0 {
        app.qemu_session_generation = 1;
    }
    QemuSessionId(app.qemu_session_generation)
}

pub(crate) fn qemu_background_job_id(id: QemuSessionId) -> BackgroundJobId {
    BackgroundJobId(QEMU_BACKGROUND_JOB_NAMESPACE | id.0)
}

pub(crate) fn qemu_job_id(app: &App, id: QemuSessionId) -> Option<BackgroundJobId> {
    app.qemu_session(id)
        .map(|session| session.background_job_id)
}

pub(crate) fn mutate_qemu_session(
    app: &mut App,
    id: QemuSessionId,
    mutation: impl FnOnce(&mut QemuSession),
) -> Option<BackgroundJobId> {
    let session = app
        .qemu_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

pub(crate) fn note_stale_qemu_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

pub(crate) const MAX_WIC_SESSIONS: usize = 32;
pub(crate) const WIC_BACKGROUND_JOB_NAMESPACE: u64 = 2 << 62;

pub(crate) fn next_wic_session_id(app: &mut App) -> WicSessionId {
    app.wic_session_generation = app.wic_session_generation.wrapping_add(1);
    if app.wic_session_generation == 0 {
        app.wic_session_generation = 1;
    }
    WicSessionId(app.wic_session_generation)
}

pub(crate) fn wic_background_job_id(id: WicSessionId) -> BackgroundJobId {
    BackgroundJobId(WIC_BACKGROUND_JOB_NAMESPACE | id.0)
}

pub(crate) fn wic_job_id(app: &App, id: WicSessionId) -> Option<BackgroundJobId> {
    app.wic_session(id).map(|session| session.background_job_id)
}

pub(crate) fn mutate_wic_session(
    app: &mut App,
    id: WicSessionId,
    mutation: impl FnOnce(&mut WicSession),
) -> Option<BackgroundJobId> {
    let session = app
        .wic_sessions
        .iter_mut()
        .find(|session| session.id == id)?;
    let job_id = session.background_job_id;
    mutation(session);
    Some(job_id)
}

pub(crate) fn note_stale_wic_event(app: &mut App) {
    app.background_jobs.ignored_transitions += 1;
}

pub(crate) fn is_uncompressed_wic_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".wic") || name.ends_with(".direct"))
}

pub(crate) fn reconcile_wic_output_selection(app: &mut App) {
    let rows = app.wic_output_rows();
    let selected_is_present = app
        .wic_output_selection
        .as_ref()
        .is_some_and(|selected| rows.iter().any(|row| &row.identity == selected));
    if !selected_is_present {
        app.wic_output_selection = rows.first().map(|row| row.identity.clone());
    }
}

pub(crate) fn reconcile_wic_device_selection(app: &mut App) {
    let rows = app.wic_device_rows();
    let selected_is_present = app
        .wic_device_selection
        .as_ref()
        .is_some_and(|selected| rows.iter().any(|row| &row.identity == selected));
    if !selected_is_present {
        app.wic_device_selection = rows.first().map(|row| row.identity.clone());
    }
}

pub(crate) fn current_wic_write_preview(
    app: &App,
    request: &WicDeviceInventoryRequest,
    device_identity: &WicDeviceIdentity,
    phrase: &str,
) -> Result<WicWritePreview, String> {
    let (active_request, devices) = match &app.wic_devices {
        WicDeviceInventoryState::Available {
            request, devices, ..
        }
        | WicDeviceInventoryState::Partial {
            request, devices, ..
        } => (request, devices),
        _ => return Err("The Wic device inventory is unavailable.".into()),
    };
    if active_request != request {
        return Err("The Wic device inventory request is stale.".into());
    }
    let current_image = app.selected_wic_write_image()?;
    if current_image != request.image {
        return Err("The selected Wic image identity changed.".into());
    }
    let device = devices
        .iter()
        .find(|device| &device.identity == device_identity)
        .ok_or_else(|| "The selected Wic device identity is stale.".to_owned())?;
    WicWritePreview::new(&app.wic_capability, request.image.clone(), device, phrase)
        .map_err(str::to_owned)
}

pub(crate) fn queue_wic_session(app: &mut App, operation: WicOperation) -> Option<Effect> {
    if app.active_wic_session().is_some() {
        app.notification = Some("A managed Wic operation is already active.".into());
        return None;
    }
    while app.wic_sessions.len() >= MAX_WIC_SESSIONS {
        let Some(index) = app.wic_sessions.iter().position(|session| {
            app.background_jobs
                .get(session.background_job_id)
                .is_none_or(|job| job.status.is_terminal())
        }) else {
            app.notification = Some("The Wic operation history is full.".into());
            return None;
        };
        app.wic_sessions.remove(index);
    }
    let id = next_wic_session_id(app);
    let background_job_id = wic_background_job_id(id);
    let (title, target, path) = match &operation {
        WicOperation::Create(request) => (
            format!("wic create {}", request.image),
            Some(request.image.clone()),
            Some(request.output_directory.clone()),
        ),
        WicOperation::Write(request) => (
            format!("wic write {}", request.device.path.display()),
            None,
            Some(request.device.path.clone()),
        ),
    };
    app.background_jobs.queue(BackgroundJobSpec {
        id: background_job_id,
        kind: BackgroundJobKind::Wic,
        title,
        context: BackgroundJobContext {
            workspace: Some(Screen::Images),
            target,
            path,
            ..BackgroundJobContext::default()
        },
        cancellation_supported: true,
        queued_at: SystemTime::now(),
    });
    if app.background_jobs.get(background_job_id).is_none() {
        app.notification = Some("The Wic operation could not be queued.".into());
        return None;
    }
    app.wic_sessions.push_back(WicSession {
        id,
        background_job_id,
        operation: operation.clone(),
        exit_code: None,
        error_detail: None,
    });
    Some(Effect::StartWicSession { id, operation })
}
