#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaReportRequest {
    pub generation: u64,
    pub paths: Vec<PathBuf>,
}

impl QaReportRequest {
    pub fn new(generation: u64, paths: Vec<PathBuf>) -> Result<Self, &'static str> {
        if generation == 0
            || paths.is_empty()
            || paths.len() > MAX_QA_REPORT_PATHS
            || paths.iter().any(|path| !absolute_normal_path(path))
        {
            return Err("QA report request is invalid");
        }
        let paths = normalize_paths(paths);
        if paths.is_empty() {
            return Err("QA report request path is invalid");
        }
        Ok(Self { generation, paths })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaReportFailureKind {
    Missing,
    PermissionDenied,
    Stale,
    Malformed,
    Failed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QaReportInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: QaReportRequest,
    },
    AvailableEmpty {
        request: QaReportRequest,
    },
    Available {
        request: QaReportRequest,
        reports: Vec<QaReport>,
    },
    Partial {
        request: QaReportRequest,
        reports: Vec<QaReport>,
        limitations: Vec<String>,
    },
    Failed {
        request: QaReportRequest,
        kind: QaReportFailureKind,
        message: String,
    },
    Cancelled {
        request: QaReportRequest,
    },
    TimedOut {
        request: QaReportRequest,
    },
    Lost {
        request: QaReportRequest,
        message: String,
    },
}

