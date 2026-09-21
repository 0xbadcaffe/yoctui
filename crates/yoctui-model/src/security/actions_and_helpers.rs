#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityAction {
    InspectCapability,
    CapabilityLoaded(SecurityCapabilitySnapshot),
    CapabilityFailed(String),
    CycleView,
    CycleScope,
    SetScope(SecurityScope),
    BeginCveCheck,
    BeginSbomGeneration,
    BeginPackageMap,
    ConfirmOperation(SecurityOperationPreview),
    AttachBackgroundJob {
        id: SecuritySessionId,
        background_job_id: BackgroundJobId,
    },
    SessionRunning(SecuritySessionId),
    SessionOutput {
        id: SecuritySessionId,
        stream: SecurityOutputStream,
        line: String,
        truncated: bool,
    },
    CompleteSession {
        id: SecuritySessionId,
        result_paths: Vec<PathBuf>,
        finished_at: SystemTime,
    },
    FailSession {
        id: SecuritySessionId,
        message: String,
        finished_at: SystemTime,
    },
    TimeoutSession {
        id: SecuritySessionId,
        finished_at: SystemTime,
    },
    LoseSession {
        id: SecuritySessionId,
        message: String,
        finished_at: SystemTime,
    },
    BeginCancellation,
    ConfirmCancellation(SecuritySessionId),
    RejectCancellation {
        id: SecuritySessionId,
        message: String,
    },
    CancelSession {
        id: SecuritySessionId,
        finished_at: SystemTime,
    },
    BeginImport,
    UpdateImport(String),
    ConfirmImport(String),
    CancelDialog,
    RefreshReports,
    ReportsLoaded {
        request: SecurityReportRequest,
        reports: Vec<SecurityReport>,
        limitations: Vec<String>,
    },
    ReportsFailed {
        request: SecurityReportRequest,
        message: String,
    },
    ReportsCancelled(SecurityReportRequest),
    ReportsTimedOut(SecurityReportRequest),
    ReportsLost {
        request: SecurityReportRequest,
        message: String,
    },
    SelectReport(isize),
    SelectFinding(isize),
    SelectComponent(isize),
    Drill,
    LeaveDrill,
    BeginSearch,
    AppendQuery(char),
    BackspaceQuery,
    ClearQuery,
    FinishSearch,
    CycleCveFilter,
    OpenSelectedReport,
    OpenSelectedRecipe,
    OpenSelectedAdvisory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityEffect {
    InspectCapability,
    StartBuild {
        id: SecuritySessionId,
        request: BuildRequest,
    },
    StartPackageMap {
        id: SecuritySessionId,
        executable: PathBuf,
        arguments: Vec<String>,
    },
    CancelSession(SecuritySessionId),
    ImportReports(SecurityReportRequest),
    OpenPath(PathBuf),
    OpenUrl(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityDialogUpdate {
    None,
    Open(SecurityDialog),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityTransition {
    pub effect: Option<SecurityEffect>,
    pub dialog: SecurityDialogUpdate,
    pub notification: Option<String>,
}

impl SecurityTransition {
    fn none() -> Self {
        Self {
            effect: None,
            dialog: SecurityDialogUpdate::None,
            notification: None,
        }
    }

    fn effect(effect: SecurityEffect) -> Self {
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
    let mut values = vec!["0: bitbake".into()];
    for (index, target) in request.targets.iter().enumerate() {
        values.push(format!("{}: {target}", index + 1));
    }
    if let Some(task) = &request.task {
        values.push(format!("{}: -c", values.len()));
        values.push(format!("{}: {task}", values.len()));
    }
    values
}

fn operation_preview(
    state: &mut SecurityState,
    operation: SecurityOperation,
    report_roots: Vec<PathBuf>,
) -> Result<SecurityOperationPreview, &'static str> {
    if state.active_session().is_some() {
        return Err("a Security operation is already active");
    }
    let scope = state
        .scope
        .clone()
        .ok_or("select an exact Security scope")?;
    let indexed_arguments = match &operation {
        SecurityOperation::CveCheck(request) | SecurityOperation::SbomBuild(request) => {
            request.validate().map_err(|_| "invalid BitBake request")?;
            indexed_build_arguments(request)
        }
        SecurityOperation::PackageMap {
            executable,
            arguments,
        } => {
            if !absolute_normal_path(executable)
                || arguments.len() > 64
                || arguments.iter().any(|value| !bounded_text(value))
            {
                return Err("invalid package mapping operation");
            }
            std::iter::once(format!("0: {}", executable.display()))
                .chain(
                    arguments
                        .iter()
                        .enumerate()
                        .map(|(index, value)| format!("{}: {value}", index + 1)),
                )
                .collect()
        }
    };
    Ok(SecurityOperationPreview {
        id: SecuritySessionId(next_id(&mut state.session_generation)),
        scope,
        operation,
        indexed_arguments,
        report_roots,
    })
}

fn clamp_selection(state: &mut SecurityState) {
    let visible = state
        .visible_reports()
        .into_iter()
        .map(|report| report.identity().clone())
        .collect::<Vec<_>>();
    state.report_selection = state
        .report_selection
        .take()
        .filter(|identity| visible.contains(identity))
        .or_else(|| visible.first().cloned());
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
}

fn begin_report_request(
    state: &mut SecurityState,
    paths: Vec<PathBuf>,
) -> Result<SecurityEffect, &'static str> {
    let request = SecurityReportRequest::new(next_id(&mut state.report_generation), paths)?;
    state.inventory = SecurityInventoryState::Loading {
        request: request.clone(),
    };
    Ok(SecurityEffect::ImportReports(request))
}

fn exact_request_matches(state: &SecurityState, request: &SecurityReportRequest) -> bool {
    state.inventory.request() == Some(request)
}

fn selected_provider(state: &SecurityState) -> Option<PathBuf> {
    match state.scope.as_ref()? {
        SecurityScope::Recipe(identity) => Some(identity.file.clone()),
        SecurityScope::Image { .. } => None,
    }
}
