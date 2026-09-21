#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityReportRequest {
    pub generation: u64,
    pub paths: Vec<PathBuf>,
}

impl SecurityReportRequest {
    pub fn new(generation: u64, mut paths: Vec<PathBuf>) -> Result<Self, &'static str> {
        if generation == 0 || paths.is_empty() || paths.len() > MAX_SECURITY_PATHS {
            return Err("security report request is invalid");
        }
        paths.sort();
        paths.dedup();
        if paths.iter().any(|path| !absolute_normal_path(path)) {
            return Err("security report request path is invalid");
        }
        Ok(Self { generation, paths })
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SecurityInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: SecurityReportRequest,
    },
    AvailableEmpty {
        request: SecurityReportRequest,
    },
    Available {
        request: SecurityReportRequest,
        reports: Vec<SecurityReport>,
    },
    Partial {
        request: SecurityReportRequest,
        reports: Vec<SecurityReport>,
        limitations: Vec<String>,
    },
    Failed {
        request: SecurityReportRequest,
        message: String,
    },
    Cancelled {
        request: SecurityReportRequest,
    },
    TimedOut {
        request: SecurityReportRequest,
    },
    Lost {
        request: SecurityReportRequest,
        message: String,
    },
}

impl SecurityInventoryState {
    pub fn request(&self) -> Option<&SecurityReportRequest> {
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

    pub fn reports(&self) -> Option<&[SecurityReport]> {
        match self {
            Self::Available { reports, .. } | Self::Partial { reports, .. } => Some(reports),
            Self::AvailableEmpty { .. } => Some(&[]),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecuritySessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityOperation {
    CveCheck(BuildRequest),
    SbomBuild(BuildRequest),
    PackageMap {
        executable: PathBuf,
        arguments: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityOperationPreview {
    pub id: SecuritySessionId,
    pub scope: SecurityScope,
    pub operation: SecurityOperation,
    pub indexed_arguments: Vec<String>,
    pub report_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecuritySessionStatus {
    Starting,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
}

impl SecuritySessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::TimedOut | Self::Lost
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityOutputLine {
    pub stream: SecurityOutputStream,
    pub line: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecuritySession {
    pub preview: SecurityOperationPreview,
    pub status: SecuritySessionStatus,
    pub background_job_id: Option<BackgroundJobId>,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub message: Option<String>,
    pub result_paths: Vec<PathBuf>,
    pub output: Vec<SecurityOutputLine>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CveStatusFilter {
    #[default]
    All,
    Vulnerable,
    Patched,
    Ignored,
    NotAffected,
    Unknown,
}

impl CveStatusFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Vulnerable,
            Self::Vulnerable => Self::Patched,
            Self::Patched => Self::Ignored,
            Self::Ignored => Self::NotAffected,
            Self::NotAffected => Self::Unknown,
            Self::Unknown => Self::All,
        }
    }

    fn matches(self, status: CveStatus) -> bool {
        matches!(self, Self::All)
            || matches!(
                (self, status),
                (Self::Vulnerable, CveStatus::Vulnerable)
                    | (Self::Patched, CveStatus::Patched)
                    | (Self::Ignored, CveStatus::Ignored)
                    | (Self::NotAffected, CveStatus::NotAffected)
                    | (Self::Unknown, CveStatus::Unknown)
            )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityDialog {
    Operation(SecurityOperationPreview),
    Import {
        editor: PopupEditor,
        validation_error: Option<String>,
    },
    Cancellation(SecuritySessionId),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SecurityState {
    pub view: SecurityView,
    pub scope: Option<SecurityScope>,
    pub capability: SecurityCapability,
    pub inventory: SecurityInventoryState,
    pub report_selection: Option<SecurityReportIdentity>,
    pub finding_selection: Option<CveFindingIdentity>,
    pub component_selection: Option<String>,
    pub drilled: bool,
    pub query: String,
    pub searching: bool,
    pub cve_filter: CveStatusFilter,
    pub sessions: Vec<SecuritySession>,
    pub session_generation: u64,
    pub report_generation: u64,
}

impl SecurityState {
    pub fn active_session(&self) -> Option<&SecuritySession> {
        self.sessions
            .iter()
            .rev()
            .find(|session| !session.status.is_terminal())
    }

    pub fn selected_report(&self) -> Option<&SecurityReport> {
        let identity = self.report_selection.as_ref()?;
        self.inventory
            .reports()?
            .iter()
            .find(|report| report.identity() == identity)
    }

    pub fn visible_reports(&self) -> Vec<&SecurityReport> {
        let query = self.query.to_ascii_lowercase();
        self.inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .filter(|report| report.is_cve() == (self.view == SecurityView::Cves))
            .filter(|report| {
                query.is_empty()
                    || report
                        .identity()
                        .path
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&query)
            })
            .collect()
    }

    pub fn visible_findings(&self) -> Vec<&CveFinding> {
        let query = self.query.to_ascii_lowercase();
        let reports = self.inventory.reports().unwrap_or_default();
        reports
            .iter()
            .filter_map(|report| match report {
                SecurityReport::Cve(report) => Some(report),
                SecurityReport::Spdx(_)
                | SecurityReport::CycloneDx(_)
                | SecurityReport::PackageManifest(_) => None,
            })
            .flat_map(|report| report.findings.iter())
            .filter(|finding| self.cve_filter.matches(finding.status))
            .filter(|finding| {
                query.is_empty()
                    || [
                        Some(finding.identity.cve.as_str()),
                        Some(finding.identity.recipe.as_str()),
                        finding.identity.package.as_deref(),
                        finding.product.as_deref(),
                        finding.version.as_deref(),
                        finding.summary.as_deref(),
                    ]
                    .into_iter()
                    .flatten()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect()
    }

    pub fn visible_components(&self) -> Vec<&SpdxComponent> {
        let query = self.query.to_ascii_lowercase();
        match self.selected_report() {
            Some(report) => match report {
                SecurityReport::Spdx(document) => &document.components,
                SecurityReport::CycloneDx(document) => &document.components,
                SecurityReport::PackageManifest(document) => &document.components,
                SecurityReport::Cve(_) => return Vec::new(),
            }
            .iter()
            .filter(|component| {
                query.is_empty()
                    || [
                        component.identity.as_str(),
                        component.name.as_str(),
                        component.version.as_deref().unwrap_or_default(),
                        component.supplier.as_deref().unwrap_or_default(),
                        component.license.as_deref().unwrap_or_default(),
                    ]
                    .into_iter()
                    .any(|value| value.to_ascii_lowercase().contains(&query))
            })
            .collect(),
            _ => Vec::new(),
        }
    }
}
