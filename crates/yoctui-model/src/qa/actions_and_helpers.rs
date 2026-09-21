#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaAction {
    CycleView,
    InspectCapability,
    CapabilityLoaded(QaCapabilitySnapshot),
    CapabilityPartial {
        snapshot: QaCapabilitySnapshot,
        limitations: Vec<String>,
    },
    CapabilityFailed(String),
    CycleScope,
    SelectCheck(isize),
    BeginSelectedCheck,
    ConfirmOperation(QaOperationPreview),
    AttachBackgroundJob {
        session: QaSessionId,
        background_job: BackgroundJobId,
    },
    SessionRunning(QaSessionId),
    SessionOutput {
        session: QaSessionId,
        stream: QaOutputStream,
        line: String,
        truncated: bool,
    },
    CompleteSession {
        session: QaSessionId,
        result_paths: Vec<PathBuf>,
        finished_at: SystemTime,
    },
    FailSession {
        session: QaSessionId,
        message: String,
        finished_at: SystemTime,
    },
    TimeoutSession {
        session: QaSessionId,
        forced: bool,
        finished_at: SystemTime,
    },
    LoseSession {
        session: QaSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginCancellation,
    ConfirmCancellation(QaSessionId),
    RejectCancellation {
        session: QaSessionId,
        message: String,
    },
    CancelSession {
        session: QaSessionId,
        finished_at: SystemTime,
    },
    BeginImport,
    UpdateImport(String),
    ConfirmImport(String),
    CancelDialog,
    RefreshReports,
    ReportsLoaded {
        request: QaReportRequest,
        reports: Vec<QaReport>,
        limitations: Vec<String>,
    },
    ReportsFailed {
        request: QaReportRequest,
        kind: QaReportFailureKind,
        message: String,
    },
    ReportsCancelled(QaReportRequest),
    ReportsTimedOut(QaReportRequest),
    ReportsLost {
        request: QaReportRequest,
        message: String,
    },
    SelectReport(isize),
    SelectFinding(isize),
    Drill,
    LeaveDrill,
    BeginSearch,
    AppendQuery(char),
    BackspaceQuery,
    ClearQuery,
    FinishSearch,
    CycleStatusFilter,
    OpenSelectedReport,
    OpenProvider,
    OpenSelectedSource,
    InspectLayerCapability,
    LayerCapabilityLoaded(QaLayerCapabilitySnapshot),
    LayerCapabilityPartial {
        snapshot: QaLayerCapabilitySnapshot,
        limitations: Vec<String>,
    },
    LayerCapabilityFailed(String),
    SelectLayer(isize),
    BeginSelectedLayerCheck,
    ConfirmLayerOperation(QaLayerOperationPreview),
    LayerSessionRunning(QaLayerSessionId),
    LayerSessionOutput {
        session: QaLayerSessionId,
        stream: QaOutputStream,
        line: String,
        truncated: bool,
    },
    CompleteLayerSession {
        session: QaLayerSessionId,
        exit_code: i32,
        result_paths: Vec<PathBuf>,
        finished_at: SystemTime,
    },
    FailLayerSession {
        session: QaLayerSessionId,
        exit_code: Option<i32>,
        message: String,
        finished_at: SystemTime,
    },
    TimeoutLayerSession {
        session: QaLayerSessionId,
        forced: bool,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    LoseLayerSession {
        session: QaLayerSessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginLayerCancellation,
    ConfirmLayerCancellation(QaLayerSessionId),
    RejectLayerCancellation {
        session: QaLayerSessionId,
        message: String,
    },
    CancelLayerSession {
        session: QaLayerSessionId,
        forced: bool,
        exit_code: Option<i32>,
        finished_at: SystemTime,
    },
    OpenSelectedLayerRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaEffect {
    InspectCapability {
        scope: Option<QaScope>,
    },
    StartBuild {
        session: QaSessionId,
        request: BuildRequest,
    },
    CancelBuild {
        session: QaSessionId,
        background_job: BackgroundJobId,
    },
    ImportReports(QaReportRequest),
    OpenReport(QaReportIdentity),
    OpenProvider(RecipeIdentity),
    OpenSource(QaSourceLocation),
    InspectLayerCapability,
    StartLayerCheck {
        session: QaLayerSessionId,
        layer: QaLayerIdentity,
        executable: QaExecutableIdentity,
        arguments: Vec<String>,
    },
    CancelLayerCheck(QaLayerSessionId),
    OpenLayerRoot(QaLayerIdentity),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaDialogUpdate {
    None,
    Open(Box<QaDialog>),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaTransition {
    pub effect: Option<QaEffect>,
    pub dialog: QaDialogUpdate,
    pub notification: Option<String>,
}

impl QaTransition {
    fn none() -> Self {
        Self {
            effect: None,
            dialog: QaDialogUpdate::None,
            notification: None,
        }
    }

    fn effect(effect: QaEffect) -> Self {
        Self {
            effect: Some(effect),
            ..Self::none()
        }
    }

    fn notify(message: impl Into<String>) -> Self {
        Self {
            notification: Some(message.into()),
            ..Self::none()
        }
    }
}

fn next_id(value: &mut u64) -> u64 {
    *value = value.wrapping_add(1).max(1);
    *value
}

fn indexed_build_arguments(request: &BuildRequest) -> Vec<String> {
    let mut arguments = vec!["0: bitbake".into()];
    for target in &request.targets {
        arguments.push(format!("{}: {target}", arguments.len()));
    }
    if let Some(task) = &request.task {
        arguments.push(format!("{}: -c", arguments.len()));
        arguments.push(format!("{}: {task}", arguments.len()));
    }
    arguments
}

fn indexed_native_arguments(
    executable: &QaExecutableIdentity,
    arguments: &[String],
) -> Vec<String> {
    std::iter::once(format!("0: {}", executable.path.display()))
        .chain(
            arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| format!("{}: {argument}", index + 1)),
        )
        .collect()
}

fn clamp_selection(state: &mut QaState) {
    let checks = state
        .visible_checks()
        .into_iter()
        .map(|check| check.id.clone())
        .collect::<Vec<_>>();
    state.check_selection = state
        .check_selection
        .take()
        .filter(|identity| checks.contains(identity))
        .or_else(|| checks.first().cloned());
    let layers = state
        .visible_layers()
        .into_iter()
        .map(|layer| layer.identity.clone())
        .collect::<Vec<_>>();
    state.layer_selection = state
        .layer_selection
        .take()
        .filter(|identity| layers.contains(identity))
        .or_else(|| layers.first().cloned());
    let findings = state
        .visible_findings()
        .into_iter()
        .map(|finding| finding.identity.clone())
        .collect::<Vec<_>>();
    state.finding_selection = state
        .finding_selection
        .take()
        .filter(|identity| findings.contains(identity))
        .or_else(|| findings.first().cloned());
    let reports = state
        .inventory
        .reports()
        .unwrap_or_default()
        .iter()
        .map(|report| report.identity.clone())
        .collect::<Vec<_>>();
    state.report_selection = state
        .report_selection
        .take()
        .filter(|identity| reports.contains(identity))
        .or_else(|| reports.first().cloned());
}

fn begin_report_request(
    state: &mut QaState,
    paths: Vec<PathBuf>,
) -> Result<QaEffect, &'static str> {
    let request = QaReportRequest::new(next_id(&mut state.report_generation), paths)?;
    state.inventory = QaReportInventoryState::Loading {
        request: request.clone(),
    };
    Ok(QaEffect::ImportReports(request))
}

fn exact_request_matches(state: &QaState, request: &QaReportRequest) -> bool {
    state.inventory.request() == Some(request)
}

fn exact_capability_check<'a>(
    state: &'a QaState,
    preview: &QaOperationPreview,
) -> Option<&'a QaCheckCapability> {
    state.capability.snapshot()?.checks.iter().find(|check| {
        check.id == preview.check
            && check.family == preview.family
            && check.scope == preview.scope
            && check.task.as_ref() == preview.request.task.as_ref()
            && check.report_roots == preview.report_roots
            && matches!(check.availability, QaCheckAvailability::Available)
    })
}

fn session_mut(state: &mut QaState, id: QaSessionId) -> Option<&mut QaSession> {
    state.sessions.iter_mut().find(|session| session.id == id)
}

fn layer_session_mut(state: &mut QaState, id: QaLayerSessionId) -> Option<&mut QaLayerSession> {
    state
        .layer_sessions
        .iter_mut()
        .find(|session| session.id == id)
}

fn exact_layer_capability<'a>(
    state: &'a QaState,
    preview: &QaLayerOperationPreview,
) -> Option<&'a QaConfiguredLayerCapability> {
    state
        .layer_capability
        .snapshot()?
        .layers
        .iter()
        .find(|capability| {
            if capability.identity != preview.layer
                || capability.check != preview.check
                || capability.limitations != preview.limitations
            {
                return false;
            }
            matches!(
                &capability.run,
                QaLayerRunCapability::Available {
                    executable,
                    arguments,
                    report_roots,
                } if executable == &preview.executable
                    && arguments == &preview.arguments
                    && report_roots == &preview.report_roots
            )
        })
}

fn select_index<T: Clone + PartialEq>(
    items: &[T],
    selected: Option<&T>,
    delta: isize,
) -> Option<T> {
    let current = selected
        .and_then(|selected| items.iter().position(|item| item == selected))
        .unwrap_or(0);
    let next = if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current
            .saturating_add(delta as usize)
            .min(items.len().saturating_sub(1))
    };
    items.get(next).cloned()
}