impl QaReportInventoryState {
    pub fn request(&self) -> Option<&QaReportRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. }
            | Self::Cancelled { request }
            | Self::TimedOut { request }
            | Self::Lost { request, .. } => Some(request),
        }
    }

    pub fn reports(&self) -> Option<&[QaReport]> {
        match self {
            Self::Available { reports, .. } | Self::Partial { reports, .. } => Some(reports),
            Self::AvailableEmpty { .. } => Some(&[]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaDialog {
    Operation(QaOperationPreview),
    LayerOperation(QaLayerOperationPreview),
    Import {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    Cancellation {
        session: QaSessionId,
        background_job: BackgroundJobId,
    },
    LayerCancellation(QaLayerSessionId),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QaState {
    pub view: QaView,
    pub scope: Option<QaScope>,
    pub capability: QaCapability,
    pub check_selection: Option<QaCheckId>,
    pub inventory: QaReportInventoryState,
    pub report_selection: Option<QaReportIdentity>,
    pub finding_selection: Option<QaFindingIdentity>,
    pub drilled: bool,
    pub query: String,
    pub searching: bool,
    pub status_filter: QaStatusFilter,
    pub sessions: VecDeque<QaSession>,
    pub operation_generation: u64,
    pub session_generation: u64,
    pub report_generation: u64,
    pub pending_operation: Option<QaOperationPreview>,
    pub layer_capability: QaLayerCapability,
    pub layer_selection: Option<QaLayerIdentity>,
    pub layer_sessions: VecDeque<QaLayerSession>,
    pub layer_operation_generation: u64,
    pub layer_session_generation: u64,
    pub pending_layer_operation: Option<QaLayerOperationPreview>,
}

impl QaState {
    pub fn active_session(&self) -> Option<&QaSession> {
        self.sessions
            .iter()
            .rev()
            .find(|session| !session.status.is_terminal())
    }

    pub fn active_layer_session(&self) -> Option<&QaLayerSession> {
        self.layer_sessions
            .iter()
            .rev()
            .find(|session| !session.status.is_terminal())
    }

    pub fn visible_layers(&self) -> Vec<&QaConfiguredLayerCapability> {
        let query = self.query.to_ascii_lowercase();
        self.layer_capability
            .snapshot()
            .map(|snapshot| {
                snapshot
                    .layers
                    .iter()
                    .filter(|layer| {
                        matches!(self.status_filter, QaStatusFilter::All)
                            || self
                                .latest_status_for_layer(&layer.identity)
                                .is_some_and(|status| self.status_filter.matches(status))
                    })
                    .filter(|layer| {
                        query.is_empty()
                            || [
                                layer.identity.name.as_str(),
                                layer.identity.root.to_str().unwrap_or_default(),
                            ]
                            .into_iter()
                            .chain(layer.compatible_series.iter().map(String::as_str))
                            .any(|value| value.to_ascii_lowercase().contains(&query))
                            || self
                                .findings_for_layer(&layer.identity)
                                .into_iter()
                                .any(|finding| finding_matches_query(finding, &query))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn selected_layer(&self) -> Option<&QaConfiguredLayerCapability> {
        let identity = self.layer_selection.as_ref()?;
        self.layer_capability
            .snapshot()?
            .layers
            .iter()
            .find(|layer| &layer.identity == identity)
    }

    pub fn checks_for_scope(&self) -> Vec<&QaCheckCapability> {
        let Some(scope) = self.scope.as_ref() else {
            return Vec::new();
        };
        self.capability
            .snapshot()
            .map(|snapshot| {
                snapshot
                    .checks
                    .iter()
                    .filter(|check| &check.scope == scope)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn visible_checks(&self) -> Vec<&QaCheckCapability> {
        let query = self.query.to_ascii_lowercase();
        self.checks_for_scope()
            .into_iter()
            .filter(|check| {
                let status_matches = self
                    .latest_status_for_check(&check.id)
                    .is_some_and(|status| self.status_filter.matches(status));
                matches!(self.status_filter, QaStatusFilter::All) || status_matches
            })
            .filter(|check| {
                query.is_empty()
                    || [
                        check.label.as_str(),
                        check.id.0.as_str(),
                        check.scope.recipe.name.as_str(),
                        check.scope.recipe.file.to_str().unwrap_or_default(),
                        check.task.as_deref().unwrap_or_default(),
                    ]
                    .into_iter()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
                    || self
                        .findings_for_check(&check.id)
                        .into_iter()
                        .any(|finding| finding_matches_query(finding, &query))
            })
            .collect()
    }

    pub fn selected_check(&self) -> Option<&QaCheckCapability> {
        let id = self.check_selection.as_ref()?;
        self.checks_for_scope()
            .into_iter()
            .find(|check| &check.id == id)
    }

    pub fn findings_for_check(&self, check: &QaCheckId) -> Vec<&QaFinding> {
        self.inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .flat_map(|report| report.findings.iter())
            .filter(|finding| &finding.identity.check == check)
            .collect()
    }

    pub fn visible_findings(&self) -> Vec<&QaFinding> {
        let query = self.query.to_ascii_lowercase();
        let findings = match self.view {
            QaView::RecipeKernel => self
                .check_selection
                .as_ref()
                .map(|check| self.findings_for_check(check))
                .unwrap_or_default(),
            QaView::LayerQa => self
                .layer_selection
                .as_ref()
                .map(|layer| self.findings_for_layer(layer))
                .unwrap_or_default(),
        };
        findings
            .into_iter()
            .filter(|finding| self.status_filter.matches(finding.status))
            .filter(|finding| finding_matches_query(finding, &query))
            .collect()
    }

    pub fn selected_finding(&self) -> Option<&QaFinding> {
        let identity = self.finding_selection.as_ref()?;
        self.visible_findings()
            .into_iter()
            .find(|finding| &finding.identity == identity)
    }

    pub fn selected_report(&self) -> Option<&QaReport> {
        let identity = self.report_selection.as_ref()?;
        self.inventory
            .reports()?
            .iter()
            .find(|report| &report.identity == identity)
    }

    fn latest_status_for_check(&self, check: &QaCheckId) -> Option<QaFindingStatus> {
        self.findings_for_check(check)
            .into_iter()
            .map(|finding| finding.status)
            .max_by_key(|status| match status {
                QaFindingStatus::Failed => 5,
                QaFindingStatus::Warning => 4,
                QaFindingStatus::Unknown => 3,
                QaFindingStatus::Skipped => 2,
                QaFindingStatus::Passed => 1,
            })
            .or_else(|| {
                self.sessions
                    .iter()
                    .rev()
                    .find(|session| &session.operation.check == check)
                    .map(|session| match session.status {
                        QaSessionStatus::Succeeded => QaFindingStatus::Passed,
                        QaSessionStatus::Failed => QaFindingStatus::Failed,
                        QaSessionStatus::Cancelled | QaSessionStatus::TimedOut => {
                            QaFindingStatus::Skipped
                        }
                        QaSessionStatus::Starting
                        | QaSessionStatus::Running
                        | QaSessionStatus::Cancelling
                        | QaSessionStatus::Lost => QaFindingStatus::Unknown,
                    })
            })
    }

    pub fn findings_for_layer(&self, layer: &QaLayerIdentity) -> Vec<&QaFinding> {
        self.inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .flat_map(|report| report.findings.iter())
            .filter(|finding| {
                matches!(&finding.scope, QaFindingScope::Layer(candidate) if candidate == layer)
            })
            .collect()
    }

    pub fn layer_finding_counts(&self, layer: &QaLayerIdentity) -> QaFindingCounts {
        let mut counts = QaFindingCounts::default();
        for finding in self.findings_for_layer(layer) {
            counts.add(finding.status);
        }
        counts
    }

    fn latest_status_for_layer(&self, layer: &QaLayerIdentity) -> Option<QaFindingStatus> {
        self.findings_for_layer(layer)
            .into_iter()
            .map(|finding| finding.status)
            .max_by_key(|status| match status {
                QaFindingStatus::Failed => 5,
                QaFindingStatus::Warning => 4,
                QaFindingStatus::Unknown => 3,
                QaFindingStatus::Skipped => 2,
                QaFindingStatus::Passed => 1,
            })
            .or_else(|| {
                self.layer_sessions
                    .iter()
                    .rev()
                    .find(|session| &session.operation.layer == layer)
                    .map(|session| match session.status {
                        QaSessionStatus::Succeeded => QaFindingStatus::Passed,
                        QaSessionStatus::Failed => QaFindingStatus::Failed,
                        QaSessionStatus::Cancelled | QaSessionStatus::TimedOut => {
                            QaFindingStatus::Skipped
                        }
                        QaSessionStatus::Starting
                        | QaSessionStatus::Running
                        | QaSessionStatus::Cancelling
                        | QaSessionStatus::Lost => QaFindingStatus::Unknown,
                    })
            })
    }
}

fn finding_matches_query(finding: &QaFinding, query: &str) -> bool {
    query.is_empty()
        || [
            finding.identity.check.0.as_str(),
            finding.message.as_str(),
            finding.scope.name(),
            finding.scope.path().to_str().unwrap_or_default(),
            finding.task.as_deref().unwrap_or_default(),
            finding.test_name.as_deref().unwrap_or_default(),
            finding.severity.as_deref().unwrap_or_default(),
            finding.rule.as_deref().unwrap_or_default(),
            finding.suggestion.as_deref().unwrap_or_default(),
            finding
                .source
                .as_ref()
                .and_then(|source| source.path.to_str())
                .unwrap_or_default(),
        ]
        .into_iter()
        .any(|value| value.to_ascii_lowercase().contains(query))
}
