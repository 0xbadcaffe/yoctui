use crate::{BackgroundJobId, BuildRequest, RecipeIdentity};
use std::{
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

pub const MAX_SECURITY_REPORTS: usize = 256;
pub const MAX_SECURITY_FINDINGS: usize = 16_384;
pub const MAX_SECURITY_COMPONENTS: usize = 16_384;
pub const MAX_SECURITY_METADATA: usize = 128;
pub const MAX_SECURITY_LIMITATIONS: usize = 128;
pub const MAX_SECURITY_TEXT_BYTES: usize = 4_096;
pub const MAX_SECURITY_QUERY_BYTES: usize = 512;
pub const MAX_SECURITY_FINGERPRINT_BYTES: usize = 256;
pub const MAX_SECURITY_PATHS: usize = 256;
pub const MAX_SECURITY_SESSIONS: usize = 64;
pub const MAX_SECURITY_SESSION_OUTPUT: usize = 256;

fn bounded_text(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SECURITY_TEXT_BYTES
        && !value.chars().any(char::is_control)
}

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().len() <= 4_096
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

fn bounded_fingerprint(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SECURITY_FINGERPRINT_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn normalize_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|value| bounded_text(value));
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_SECURITY_LIMITATIONS);
    limitations
}

fn normalize_metadata(mut metadata: Vec<SecurityMetadata>) -> Vec<SecurityMetadata> {
    metadata.retain(SecurityMetadata::is_valid);
    metadata.sort();
    metadata.dedup();
    metadata.truncate(MAX_SECURITY_METADATA);
    metadata
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SecurityView {
    #[default]
    Cves,
    Sbom,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SecurityScope {
    Recipe(RecipeIdentity),
    Image {
        target: String,
        machine: String,
        distro: String,
    },
}

impl SecurityScope {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Recipe(identity) => {
                bounded_token(&identity.name) && absolute_normal_path(&identity.file)
            }
            Self::Image {
                target,
                machine,
                distro,
            } => bounded_token(target) && bounded_token(machine) && bounded_token(distro),
        }
    }

    pub fn target(&self) -> &str {
        match self {
            Self::Recipe(identity) => &identity.name,
            Self::Image { target, .. } => target,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityMapperCapability {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
}

impl SecurityMapperCapability {
    pub fn is_valid(&self) -> bool {
        absolute_normal_path(&self.executable)
            && self.arguments.len() <= 64
            && self.arguments.iter().all(|argument| bounded_text(argument))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityCapabilitySnapshot {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub scope: SecurityScope,
    pub available_scopes: Vec<SecurityScope>,
    pub cve_task: Option<String>,
    pub recipe_sbom_task: Option<String>,
    pub image_sbom_task: Option<String>,
    pub image_build_emits_sbom: bool,
    pub mapper: Option<SecurityMapperCapability>,
    pub cve_roots: Vec<PathBuf>,
    pub sbom_roots: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

impl SecurityCapabilitySnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        release: Option<String>,
        build_directory: PathBuf,
        scope: SecurityScope,
        mut available_scopes: Vec<SecurityScope>,
        cve_task: Option<String>,
        recipe_sbom_task: Option<String>,
        image_sbom_task: Option<String>,
        image_build_emits_sbom: bool,
        mapper: Option<SecurityMapperCapability>,
        mut cve_roots: Vec<PathBuf>,
        mut sbom_roots: Vec<PathBuf>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&build_directory)
            || !scope.is_valid()
            || release.as_deref().is_some_and(|value| !bounded_text(value))
            || cve_task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || recipe_sbom_task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || image_sbom_task
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
            || mapper.as_ref().is_some_and(|value| !value.is_valid())
        {
            return Err("security capability identity is invalid");
        }
        cve_roots.retain(|path| absolute_normal_path(path));
        cve_roots.sort();
        cve_roots.dedup();
        cve_roots.truncate(MAX_SECURITY_PATHS);
        sbom_roots.retain(|path| absolute_normal_path(path));
        sbom_roots.sort();
        sbom_roots.dedup();
        sbom_roots.truncate(MAX_SECURITY_PATHS);
        available_scopes.retain(SecurityScope::is_valid);
        if !available_scopes.contains(&scope) {
            available_scopes.insert(0, scope.clone());
        }
        available_scopes.dedup();
        available_scopes.truncate(MAX_SECURITY_PATHS);
        Ok(Self {
            release,
            build_directory,
            scope,
            available_scopes,
            cve_task,
            recipe_sbom_task,
            image_sbom_task,
            image_build_emits_sbom,
            mapper,
            cve_roots,
            sbom_roots,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn cve_unavailable_reason(&self) -> Option<&'static str> {
        self.cve_task
            .is_none()
            .then_some("do_cve_check is not reported for the exact scope")
    }

    pub fn sbom_unavailable_reason(&self) -> Option<&'static str> {
        (self.recipe_sbom_task.is_none()
            && self.image_sbom_task.is_none()
            && !self.image_build_emits_sbom)
            .then_some("no authoritative SBOM task or image-build capability is reported")
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum SecurityCapability {
    #[default]
    NotInspected,
    Inspecting,
    Available(Box<SecurityCapabilitySnapshot>),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SecurityReportIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
    pub fingerprint: String,
}

impl SecurityReportIdentity {
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
        fingerprint: String,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) || byte_size == 0 || !bounded_fingerprint(&fingerprint) {
            return Err("security report identity is invalid");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
            fingerprint,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.path.clone(),
            self.byte_size,
            self.modified_at,
            self.fingerprint.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CveStatus {
    Vulnerable,
    Patched,
    Ignored,
    NotAffected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecurityMetadata {
    pub key: String,
    pub value: String,
}

impl SecurityMetadata {
    pub fn new(key: String, value: String) -> Result<Self, &'static str> {
        if !bounded_text(&key) || !bounded_text(&value) {
            return Err("security metadata is invalid");
        }
        Ok(Self { key, value })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.key.clone(), self.value.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CveFindingIdentity {
    pub cve: String,
    pub recipe: String,
    pub package: Option<String>,
}

impl CveFindingIdentity {
    pub fn new(cve: String, recipe: String, package: Option<String>) -> Result<Self, &'static str> {
        if !cve.starts_with("CVE-")
            || !bounded_token(&cve)
            || !bounded_token(&recipe)
            || package
                .as_deref()
                .is_some_and(|value| !bounded_token(value))
        {
            return Err("CVE finding identity is invalid");
        }
        Ok(Self {
            cve,
            recipe,
            package,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CveFinding {
    pub identity: CveFindingIdentity,
    pub status: CveStatus,
    pub product: Option<String>,
    pub version: Option<String>,
    pub severity: Option<String>,
    pub score: Option<String>,
    pub vector: Option<String>,
    pub advisory_url: Option<String>,
    pub summary: Option<String>,
    pub mapping: Vec<SecurityMetadata>,
}

impl CveFinding {
    pub fn is_valid(&self) -> bool {
        CveFindingIdentity::new(
            self.identity.cve.clone(),
            self.identity.recipe.clone(),
            self.identity.package.clone(),
        )
        .as_ref()
            == Ok(&self.identity)
            && [
                &self.product,
                &self.version,
                &self.severity,
                &self.score,
                &self.vector,
                &self.summary,
            ]
            .into_iter()
            .all(|value| value.as_deref().is_none_or(bounded_text))
            && self
                .advisory_url
                .as_deref()
                .is_none_or(|value| bounded_text(value) && value.starts_with("https://"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CveReport {
    pub identity: SecurityReportIdentity,
    pub scope: Option<SecurityScope>,
    pub findings: Vec<CveFinding>,
    pub metadata: Vec<SecurityMetadata>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpdxArtifactKind {
    Json,
    Archive,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SpdxComponent {
    pub identity: String,
    pub name: String,
    pub version: Option<String>,
    pub supplier: Option<String>,
    pub license: Option<String>,
}

impl SpdxComponent {
    pub fn is_valid(&self) -> bool {
        bounded_text(&self.identity)
            && bounded_text(&self.name)
            && [&self.version, &self.supplier, &self.license]
                .into_iter()
                .all(|value| value.as_deref().is_none_or(bounded_text))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpdxDocument {
    pub identity: SecurityReportIdentity,
    pub scope: Option<SecurityScope>,
    pub kind: SpdxArtifactKind,
    pub spdx_version: Option<String>,
    pub name: Option<String>,
    pub namespace: Option<String>,
    pub data_license: Option<String>,
    pub creators: Vec<String>,
    pub components: Vec<SpdxComponent>,
    pub file_count: Option<u64>,
    pub relationship_count: Option<u64>,
    pub checksums: Vec<SecurityMetadata>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityReport {
    Cve(CveReport),
    Spdx(SpdxDocument),
}

impl SecurityReport {
    pub fn identity(&self) -> &SecurityReportIdentity {
        match self {
            Self::Cve(report) => &report.identity,
            Self::Spdx(document) => &document.identity,
        }
    }

    pub fn is_cve(&self) -> bool {
        matches!(self, Self::Cve(_))
    }
}

pub fn normalize_security_reports(
    mut reports: Vec<SecurityReport>,
) -> (Vec<SecurityReport>, Vec<String>) {
    let mut limitations = Vec::new();
    reports.retain(|report| {
        let valid = report.identity().is_valid();
        if !valid {
            limitations.push("ignored a report with an invalid identity".into());
        }
        valid
    });
    for report in &mut reports {
        match report {
            SecurityReport::Cve(report) => {
                report.findings.retain(CveFinding::is_valid);
                report.findings.sort_by(|left, right| {
                    left.identity.cmp(&right.identity).then_with(|| {
                        left.status
                            .cmp(&right.status)
                            .then_with(|| left.product.cmp(&right.product))
                    })
                });
                report
                    .findings
                    .dedup_by(|left, right| left.identity == right.identity);
                if report.findings.len() > MAX_SECURITY_FINDINGS {
                    let dropped = report.findings.len() - MAX_SECURITY_FINDINGS;
                    report.findings.truncate(MAX_SECURITY_FINDINGS);
                    limitations.push(format!(
                        "ignored {dropped} CVE findings beyond the model bound"
                    ));
                }
                for finding in &mut report.findings {
                    finding.mapping = normalize_metadata(std::mem::take(&mut finding.mapping));
                }
                report.metadata = normalize_metadata(std::mem::take(&mut report.metadata));
                report.limitations = normalize_limitations(std::mem::take(&mut report.limitations));
            }
            SecurityReport::Spdx(document) => {
                document.components.retain(SpdxComponent::is_valid);
                document.components.sort();
                document.components.dedup();
                if document.components.len() > MAX_SECURITY_COMPONENTS {
                    let dropped = document.components.len() - MAX_SECURITY_COMPONENTS;
                    document.components.truncate(MAX_SECURITY_COMPONENTS);
                    limitations.push(format!(
                        "ignored {dropped} SPDX components beyond the model bound"
                    ));
                }
                document.checksums = normalize_metadata(std::mem::take(&mut document.checksums));
                document.limitations =
                    normalize_limitations(std::mem::take(&mut document.limitations));
            }
        }
    }
    reports.sort_by(|left, right| left.identity().cmp(right.identity()));
    reports.dedup_by(|left, right| left.identity() == right.identity());
    if reports.len() > MAX_SECURITY_REPORTS {
        let dropped = reports.len() - MAX_SECURITY_REPORTS;
        reports.truncate(MAX_SECURITY_REPORTS);
        limitations.push(format!(
            "ignored {dropped} security reports beyond the model bound"
        ));
    }
    (reports, normalize_limitations(limitations))
}

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
    Import { input: String },
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
                SecurityReport::Spdx(_) => None,
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
            Some(SecurityReport::Spdx(document)) => document
                .components
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

pub fn update_security(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        SecurityAction::InspectCapability => {
            state.capability = SecurityCapability::Inspecting;
            SecurityTransition::effect(SecurityEffect::InspectCapability)
        }
        SecurityAction::CapabilityLoaded(capability) => {
            state.scope = Some(capability.scope.clone());
            state.capability = SecurityCapability::Available(Box::new(capability));
            SecurityTransition::none()
        }
        SecurityAction::CapabilityFailed(message) => {
            state.capability = SecurityCapability::Failed(message);
            SecurityTransition::none()
        }
        SecurityAction::CycleView => {
            state.view = match state.view {
                SecurityView::Cves => SecurityView::Sbom,
                SecurityView::Sbom => SecurityView::Cves,
            };
            state.drilled = false;
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::CycleScope => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let current = capability
                .available_scopes
                .iter()
                .position(|scope| Some(scope) == state.scope.as_ref())
                .unwrap_or(0);
            let Some(scope) = capability
                .available_scopes
                .get((current + 1) % capability.available_scopes.len().max(1))
                .cloned()
            else {
                return SecurityTransition::notify("No alternate Security scope is available.");
            };
            state.scope = Some(scope);
            state.capability = SecurityCapability::NotInspected;
            SecurityTransition::effect(SecurityEffect::InspectCapability)
        }
        SecurityAction::SetScope(scope) if scope.is_valid() => {
            state.scope = Some(scope);
            state.capability = SecurityCapability::NotInspected;
            SecurityTransition::effect(SecurityEffect::InspectCapability)
        }
        SecurityAction::SetScope(_) => SecurityTransition::notify("Invalid Security scope."),
        SecurityAction::BeginCveCheck => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let Some(task) = capability.cve_task.clone() else {
                return SecurityTransition::notify(
                    capability
                        .cve_unavailable_reason()
                        .unwrap_or("CVE check is unavailable."),
                );
            };
            let request = BuildRequest {
                targets: vec![capability.scope.target().into()],
                task: Some(task),
                force: false,
            };
            match operation_preview(
                state,
                SecurityOperation::CveCheck(request),
                capability.cve_roots.clone(),
            ) {
                Ok(preview) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)),
                    ..SecurityTransition::none()
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::BeginSbomGeneration => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let task = match &capability.scope {
                SecurityScope::Recipe(_) => capability.recipe_sbom_task.clone(),
                SecurityScope::Image { .. } => capability.image_sbom_task.clone(),
            };
            if task.is_none() && !capability.image_build_emits_sbom {
                return SecurityTransition::notify(
                    capability
                        .sbom_unavailable_reason()
                        .unwrap_or("SBOM generation is unavailable."),
                );
            }
            let request = BuildRequest {
                targets: vec![capability.scope.target().into()],
                task,
                force: false,
            };
            match operation_preview(
                state,
                SecurityOperation::SbomBuild(request),
                capability.sbom_roots.clone(),
            ) {
                Ok(preview) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)),
                    ..SecurityTransition::none()
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::BeginPackageMap => {
            let SecurityCapability::Available(capability) = &state.capability else {
                return SecurityTransition::notify("Security capability is not available.");
            };
            let Some(mapper) = capability.mapper.clone() else {
                return SecurityTransition::notify("cve-check-map-pkgs is unavailable.");
            };
            match operation_preview(
                state,
                SecurityOperation::PackageMap {
                    executable: mapper.executable,
                    arguments: mapper.arguments,
                },
                capability.cve_roots.clone(),
            ) {
                Ok(preview) => SecurityTransition {
                    dialog: SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)),
                    ..SecurityTransition::none()
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::ConfirmOperation(preview)
            if state.active_session().is_none()
                && preview.id.0 != 0
                && Some(&preview.scope) == state.scope.as_ref() =>
        {
            let effect = match &preview.operation {
                SecurityOperation::CveCheck(request) | SecurityOperation::SbomBuild(request) => {
                    SecurityEffect::StartBuild {
                        id: preview.id,
                        request: request.clone(),
                    }
                }
                SecurityOperation::PackageMap {
                    executable,
                    arguments,
                } => SecurityEffect::StartPackageMap {
                    id: preview.id,
                    executable: executable.clone(),
                    arguments: arguments.clone(),
                },
            };
            state.sessions.push(SecuritySession {
                preview,
                status: SecuritySessionStatus::Starting,
                background_job_id: None,
                started_at: SystemTime::now(),
                finished_at: None,
                message: None,
                result_paths: Vec::new(),
                output: Vec::new(),
            });
            if state.sessions.len() > MAX_SECURITY_SESSIONS {
                state.sessions.remove(0);
            }
            SecurityTransition {
                effect: Some(effect),
                dialog: SecurityDialogUpdate::Close,
                notification: None,
            }
        }
        SecurityAction::ConfirmOperation(_) => {
            SecurityTransition::notify("The Security operation preview is stale.")
        }
        SecurityAction::AttachBackgroundJob {
            id,
            background_job_id,
        } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
            {
                session.background_job_id = Some(background_job_id);
            }
            SecurityTransition::none()
        }
        SecurityAction::SessionRunning(id) => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && session.status == SecuritySessionStatus::Starting
            {
                session.status = SecuritySessionStatus::Running;
            }
            SecurityTransition::none()
        }
        SecurityAction::SessionOutput {
            id,
            stream,
            line,
            truncated,
        } => {
            if line.len() <= MAX_SECURITY_TEXT_BYTES
                && !line.chars().any(char::is_control)
                && let Some(session) = state
                    .sessions
                    .iter_mut()
                    .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.output.push(SecurityOutputLine {
                    stream,
                    line,
                    truncated,
                });
                if session.output.len() > MAX_SECURITY_SESSION_OUTPUT {
                    session.output.remove(0);
                }
            }
            SecurityTransition::none()
        }
        SecurityAction::CompleteSession {
            id,
            mut result_paths,
            finished_at,
        } => {
            result_paths.retain(|path| absolute_normal_path(path));
            result_paths.sort();
            result_paths.dedup();
            result_paths.truncate(MAX_SECURITY_PATHS);
            let mut refresh_paths = Vec::new();
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Succeeded;
                session.finished_at = Some(finished_at);
                session.result_paths = result_paths;
                refresh_paths = if session.result_paths.is_empty() {
                    session.preview.report_roots.clone()
                } else {
                    session.result_paths.clone()
                };
            }
            match begin_report_request(state, refresh_paths) {
                Ok(effect) => SecurityTransition::effect(effect),
                Err(_) => SecurityTransition::notify(
                    "Security operation succeeded, but no exact report path was reported.",
                ),
            }
        }
        SecurityAction::FailSession {
            id,
            message,
            finished_at,
        } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Failed;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            SecurityTransition::none()
        }
        SecurityAction::LoseSession {
            id,
            message,
            finished_at,
        } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Lost;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            SecurityTransition::none()
        }
        SecurityAction::TimeoutSession { id, finished_at } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::TimedOut;
                session.finished_at = Some(finished_at);
            }
            SecurityTransition::none()
        }
        SecurityAction::BeginCancellation => {
            let Some(session) = state.active_session() else {
                return SecurityTransition::notify("No Security operation is active.");
            };
            SecurityTransition {
                dialog: SecurityDialogUpdate::Open(SecurityDialog::Cancellation(
                    session.preview.id,
                )),
                ..SecurityTransition::none()
            }
        }
        SecurityAction::ConfirmCancellation(id) => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && matches!(
                    session.status,
                    SecuritySessionStatus::Starting | SecuritySessionStatus::Running
                )
            {
                session.status = SecuritySessionStatus::Cancelling;
                return SecurityTransition {
                    effect: Some(SecurityEffect::CancelSession(id)),
                    dialog: SecurityDialogUpdate::Close,
                    notification: None,
                };
            }
            SecurityTransition::notify("The Security cancellation request is stale.")
        }
        SecurityAction::RejectCancellation { id, message } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && session.status == SecuritySessionStatus::Cancelling
            {
                session.status = SecuritySessionStatus::Running;
                session.message = Some(message);
            }
            SecurityTransition::none()
        }
        SecurityAction::CancelSession { id, finished_at } => {
            if let Some(session) = state
                .sessions
                .iter_mut()
                .find(|session| session.preview.id == id)
                && !session.status.is_terminal()
            {
                session.status = SecuritySessionStatus::Cancelled;
                session.finished_at = Some(finished_at);
            }
            SecurityTransition::none()
        }
        SecurityAction::BeginImport => SecurityTransition {
            dialog: SecurityDialogUpdate::Open(SecurityDialog::Import {
                input: String::new(),
            }),
            ..SecurityTransition::none()
        },
        SecurityAction::UpdateImport(input)
            if input.len() <= MAX_SECURITY_TEXT_BYTES
                && !input.chars().any(|character| character.is_control()) =>
        {
            SecurityTransition {
                dialog: SecurityDialogUpdate::Open(SecurityDialog::Import { input }),
                ..SecurityTransition::none()
            }
        }
        SecurityAction::UpdateImport(_) => SecurityTransition::none(),
        SecurityAction::ConfirmImport(input) => {
            match begin_report_request(state, vec![PathBuf::from(input)]) {
                Ok(effect) => SecurityTransition {
                    effect: Some(effect),
                    dialog: SecurityDialogUpdate::Close,
                    notification: None,
                },
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::CancelDialog => SecurityTransition {
            dialog: SecurityDialogUpdate::Close,
            ..SecurityTransition::none()
        },
        SecurityAction::RefreshReports => {
            let Some(request) = state.inventory.request().cloned() else {
                return SecurityTransition::notify("Import or discover Security reports first.");
            };
            match begin_report_request(state, request.paths) {
                Ok(effect) => SecurityTransition::effect(effect),
                Err(message) => SecurityTransition::notify(message),
            }
        }
        SecurityAction::ReportsLoaded {
            request,
            reports,
            limitations,
        } if exact_request_matches(state, &request) => {
            let (reports, mut model_limitations) = normalize_security_reports(reports);
            model_limitations.extend(limitations);
            let limitations = normalize_limitations(model_limitations);
            state.inventory = if reports.is_empty() && limitations.is_empty() {
                SecurityInventoryState::AvailableEmpty { request }
            } else if limitations.is_empty() {
                SecurityInventoryState::Available { request, reports }
            } else {
                SecurityInventoryState::Partial {
                    request,
                    reports,
                    limitations,
                }
            };
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::ReportsFailed { request, message }
            if exact_request_matches(state, &request) =>
        {
            state.inventory = SecurityInventoryState::Failed { request, message };
            SecurityTransition::none()
        }
        SecurityAction::ReportsCancelled(request) if exact_request_matches(state, &request) => {
            state.inventory = SecurityInventoryState::Cancelled { request };
            SecurityTransition::none()
        }
        SecurityAction::ReportsTimedOut(request) if exact_request_matches(state, &request) => {
            state.inventory = SecurityInventoryState::TimedOut { request };
            SecurityTransition::none()
        }
        SecurityAction::ReportsLost { request, message }
            if exact_request_matches(state, &request) =>
        {
            state.inventory = SecurityInventoryState::Lost { request, message };
            SecurityTransition::none()
        }
        SecurityAction::ReportsLoaded { .. }
        | SecurityAction::ReportsFailed { .. }
        | SecurityAction::ReportsCancelled(_)
        | SecurityAction::ReportsTimedOut(_)
        | SecurityAction::ReportsLost { .. } => SecurityTransition::none(),
        SecurityAction::SelectReport(delta) => {
            let identities = state
                .visible_reports()
                .into_iter()
                .map(|report| report.identity().clone())
                .collect::<Vec<_>>();
            let current = state
                .report_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len().saturating_sub(1))
            };
            state.report_selection = identities.get(next).cloned();
            state.drilled = false;
            SecurityTransition::none()
        }
        SecurityAction::SelectFinding(delta) => {
            let identities = state
                .visible_findings()
                .into_iter()
                .map(|finding| finding.identity.clone())
                .collect::<Vec<_>>();
            let current = state
                .finding_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len().saturating_sub(1))
            };
            state.finding_selection = identities.get(next).cloned();
            SecurityTransition::none()
        }
        SecurityAction::SelectComponent(delta) => {
            let identities = state
                .visible_components()
                .into_iter()
                .map(|component| component.identity.clone())
                .collect::<Vec<_>>();
            let current = state
                .component_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len().saturating_sub(1))
            };
            state.component_selection = identities.get(next).cloned();
            SecurityTransition::none()
        }
        SecurityAction::Drill => {
            if matches!(state.selected_report(), Some(SecurityReport::Spdx(_))) {
                state.drilled = true;
                state.component_selection = state
                    .visible_components()
                    .first()
                    .map(|component| component.identity.clone());
            }
            SecurityTransition::none()
        }
        SecurityAction::LeaveDrill => {
            state.drilled = false;
            SecurityTransition::none()
        }
        SecurityAction::BeginSearch => {
            state.searching = true;
            SecurityTransition::none()
        }
        SecurityAction::AppendQuery(character)
            if state.searching
                && !character.is_control()
                && state.query.len() + character.len_utf8() <= MAX_SECURITY_QUERY_BYTES =>
        {
            state.query.push(character);
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::BackspaceQuery if state.searching => {
            state.query.pop();
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::FinishSearch => {
            state.searching = false;
            SecurityTransition::none()
        }
        SecurityAction::AppendQuery(_) | SecurityAction::BackspaceQuery => {
            SecurityTransition::none()
        }
        SecurityAction::CycleCveFilter => {
            state.cve_filter = state.cve_filter.next();
            clamp_selection(state);
            SecurityTransition::none()
        }
        SecurityAction::OpenSelectedReport => state.selected_report().map_or_else(
            || SecurityTransition::notify("Select an exact Security report first."),
            |report| {
                SecurityTransition::effect(SecurityEffect::OpenPath(report.identity().path.clone()))
            },
        ),
        SecurityAction::OpenSelectedRecipe => selected_provider(state).map_or_else(
            || SecurityTransition::notify("No exact recipe provider is available."),
            |path| SecurityTransition::effect(SecurityEffect::OpenPath(path)),
        ),
        SecurityAction::OpenSelectedAdvisory => {
            let finding = state
                .visible_findings()
                .into_iter()
                .find(|finding| Some(&finding.identity) == state.finding_selection.as_ref());
            finding
                .and_then(|finding| finding.advisory_url.clone())
                .map_or_else(
                    || SecurityTransition::notify("No exact HTTPS advisory URL is available."),
                    |url| SecurityTransition::effect(SecurityEffect::OpenUrl(url)),
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, App, Dialog, Effect, FocusTarget, Screen, update};

    fn recipe() -> RecipeIdentity {
        RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox_1.0.bb".into(),
        }
    }

    fn capability() -> SecurityCapabilitySnapshot {
        SecurityCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            SecurityScope::Recipe(recipe()),
            vec![SecurityScope::Recipe(recipe())],
            Some("cve_check".into()),
            Some("create_recipe_sbom".into()),
            None,
            false,
            Some(SecurityMapperCapability {
                executable: "/tools/cve-check-map-pkgs".into(),
                arguments: vec!["/build/tmp/log/cve".into()],
            }),
            vec!["/build/tmp/log/cve".into()],
            vec!["/build/tmp/deploy/spdx".into()],
            vec![],
        )
        .unwrap()
    }

    fn identity(path: &str, fingerprint: &str) -> SecurityReportIdentity {
        SecurityReportIdentity::new(path.into(), 12, SystemTime::UNIX_EPOCH, fingerprint.into())
            .unwrap()
    }

    fn finding(status: CveStatus) -> CveFinding {
        CveFinding {
            identity: CveFindingIdentity::new(
                "CVE-2026-0001".into(),
                "busybox".into(),
                Some("busybox".into()),
            )
            .unwrap(),
            status,
            product: Some("busybox".into()),
            version: Some("1.0".into()),
            severity: Some("HIGH".into()),
            score: Some("8.1".into()),
            vector: None,
            advisory_url: Some("https://example.invalid/CVE-2026-0001".into()),
            summary: Some("bounds check".into()),
            mapping: vec![],
        }
    }

    #[test]
    fn security_workflow_previews_capability_supplied_current_and_legacy_tasks() {
        let mut state = SecurityState::default();
        let _ = update_security(&mut state, SecurityAction::CapabilityLoaded(capability()));
        let transition = update_security(&mut state, SecurityAction::BeginCveCheck);
        let SecurityDialogUpdate::Open(SecurityDialog::Operation(cve)) = transition.dialog else {
            panic!("expected CVE preview");
        };
        assert!(matches!(
            &cve.operation,
            SecurityOperation::CveCheck(BuildRequest { task: Some(task), .. })
                if task == "cve_check"
        ));
        assert_eq!(cve.indexed_arguments[0], "0: bitbake");

        let transition = update_security(&mut state, SecurityAction::BeginSbomGeneration);
        let SecurityDialogUpdate::Open(SecurityDialog::Operation(sbom)) = transition.dialog else {
            panic!("expected SBOM preview");
        };
        assert!(matches!(
            &sbom.operation,
            SecurityOperation::SbomBuild(BuildRequest { task: Some(task), .. })
                if task == "create_recipe_sbom"
        ));

        let mut legacy = capability();
        legacy.recipe_sbom_task = Some("create_spdx".into());
        let _ = update_security(&mut state, SecurityAction::CapabilityLoaded(legacy));
        let transition = update_security(&mut state, SecurityAction::BeginSbomGeneration);
        assert!(matches!(
            transition.dialog,
            SecurityDialogUpdate::Open(SecurityDialog::Operation(SecurityOperationPreview {
                operation: SecurityOperation::SbomBuild(BuildRequest {
                    task: Some(ref task),
                    ..
                }),
                ..
            })) if task == "create_spdx"
        ));
    }

    #[test]
    fn security_workflow_normalizes_reports_and_rejects_stale_generations() {
        let mut state = SecurityState::default();
        let request = SecurityReportRequest::new(1, vec!["/reports".into()]).unwrap();
        state.inventory = SecurityInventoryState::Loading {
            request: request.clone(),
        };
        let report = SecurityReport::Cve(CveReport {
            identity: identity("/reports/cve.json", "abc"),
            scope: Some(SecurityScope::Recipe(recipe())),
            findings: vec![
                finding(CveStatus::Vulnerable),
                finding(CveStatus::Vulnerable),
            ],
            metadata: vec![],
            limitations: vec![],
        });
        let stale = SecurityReportRequest::new(2, vec!["/reports".into()]).unwrap();
        let _ = update_security(
            &mut state,
            SecurityAction::ReportsLoaded {
                request: stale,
                reports: vec![report.clone()],
                limitations: vec![],
            },
        );
        assert!(matches!(
            state.inventory,
            SecurityInventoryState::Loading { .. }
        ));
        let _ = update_security(
            &mut state,
            SecurityAction::ReportsLoaded {
                request,
                reports: vec![report],
                limitations: vec![],
            },
        );
        assert_eq!(state.visible_findings().len(), 1);
        assert!(matches!(
            state.inventory,
            SecurityInventoryState::Available { .. }
        ));
        let _ = update_security(&mut state, SecurityAction::CycleCveFilter);
        assert_eq!(state.cve_filter, CveStatusFilter::Vulnerable);
        let _ = update_security(&mut state, SecurityAction::BeginSearch);
        for character in "CVE-2026".chars() {
            let _ = update_security(&mut state, SecurityAction::AppendQuery(character));
        }
        assert_eq!(state.visible_findings().len(), 1);
    }

    #[test]
    fn security_workflow_correlates_session_terminal_and_refresh_states() {
        let mut state = SecurityState::default();
        let _ = update_security(&mut state, SecurityAction::CapabilityLoaded(capability()));
        let transition = update_security(&mut state, SecurityAction::BeginCveCheck);
        let SecurityDialogUpdate::Open(SecurityDialog::Operation(preview)) = transition.dialog
        else {
            panic!("preview");
        };
        let id = preview.id;
        let transition = update_security(&mut state, SecurityAction::ConfirmOperation(preview));
        assert!(matches!(
            transition.effect,
            Some(SecurityEffect::StartBuild { id: started, .. }) if started == id
        ));
        let _ = update_security(&mut state, SecurityAction::SessionRunning(id));
        for index in 0..=MAX_SECURITY_SESSION_OUTPUT {
            let _ = update_security(
                &mut state,
                SecurityAction::SessionOutput {
                    id,
                    stream: SecurityOutputStream::Stdout,
                    line: format!("mapped package {index}"),
                    truncated: index == MAX_SECURITY_SESSION_OUTPUT,
                },
            );
        }
        assert_eq!(
            state.active_session().unwrap().output.len(),
            MAX_SECURITY_SESSION_OUTPUT
        );
        assert_eq!(
            state.active_session().unwrap().output[0].line,
            "mapped package 1"
        );
        let _ = update_security(&mut state, SecurityAction::BeginCancellation);
        let transition = update_security(&mut state, SecurityAction::ConfirmCancellation(id));
        assert_eq!(transition.effect, Some(SecurityEffect::CancelSession(id)));
        let _ = update_security(
            &mut state,
            SecurityAction::RejectCancellation {
                id,
                message: "busy".into(),
            },
        );
        assert_eq!(
            state.active_session().unwrap().status,
            SecuritySessionStatus::Running
        );
        let transition = update_security(
            &mut state,
            SecurityAction::CompleteSession {
                id,
                result_paths: vec!["/reports/cve.json".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(matches!(
            transition.effect,
            Some(SecurityEffect::ImportReports(_))
        ));
        assert_eq!(state.sessions[0].status, SecuritySessionStatus::Succeeded);
        let request = state.inventory.request().unwrap().clone();
        let _ = update_security(&mut state, SecurityAction::ReportsTimedOut(request));
        assert!(matches!(
            state.inventory,
            SecurityInventoryState::TimedOut { .. }
        ));
    }

    #[test]
    fn security_workflow_app_navigation_dialog_and_effects_are_typed() {
        let mut app = App::new(20, 4_000);
        assert_eq!(
            update(&mut app, Action::Open(Screen::Security)),
            Some(Effect::Security(SecurityEffect::InspectCapability))
        );
        assert_eq!(app.screen, Screen::Security);
        assert_eq!(app.focus, FocusTarget::Workspace);
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::CapabilityLoaded(capability())),
        );
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::BeginSbomGeneration),
        );
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::Security(SecurityDialog::Operation(
                SecurityOperationPreview {
                    operation: SecurityOperation::SbomBuild(BuildRequest {
                        task: Some(task),
                        ..
                    }),
                    ..
                }
            ))) if task == "create_recipe_sbom"
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);
        let preview = match app.active_dialog().cloned().unwrap() {
            Dialog::Security(SecurityDialog::Operation(preview)) => preview,
            _ => unreachable!(),
        };
        assert!(matches!(
            update(
                &mut app,
                Action::Security(SecurityAction::ConfirmOperation(preview))
            ),
            Some(Effect::Security(SecurityEffect::StartBuild { .. }))
        ));
        assert_eq!(app.focus, FocusTarget::Workspace);
    }
}
