fn next_id(generation: &mut u64) -> u64 {
    *generation = generation.wrapping_add(1).max(1);
    *generation
}

fn exact_preview(state: &MaintenanceState, preview: &MaintenanceOperationPreview) -> bool {
    state.pending.as_ref() == Some(preview)
        && state.capability.request() == Some(preview.capability_request)
        && state
            .capability
            .snapshot()
            .is_some_and(|snapshot| snapshot.supports(preview.operation.tool()))
}

fn begin_session(
    state: &mut MaintenanceState,
    preview: MaintenanceOperationPreview,
) -> MaintenanceEffect {
    let id = MaintenanceSessionId(preview.id);
    if state.sessions.len() == MAX_MAINTENANCE_SESSIONS {
        state.sessions.pop_front();
    }
    state.sessions.push_back(MaintenanceSession {
        id,
        preview: preview.clone(),
        status: MaintenanceSessionStatus::Queued,
        started_at: None,
        finished_at: None,
        output: VecDeque::new(),
        dropped_lines: 0,
        exit_code: None,
        message: None,
    });
    state.pending = None;
    MaintenanceEffect::StartOperation {
        id,
        preview: Box::new(preview),
    }
}

fn session_mut(
    state: &mut MaintenanceState,
    id: MaintenanceSessionId,
) -> Option<&mut MaintenanceSession> {
    state.sessions.iter_mut().find(|session| session.id == id)
}

fn terminal_session(
    state: &mut MaintenanceState,
    id: MaintenanceSessionId,
    status: MaintenanceSessionStatus,
    exit_code: Option<i32>,
    message: Option<String>,
    finished_at: SystemTime,
) -> bool {
    let Some(session) = session_mut(state, id) else {
        return false;
    };
    if session.status.is_terminal() {
        return false;
    }
    session.status = status;
    session.exit_code = exit_code;
    session.message = message.filter(|message| bounded_text(message));
    session.finished_at = Some(finished_at);
    true
}
