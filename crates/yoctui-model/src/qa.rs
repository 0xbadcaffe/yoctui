use crate::{
    BackgroundJobId, BuildRequest, PopupEditor, RecipeIdentity, popup_toml_document,
    popup_toml_value,
};
use std::{
    collections::VecDeque,
    path::{Component, Path, PathBuf},
    time::SystemTime,
};

pub const MAX_QA_CHECKS: usize = 256;
pub const MAX_QA_SCOPES: usize = 256;
pub const MAX_QA_REPORTS: usize = 256;
pub const MAX_QA_FINDINGS: usize = 16_384;
pub const MAX_QA_METADATA: usize = 128;
pub const MAX_QA_LIMITATIONS: usize = 128;
pub const MAX_QA_REPORT_PATHS: usize = 256;
pub const MAX_QA_SESSIONS: usize = 64;
pub const MAX_QA_SESSION_OUTPUT: usize = 256;
pub const MAX_QA_TEXT_BYTES: usize = 4_096;
pub const MAX_QA_QUERY_BYTES: usize = 512;
pub const MAX_QA_FINGERPRINT_BYTES: usize = 256;
pub const MAX_QA_LAYER_ARGUMENTS: usize = 64;
pub const MAX_QA_COMPATIBLE_SERIES: usize = 64;

fn bounded_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_QA_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn bounded_token(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 256
        && !matches!(value, "." | "..")
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
        })
}

fn bounded_fingerprint(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_QA_FINGERPRINT_BYTES
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
}

fn absolute_normal_path(path: &Path) -> bool {
    path.is_absolute()
        && path != Path::new("/")
        && path.as_os_str().len() <= MAX_QA_TEXT_BYTES
        && path
            .components()
            .all(|component| !matches!(component, Component::ParentDir | Component::CurDir))
}

fn normalize_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    paths.retain(|path| absolute_normal_path(path));
    paths.sort();
    paths.dedup();
    paths.truncate(MAX_QA_REPORT_PATHS);
    paths
}

fn normalize_limitations(mut limitations: Vec<String>) -> Vec<String> {
    limitations.retain(|value| bounded_text(value));
    limitations.sort();
    limitations.dedup();
    limitations.truncate(MAX_QA_LIMITATIONS);
    limitations
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QaScope {
    pub recipe: RecipeIdentity,
}

impl QaScope {
    pub fn new(recipe: RecipeIdentity) -> Result<Self, &'static str> {
        if !bounded_token(&recipe.name) || !absolute_normal_path(&recipe.file) {
            return Err("QA recipe/provider scope is invalid");
        }
        Ok(Self { recipe })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.recipe.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QaView {
    #[default]
    RecipeKernel,
    LayerQa,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaLayerIdentity {
    pub name: String,
    pub root: PathBuf,
}

impl QaLayerIdentity {
    pub fn new(name: String, root: PathBuf) -> Result<Self, &'static str> {
        if !bounded_token(&name) || !absolute_normal_path(&root) {
            return Err("configured QA layer identity is invalid");
        }
        Ok(Self { name, root })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.name.clone(), self.root.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QaFindingScope {
    Recipe(QaScope),
    Layer(QaLayerIdentity),
}

impl QaFindingScope {
    pub fn is_valid(&self) -> bool {
        match self {
            Self::Recipe(scope) => scope.is_valid(),
            Self::Layer(layer) => layer.is_valid(),
        }
    }

    fn name(&self) -> &str {
        match self {
            Self::Recipe(scope) => &scope.recipe.name,
            Self::Layer(layer) => &layer.name,
        }
    }

    fn path(&self) -> &Path {
        match self {
            Self::Recipe(scope) => &scope.recipe.file,
            Self::Layer(layer) => &layer.root,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaExecutableIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
}

impl QaExecutableIdentity {
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path) || byte_size == 0 {
            return Err("QA executable identity is invalid");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.path.clone(), self.byte_size, self.modified_at).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaLayerRunCapability {
    Available {
        executable: QaExecutableIdentity,
        arguments: Vec<String>,
        report_roots: Vec<PathBuf>,
    },
    Disabled(String),
}

impl QaLayerRunCapability {
    pub fn disabled_reason(&self) -> Option<&str> {
        match self {
            Self::Available { .. } => None,
            Self::Disabled(reason) => Some(reason),
        }
    }

    fn is_valid_for(&self, layer: &QaLayerIdentity) -> bool {
        match self {
            Self::Available {
                executable,
                arguments,
                report_roots,
            } => {
                executable.is_valid()
                    && !arguments.is_empty()
                    && arguments.len() <= MAX_QA_LAYER_ARGUMENTS
                    && arguments.iter().all(|argument| bounded_text(argument))
                    && arguments
                        .iter()
                        .any(|argument| argument == &layer.root.to_string_lossy())
                    && report_roots.iter().all(|path| absolute_normal_path(path))
            }
            Self::Disabled(reason) => bounded_text(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaConfiguredLayerCapability {
    pub check: QaCheckId,
    pub identity: QaLayerIdentity,
    pub compatible_series: Vec<String>,
    pub run: QaLayerRunCapability,
    pub limitations: Vec<String>,
}

impl QaConfiguredLayerCapability {
    pub fn new(
        check: QaCheckId,
        identity: QaLayerIdentity,
        mut compatible_series: Vec<String>,
        run: QaLayerRunCapability,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !check.is_valid()
            || !identity.is_valid()
            || !run.is_valid_for(&identity)
            || compatible_series.iter().any(|value| !bounded_token(value))
        {
            return Err("configured layer QA capability is invalid");
        }
        compatible_series.sort();
        compatible_series.dedup();
        compatible_series.truncate(MAX_QA_COMPATIBLE_SERIES);
        Ok(Self {
            check,
            identity,
            compatible_series,
            run,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.check.clone(),
            self.identity.clone(),
            self.compatible_series.clone(),
            self.run.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerCapabilitySnapshot {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected_layer: QaLayerIdentity,
    pub layers: Vec<QaConfiguredLayerCapability>,
    pub limitations: Vec<String>,
}

impl QaLayerCapabilitySnapshot {
    pub fn new(
        release: Option<String>,
        build_directory: PathBuf,
        selected_layer: QaLayerIdentity,
        mut layers: Vec<QaConfiguredLayerCapability>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&build_directory)
            || !selected_layer.is_valid()
            || release.as_deref().is_some_and(|value| !bounded_text(value))
            || layers.iter().any(|layer| !layer.is_valid())
        {
            return Err("layer QA capability identity is invalid");
        }
        layers.sort_by(|left, right| left.identity.cmp(&right.identity));
        layers.dedup_by(|left, right| left.identity == right.identity);
        layers.truncate(MAX_QA_SCOPES);
        if !layers.iter().any(|layer| layer.identity == selected_layer) {
            return Err("selected layer is not in the configured QA layer inventory");
        }
        Ok(Self {
            release,
            build_directory,
            selected_layer,
            layers,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.release.clone(),
            self.build_directory.clone(),
            self.selected_layer.clone(),
            self.layers.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QaLayerCapability {
    #[default]
    NotInspected,
    Inspecting,
    Available(Box<QaLayerCapabilitySnapshot>),
    Partial {
        snapshot: Box<QaLayerCapabilitySnapshot>,
        limitations: Vec<String>,
    },
    Failed(String),
}

impl QaLayerCapability {
    pub fn snapshot(&self) -> Option<&QaLayerCapabilitySnapshot> {
        match self {
            Self::Available(snapshot) | Self::Partial { snapshot, .. } => Some(snapshot),
            Self::NotInspected | Self::Inspecting | Self::Failed(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QaCheckFamily {
    KernelConfiguration,
    UriFetch,
    Patch,
    License,
    RecipePackage,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaCheckId(pub String);

impl QaCheckId {
    pub fn new(value: String) -> Result<Self, &'static str> {
        if !bounded_token(&value) {
            return Err("QA check identity is invalid");
        }
        Ok(Self(value))
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.0.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QaCheckAvailability {
    Available,
    Disabled(String),
}

impl QaCheckAvailability {
    pub fn disabled_reason(&self) -> Option<&str> {
        match self {
            Self::Available => None,
            Self::Disabled(reason) => Some(reason),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaCheckCapability {
    pub id: QaCheckId,
    pub family: QaCheckFamily,
    pub label: String,
    pub scope: QaScope,
    pub task: Option<String>,
    pub report_roots: Vec<PathBuf>,
    pub availability: QaCheckAvailability,
    pub limitations: Vec<String>,
}

impl QaCheckCapability {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: QaCheckId,
        family: QaCheckFamily,
        label: String,
        scope: QaScope,
        task: Option<String>,
        report_roots: Vec<PathBuf>,
        availability: QaCheckAvailability,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !id.is_valid()
            || !bounded_text(&label)
            || !scope.is_valid()
            || task.as_deref().is_some_and(|value| !bounded_token(value))
            || report_roots.iter().any(|path| !absolute_normal_path(path))
            || matches!(
                &availability,
                QaCheckAvailability::Disabled(reason) if !bounded_text(reason)
            )
        {
            return Err("QA check capability is invalid");
        }
        let report_roots = normalize_paths(report_roots);
        match &availability {
            QaCheckAvailability::Available if task.is_none() => {
                return Err("available QA check has no capability-supplied task");
            }
            QaCheckAvailability::Disabled(_) if task.is_some() => {
                return Err("disabled QA check cannot expose an executable task");
            }
            _ => {}
        }
        Ok(Self {
            id,
            family,
            label,
            scope,
            task,
            report_roots,
            availability,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.id.clone(),
            self.family,
            self.label.clone(),
            self.scope.clone(),
            self.task.clone(),
            self.report_roots.clone(),
            self.availability.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaCapabilitySnapshot {
    pub release: Option<String>,
    pub build_directory: PathBuf,
    pub selected_scope: QaScope,
    pub scopes: Vec<QaScope>,
    pub checks: Vec<QaCheckCapability>,
    pub limitations: Vec<String>,
}

impl QaCapabilitySnapshot {
    pub fn new(
        release: Option<String>,
        build_directory: PathBuf,
        selected_scope: QaScope,
        mut scopes: Vec<QaScope>,
        mut checks: Vec<QaCheckCapability>,
        limitations: Vec<String>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&build_directory)
            || !selected_scope.is_valid()
            || release.as_deref().is_some_and(|value| !bounded_text(value))
            || scopes.iter().any(|scope| !scope.is_valid())
            || checks.iter().any(|check| !check.is_valid())
        {
            return Err("QA capability identity is invalid");
        }
        if !scopes.contains(&selected_scope) {
            scopes.insert(0, selected_scope.clone());
        }
        scopes.dedup();
        scopes.truncate(MAX_QA_SCOPES);
        if checks.iter().any(|check| !scopes.contains(&check.scope)) {
            return Err("QA check scope is not capability supplied");
        }
        checks.sort_by(|left, right| {
            left.scope
                .recipe
                .name
                .cmp(&right.scope.recipe.name)
                .then_with(|| left.scope.recipe.file.cmp(&right.scope.recipe.file))
                .then_with(|| left.id.cmp(&right.id))
        });
        checks.dedup_by(|left, right| left.scope == right.scope && left.id == right.id);
        checks.truncate(MAX_QA_CHECKS);
        Ok(Self {
            release,
            build_directory,
            selected_scope,
            scopes,
            checks,
            limitations: normalize_limitations(limitations),
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.release.clone(),
            self.build_directory.clone(),
            self.selected_scope.clone(),
            self.scopes.clone(),
            self.checks.clone(),
            self.limitations.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QaCapability {
    #[default]
    NotInspected,
    Inspecting,
    Available(Box<QaCapabilitySnapshot>),
    Partial {
        snapshot: Box<QaCapabilitySnapshot>,
        limitations: Vec<String>,
    },
    Failed(String),
}

impl QaCapability {
    pub fn snapshot(&self) -> Option<&QaCapabilitySnapshot> {
        match self {
            Self::Available(snapshot) | Self::Partial { snapshot, .. } => Some(snapshot),
            Self::NotInspected | Self::Inspecting | Self::Failed(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaOperationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaOperationPreview {
    pub id: QaOperationId,
    pub check: QaCheckId,
    pub family: QaCheckFamily,
    pub scope: QaScope,
    pub request: BuildRequest,
    pub indexed_arguments: Vec<String>,
    pub report_roots: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaSessionStatus {
    Starting,
    Running,
    Cancelling,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Lost,
}

impl QaSessionStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Succeeded | Self::Failed | Self::Cancelled | Self::TimedOut | Self::Lost
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QaOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaOutputLine {
    pub stream: QaOutputStream,
    pub line: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaSession {
    pub id: QaSessionId,
    pub operation: QaOperationPreview,
    pub status: QaSessionStatus,
    pub background_job_id: Option<BackgroundJobId>,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub message: Option<String>,
    pub result_paths: Vec<PathBuf>,
    pub output: VecDeque<QaOutputLine>,
    pub dropped_output: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaLayerOperationId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaLayerSessionId(pub u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerOperationPreview {
    pub id: QaLayerOperationId,
    pub check: QaCheckId,
    pub layer: QaLayerIdentity,
    pub executable: QaExecutableIdentity,
    pub arguments: Vec<String>,
    pub indexed_arguments: Vec<String>,
    pub report_roots: Vec<PathBuf>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaLayerSession {
    pub id: QaLayerSessionId,
    pub operation: QaLayerOperationPreview,
    pub status: QaSessionStatus,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub exit_code: Option<i32>,
    pub message: Option<String>,
    pub result_paths: Vec<PathBuf>,
    pub output: VecDeque<QaOutputLine>,
    pub dropped_output: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct QaFindingCounts {
    pub passed: usize,
    pub warnings: usize,
    pub failed: usize,
    pub skipped: usize,
    pub unknown: usize,
}

impl QaFindingCounts {
    fn add(&mut self, status: QaFindingStatus) {
        match status {
            QaFindingStatus::Passed => self.passed += 1,
            QaFindingStatus::Warning => self.warnings += 1,
            QaFindingStatus::Failed => self.failed += 1,
            QaFindingStatus::Skipped => self.skipped += 1,
            QaFindingStatus::Unknown => self.unknown += 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QaFindingStatus {
    Passed,
    Warning,
    Failed,
    Skipped,
    Unknown,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum QaStatusFilter {
    #[default]
    All,
    Failed,
    Warning,
    Passed,
    Skipped,
    Unknown,
}

impl QaStatusFilter {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Failed,
            Self::Failed => Self::Warning,
            Self::Warning => Self::Passed,
            Self::Passed => Self::Skipped,
            Self::Skipped => Self::Unknown,
            Self::Unknown => Self::All,
        }
    }

    fn matches(self, status: QaFindingStatus) -> bool {
        matches!(self, Self::All)
            || matches!(
                (self, status),
                (Self::Failed, QaFindingStatus::Failed)
                    | (Self::Warning, QaFindingStatus::Warning)
                    | (Self::Passed, QaFindingStatus::Passed)
                    | (Self::Skipped, QaFindingStatus::Skipped)
                    | (Self::Unknown, QaFindingStatus::Unknown)
            )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum QaReportFormat {
    Json,
    Xml,
    Text,
    BitBakeLog,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QaReportIdentity {
    pub path: PathBuf,
    pub byte_size: u64,
    pub modified_at: SystemTime,
    pub fingerprint: String,
    pub format: QaReportFormat,
    pub producer: Option<QaCheckId>,
    pub scope: Option<QaFindingScope>,
}

impl QaReportIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        path: PathBuf,
        byte_size: u64,
        modified_at: SystemTime,
        fingerprint: String,
        format: QaReportFormat,
        producer: Option<QaCheckId>,
        scope: Option<QaFindingScope>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path)
            || byte_size == 0
            || !bounded_fingerprint(&fingerprint)
            || producer.as_ref().is_some_and(|value| !value.is_valid())
            || scope.as_ref().is_some_and(|value| !value.is_valid())
        {
            return Err("QA report identity is invalid");
        }
        Ok(Self {
            path,
            byte_size,
            modified_at,
            fingerprint,
            format,
            producer,
            scope,
        })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(
            self.path.clone(),
            self.byte_size,
            self.modified_at,
            self.fingerprint.clone(),
            self.format,
            self.producer.clone(),
            self.scope.clone(),
        )
        .as_ref()
            == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct QaFindingIdentity {
    pub check: QaCheckId,
    pub fingerprint: String,
}

impl QaFindingIdentity {
    pub fn new(check: QaCheckId, fingerprint: String) -> Result<Self, &'static str> {
        if !check.is_valid() || !bounded_fingerprint(&fingerprint) {
            return Err("QA finding identity is invalid");
        }
        Ok(Self { check, fingerprint })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct QaSourceLocation {
    pub path: PathBuf,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

impl QaSourceLocation {
    pub fn new(
        path: PathBuf,
        line: Option<u32>,
        column: Option<u32>,
    ) -> Result<Self, &'static str> {
        if !absolute_normal_path(&path)
            || line == Some(0)
            || column == Some(0)
            || (column.is_some() && line.is_none())
        {
            return Err("QA source location is invalid");
        }
        Ok(Self { path, line, column })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct QaMetadata {
    pub key: String,
    pub value: String,
}

impl QaMetadata {
    pub fn new(key: String, value: String) -> Result<Self, &'static str> {
        if !bounded_text(&key) || !bounded_text(&value) {
            return Err("QA metadata is invalid");
        }
        Ok(Self { key, value })
    }

    pub fn is_valid(&self) -> bool {
        Self::new(self.key.clone(), self.value.clone()).as_ref() == Ok(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaFinding {
    pub identity: QaFindingIdentity,
    pub status: QaFindingStatus,
    pub severity: Option<String>,
    pub message: String,
    pub scope: QaFindingScope,
    pub task: Option<String>,
    pub test_name: Option<String>,
    pub source: Option<QaSourceLocation>,
    pub rule: Option<String>,
    pub suggestion: Option<String>,
    pub metadata: Vec<QaMetadata>,
}

impl QaFinding {
    pub fn is_valid(&self) -> bool {
        bounded_text(&self.message)
            && self.scope.is_valid()
            && [
                self.severity.as_deref(),
                self.rule.as_deref(),
                self.suggestion.as_deref(),
            ]
            .into_iter()
            .flatten()
            .all(bounded_text)
            && self.task.as_deref().is_none_or(bounded_token)
            && self.test_name.as_deref().is_none_or(bounded_text)
            && matches!(
                (&self.scope, &self.task, &self.test_name),
                (QaFindingScope::Recipe(_), _, None) | (QaFindingScope::Layer(_), None, Some(_))
            )
            && self.source.as_ref().is_none_or(|source| {
                QaSourceLocation::new(source.path.clone(), source.line, source.column).as_ref()
                    == Ok(source)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QaReport {
    pub identity: QaReportIdentity,
    pub findings: Vec<QaFinding>,
    pub metadata: Vec<QaMetadata>,
    pub limitations: Vec<String>,
}

pub fn normalize_qa_reports(
    mut reports: Vec<QaReport>,
    known_checks: &[QaCheckId],
    known_scopes: &[QaFindingScope],
) -> (Vec<QaReport>, Vec<String>) {
    let mut limitations = Vec::new();
    reports.retain(|report| {
        let valid = report.identity.is_valid()
            && report
                .identity
                .scope
                .as_ref()
                .is_none_or(|scope| known_scopes.contains(scope));
        if !valid {
            limitations.push("ignored a QA report with an invalid identity".into());
        }
        valid
    });
    for report in &mut reports {
        report.findings.retain(|finding| {
            let valid = finding.is_valid()
                && known_checks.contains(&finding.identity.check)
                && known_scopes.contains(&finding.scope);
            if !valid {
                limitations.push("ignored an invalid or unknown QA finding".into());
            }
            valid
        });
        for finding in &mut report.findings {
            finding.metadata.retain(QaMetadata::is_valid);
            finding.metadata.sort();
            finding.metadata.dedup();
            finding.metadata.truncate(MAX_QA_METADATA);
        }
        report.findings.sort_by(|left, right| {
            left.identity
                .cmp(&right.identity)
                .then_with(|| left.status.cmp(&right.status))
        });
        report
            .findings
            .dedup_by(|left, right| left.identity == right.identity);
        if report.findings.len() > MAX_QA_FINDINGS {
            let dropped = report.findings.len() - MAX_QA_FINDINGS;
            report.findings.truncate(MAX_QA_FINDINGS);
            limitations.push(format!(
                "ignored {dropped} QA findings beyond the model bound"
            ));
        }
        report.metadata.retain(QaMetadata::is_valid);
        report.metadata.sort();
        report.metadata.dedup();
        report.metadata.truncate(MAX_QA_METADATA);
        report.limitations = normalize_limitations(std::mem::take(&mut report.limitations));
    }
    reports.sort_by(|left, right| {
        left.identity
            .path
            .cmp(&right.identity.path)
            .then_with(|| left.identity.fingerprint.cmp(&right.identity.fingerprint))
    });
    reports.dedup_by(|left, right| left.identity == right.identity);
    if reports.len() > MAX_QA_REPORTS {
        let dropped = reports.len() - MAX_QA_REPORTS;
        reports.truncate(MAX_QA_REPORTS);
        limitations.push(format!(
            "ignored {dropped} QA reports beyond the model bound"
        ));
    }
    (reports, normalize_limitations(limitations))
}

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

pub fn update_qa(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        QaAction::CycleView => {
            state.view = match state.view {
                QaView::RecipeKernel => QaView::LayerQa,
                QaView::LayerQa => QaView::RecipeKernel,
            };
            state.drilled = false;
            clamp_selection(state);
            if state.view == QaView::LayerQa
                && matches!(state.layer_capability, QaLayerCapability::NotInspected)
            {
                state.layer_capability = QaLayerCapability::Inspecting;
                return QaTransition::effect(QaEffect::InspectLayerCapability);
            }
            QaTransition::none()
        }
        QaAction::InspectCapability => {
            state.capability = QaCapability::Inspecting;
            QaTransition::effect(QaEffect::InspectCapability {
                scope: state.scope.clone(),
            })
        }
        QaAction::CapabilityLoaded(snapshot) => {
            if !snapshot.is_valid() {
                state.capability =
                    QaCapability::Failed("QA capability response is invalid.".into());
                state.check_selection = None;
                return QaTransition::notify("QA capability response is invalid.");
            }
            state.scope = Some(snapshot.selected_scope.clone());
            state.capability = QaCapability::Available(Box::new(snapshot));
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::CapabilityPartial {
            snapshot,
            limitations,
        } => {
            if !snapshot.is_valid() {
                state.capability =
                    QaCapability::Failed("QA capability response is invalid.".into());
                state.check_selection = None;
                return QaTransition::notify("QA capability response is invalid.");
            }
            state.scope = Some(snapshot.selected_scope.clone());
            state.capability = QaCapability::Partial {
                snapshot: Box::new(snapshot),
                limitations: normalize_limitations(limitations),
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::CapabilityFailed(message) => {
            state.capability = QaCapability::Failed(message);
            state.check_selection = None;
            QaTransition::none()
        }
        QaAction::CycleScope => {
            let Some(snapshot) = state.capability.snapshot() else {
                return QaTransition::notify("QA capability is not available.");
            };
            let current = snapshot
                .scopes
                .iter()
                .position(|scope| Some(scope) == state.scope.as_ref())
                .unwrap_or(0);
            let Some(scope) = snapshot
                .scopes
                .get((current + 1) % snapshot.scopes.len().max(1))
                .cloned()
            else {
                return QaTransition::notify("No exact QA recipe scope is available.");
            };
            state.scope = Some(scope);
            state.drilled = false;
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::SelectCheck(delta) => {
            let checks = state
                .visible_checks()
                .into_iter()
                .map(|check| check.id.clone())
                .collect::<Vec<_>>();
            state.check_selection = select_index(&checks, state.check_selection.as_ref(), delta);
            state.drilled = false;
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::BeginSelectedCheck => {
            if state.active_session().is_some() {
                return QaTransition::notify("A QA operation is already active.");
            }
            let Some(check) = state.selected_check().cloned() else {
                return QaTransition::notify("Select an exact QA check first.");
            };
            let Some(task) = check.task.clone() else {
                return QaTransition::notify(
                    check
                        .availability
                        .disabled_reason()
                        .unwrap_or("The selected QA check is unavailable."),
                );
            };
            if !matches!(check.availability, QaCheckAvailability::Available) {
                return QaTransition::notify(
                    check
                        .availability
                        .disabled_reason()
                        .unwrap_or("The selected QA check is unavailable."),
                );
            }
            let request = BuildRequest {
                targets: vec![check.scope.recipe.name.clone()],
                task: Some(task),
                force: false,
            };
            if request.validate().is_err() {
                return QaTransition::notify("The capability supplied an invalid BitBake request.");
            }
            let preview = QaOperationPreview {
                id: QaOperationId(next_id(&mut state.operation_generation)),
                check: check.id,
                family: check.family,
                scope: check.scope,
                indexed_arguments: indexed_build_arguments(&request),
                request,
                report_roots: check.report_roots,
                limitations: check.limitations,
            };
            state.pending_operation = Some(preview.clone());
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::Operation(preview))),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmOperation(preview) => {
            if state.pending_operation.as_ref() != Some(&preview)
                || exact_capability_check(state, &preview).is_none()
                || state.active_session().is_some()
            {
                return QaTransition::notify(
                    "The QA confirmation is stale or no longer available.",
                );
            }
            let session = QaSessionId(next_id(&mut state.session_generation));
            state.sessions.push_back(QaSession {
                id: session,
                operation: preview.clone(),
                status: QaSessionStatus::Starting,
                background_job_id: None,
                started_at: SystemTime::now(),
                finished_at: None,
                message: None,
                result_paths: Vec::new(),
                output: VecDeque::new(),
                dropped_output: 0,
            });
            while state.sessions.len() > MAX_QA_SESSIONS {
                state.sessions.pop_front();
            }
            state.pending_operation = None;
            QaTransition {
                effect: Some(QaEffect::StartBuild {
                    session,
                    request: preview.request,
                }),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::AttachBackgroundJob {
            session,
            background_job,
        } => {
            if let Some(session) = session_mut(state, session)
                && !session.status.is_terminal()
            {
                session.background_job_id = Some(background_job);
            }
            QaTransition::none()
        }
        QaAction::SessionRunning(id) => {
            if let Some(session) = session_mut(state, id)
                && session.status == QaSessionStatus::Starting
            {
                session.status = QaSessionStatus::Running;
            }
            QaTransition::none()
        }
        QaAction::SessionOutput {
            session: id,
            stream,
            line,
            truncated,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
                && !line.is_empty()
                && line.len() <= MAX_QA_TEXT_BYTES
                && !line.contains('\0')
            {
                if session.output.len() == MAX_QA_SESSION_OUTPUT {
                    session.output.pop_front();
                    session.dropped_output = session.dropped_output.saturating_add(1);
                }
                session.output.push_back(QaOutputLine {
                    stream,
                    line,
                    truncated,
                });
            }
            QaTransition::none()
        }
        QaAction::CompleteSession {
            session: id,
            result_paths,
            finished_at,
        } => {
            let paths = normalize_paths(result_paths);
            let Some(session) = session_mut(state, id) else {
                return QaTransition::none();
            };
            if session.status.is_terminal() {
                return QaTransition::none();
            }
            session.status = QaSessionStatus::Succeeded;
            session.finished_at = Some(finished_at);
            session.result_paths = paths.clone();
            if paths.is_empty() {
                session.message = Some("no report supplied".into());
                return QaTransition::none();
            }
            match begin_report_request(state, paths) {
                Ok(effect) => QaTransition::effect(effect),
                Err(message) => QaTransition::notify(message),
            }
        }
        QaAction::FailSession {
            session: id,
            message,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Failed;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::TimeoutSession {
            session: id,
            forced,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::TimedOut;
                session.finished_at = Some(finished_at);
                session.message = Some(if forced {
                    "QA cancellation timed out; the process group was forced".into()
                } else {
                    "QA operation timed out".into()
                });
            }
            QaTransition::none()
        }
        QaAction::LoseSession {
            session: id,
            message,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Lost;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::BeginCancellation => {
            let Some(session) = state.active_session() else {
                return QaTransition::notify("No active QA operation can be cancelled.");
            };
            let Some(background_job) = session.background_job_id else {
                return QaTransition::notify(
                    "The active QA operation is not attached to a managed build yet.",
                );
            };
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::Cancellation {
                    session: session.id,
                    background_job,
                })),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmCancellation(id) => {
            let Some(session) = session_mut(state, id) else {
                return QaTransition::notify("The QA cancellation target is stale.");
            };
            let Some(background_job) = session.background_job_id else {
                return QaTransition::notify("The QA cancellation target has no managed job.");
            };
            if session.status.is_terminal() {
                return QaTransition::notify("The QA cancellation target is already complete.");
            }
            session.status = QaSessionStatus::Cancelling;
            QaTransition {
                effect: Some(QaEffect::CancelBuild {
                    session: id,
                    background_job,
                }),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::RejectCancellation {
            session: id,
            message,
        } => {
            if let Some(session) = session_mut(state, id)
                && session.status == QaSessionStatus::Cancelling
            {
                session.status = QaSessionStatus::Running;
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::CancelSession {
            session: id,
            finished_at,
        } => {
            if let Some(session) = session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Cancelled;
                session.finished_at = Some(finished_at);
            }
            QaTransition::none()
        }
        QaAction::BeginImport => QaTransition {
            dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                editor: {
                    let mut editor = PopupEditor::new(popup_toml_document(
                        "root",
                        "",
                        Some("normalized absolute QA report or bounded directory"),
                    ));
                    let _ = editor.select_toml_value("root");
                    editor
                },
                validation_error: None,
            })),
            ..QaTransition::none()
        },
        QaAction::UpdateImport(document) if document.len() <= MAX_QA_TEXT_BYTES => QaTransition {
            dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                editor: PopupEditor::new(document),
                validation_error: None,
            })),
            ..QaTransition::none()
        },
        QaAction::UpdateImport(_) => QaTransition::none(),
        QaAction::ConfirmImport(document) => {
            let root = match popup_toml_value(&document, "root") {
                Ok(root) => PathBuf::from(root),
                Err(message) => {
                    return QaTransition {
                        dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                            editor: PopupEditor::new(document),
                            validation_error: Some(message),
                        })),
                        ..QaTransition::none()
                    };
                }
            };
            if !absolute_normal_path(&root) {
                return QaTransition {
                    dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                        editor: PopupEditor::new(document),
                        validation_error: Some("`root` must be a normalized absolute path.".into()),
                    })),
                    ..QaTransition::none()
                };
            }
            match begin_report_request(state, vec![root]) {
                Ok(effect) => QaTransition {
                    effect: Some(effect),
                    dialog: QaDialogUpdate::Close,
                    notification: None,
                },
                Err(message) => QaTransition {
                    dialog: QaDialogUpdate::Open(Box::new(QaDialog::Import {
                        editor: PopupEditor::new(document),
                        validation_error: Some(message.into()),
                    })),
                    ..QaTransition::none()
                },
            }
        }
        QaAction::CancelDialog => {
            state.pending_operation = None;
            state.pending_layer_operation = None;
            QaTransition {
                dialog: QaDialogUpdate::Close,
                ..QaTransition::none()
            }
        }
        QaAction::RefreshReports => {
            let Some(request) = state.inventory.request().cloned() else {
                return QaTransition::notify("Import or run a QA check first.");
            };
            match begin_report_request(state, request.paths) {
                Ok(effect) => QaTransition::effect(effect),
                Err(message) => QaTransition::notify(message),
            }
        }
        QaAction::ReportsLoaded {
            request,
            reports,
            limitations,
        } if exact_request_matches(state, &request) => {
            let known_checks = state
                .capability
                .snapshot()
                .map(|snapshot| {
                    snapshot
                        .checks
                        .iter()
                        .map(|check| check.id.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let mut known_checks = known_checks;
            let mut known_scopes = state
                .capability
                .snapshot()
                .map(|snapshot| {
                    snapshot
                        .scopes
                        .iter()
                        .cloned()
                        .map(QaFindingScope::Recipe)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if let Some(snapshot) = state.layer_capability.snapshot() {
                known_checks.extend(snapshot.layers.iter().map(|layer| layer.check.clone()));
                known_checks.sort();
                known_checks.dedup();
                known_scopes.extend(
                    snapshot
                        .layers
                        .iter()
                        .map(|layer| QaFindingScope::Layer(layer.identity.clone())),
                );
                known_scopes.sort_by(|left, right| {
                    left.name()
                        .cmp(right.name())
                        .then_with(|| left.path().cmp(right.path()))
                });
                known_scopes.dedup();
            }
            let (reports, mut model_limitations) =
                normalize_qa_reports(reports, &known_checks, &known_scopes);
            model_limitations.extend(limitations);
            let limitations = normalize_limitations(model_limitations);
            state.inventory = if reports.is_empty() && limitations.is_empty() {
                QaReportInventoryState::AvailableEmpty { request }
            } else if limitations.is_empty() {
                QaReportInventoryState::Available { request, reports }
            } else {
                QaReportInventoryState::Partial {
                    request,
                    reports,
                    limitations,
                }
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsFailed {
            request,
            kind,
            message,
        } if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::Failed {
                request,
                kind,
                message,
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsCancelled(request) if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::Cancelled { request };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsTimedOut(request) if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::TimedOut { request };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsLost { request, message } if exact_request_matches(state, &request) => {
            state.inventory = QaReportInventoryState::Lost { request, message };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ReportsLoaded { .. }
        | QaAction::ReportsFailed { .. }
        | QaAction::ReportsCancelled(_)
        | QaAction::ReportsTimedOut(_)
        | QaAction::ReportsLost { .. } => QaTransition::none(),
        QaAction::SelectReport(delta) => {
            let reports = state
                .inventory
                .reports()
                .unwrap_or_default()
                .iter()
                .map(|report| report.identity.clone())
                .collect::<Vec<_>>();
            state.report_selection = select_index(&reports, state.report_selection.as_ref(), delta);
            QaTransition::none()
        }
        QaAction::SelectFinding(delta) => {
            let findings = state
                .visible_findings()
                .into_iter()
                .map(|finding| finding.identity.clone())
                .collect::<Vec<_>>();
            state.finding_selection =
                select_index(&findings, state.finding_selection.as_ref(), delta);
            QaTransition::none()
        }
        QaAction::Drill => {
            if (state.view == QaView::RecipeKernel && state.selected_check().is_some())
                || (state.view == QaView::LayerQa && state.selected_layer().is_some())
            {
                state.drilled = true;
                clamp_selection(state);
            }
            QaTransition::none()
        }
        QaAction::LeaveDrill => {
            state.drilled = false;
            QaTransition::none()
        }
        QaAction::BeginSearch => {
            state.searching = true;
            QaTransition::none()
        }
        QaAction::AppendQuery(character)
            if state.searching
                && !character.is_control()
                && state.query.len() + character.len_utf8() <= MAX_QA_QUERY_BYTES =>
        {
            state.query.push(character);
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::BackspaceQuery if state.searching => {
            state.query.pop();
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::ClearQuery => {
            state.query.clear();
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::FinishSearch => {
            state.searching = false;
            QaTransition::none()
        }
        QaAction::AppendQuery(_) | QaAction::BackspaceQuery => QaTransition::none(),
        QaAction::CycleStatusFilter => {
            state.status_filter = state.status_filter.next();
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::OpenSelectedReport => state.selected_report().map_or_else(
            || QaTransition::notify("Select an exact QA report first."),
            |report| QaTransition::effect(QaEffect::OpenReport(report.identity.clone())),
        ),
        QaAction::OpenProvider => state.scope.as_ref().map_or_else(
            || QaTransition::notify("No exact QA provider is selected."),
            |scope| QaTransition::effect(QaEffect::OpenProvider(scope.recipe.clone())),
        ),
        QaAction::OpenSelectedSource => state
            .selected_finding()
            .and_then(|finding| finding.source.clone())
            .map_or_else(
                || QaTransition::notify("No exact QA finding source is available."),
                |source| QaTransition::effect(QaEffect::OpenSource(source)),
            ),
        QaAction::InspectLayerCapability => {
            state.layer_capability = QaLayerCapability::Inspecting;
            QaTransition::effect(QaEffect::InspectLayerCapability)
        }
        QaAction::LayerCapabilityLoaded(snapshot) => {
            if !snapshot.is_valid() {
                state.layer_capability =
                    QaLayerCapability::Failed("Layer QA capability response is invalid.".into());
                state.layer_selection = None;
                return QaTransition::notify("Layer QA capability response is invalid.");
            }
            state.layer_selection = Some(snapshot.selected_layer.clone());
            state.layer_capability = QaLayerCapability::Available(Box::new(snapshot));
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::LayerCapabilityPartial {
            snapshot,
            limitations,
        } => {
            if !snapshot.is_valid() {
                state.layer_capability =
                    QaLayerCapability::Failed("Layer QA capability response is invalid.".into());
                state.layer_selection = None;
                return QaTransition::notify("Layer QA capability response is invalid.");
            }
            state.layer_selection = Some(snapshot.selected_layer.clone());
            state.layer_capability = QaLayerCapability::Partial {
                snapshot: Box::new(snapshot),
                limitations: normalize_limitations(limitations),
            };
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::LayerCapabilityFailed(message) => {
            state.layer_capability = QaLayerCapability::Failed(message);
            state.layer_selection = None;
            QaTransition::none()
        }
        QaAction::SelectLayer(delta) => {
            let layers = state
                .visible_layers()
                .into_iter()
                .map(|layer| layer.identity.clone())
                .collect::<Vec<_>>();
            state.layer_selection = select_index(&layers, state.layer_selection.as_ref(), delta);
            state.drilled = false;
            clamp_selection(state);
            QaTransition::none()
        }
        QaAction::BeginSelectedLayerCheck => {
            if state.active_layer_session().is_some() {
                return QaTransition::notify("A layer QA operation is already active.");
            }
            let Some(capability) = state.selected_layer().cloned() else {
                return QaTransition::notify("Select an exact configured layer first.");
            };
            let (executable, arguments, report_roots) = match capability.run {
                QaLayerRunCapability::Available {
                    executable,
                    arguments,
                    report_roots,
                } => (executable, arguments, report_roots),
                QaLayerRunCapability::Disabled(reason) => {
                    return QaTransition::notify(reason);
                }
            };
            let preview = QaLayerOperationPreview {
                id: QaLayerOperationId(next_id(&mut state.layer_operation_generation)),
                check: capability.check,
                layer: capability.identity,
                indexed_arguments: indexed_native_arguments(&executable, &arguments),
                executable,
                arguments,
                report_roots,
                limitations: capability.limitations,
            };
            state.pending_layer_operation = Some(preview.clone());
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::LayerOperation(preview))),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmLayerOperation(preview) => {
            if state.pending_layer_operation.as_ref() != Some(&preview)
                || exact_layer_capability(state, &preview).is_none()
                || state.active_layer_session().is_some()
            {
                return QaTransition::notify(
                    "The layer QA confirmation is stale or no longer available.",
                );
            }
            let session = QaLayerSessionId(next_id(&mut state.layer_session_generation));
            state.layer_sessions.push_back(QaLayerSession {
                id: session,
                operation: preview.clone(),
                status: QaSessionStatus::Starting,
                started_at: SystemTime::now(),
                finished_at: None,
                exit_code: None,
                message: None,
                result_paths: Vec::new(),
                output: VecDeque::new(),
                dropped_output: 0,
            });
            while state.layer_sessions.len() > MAX_QA_SESSIONS {
                state.layer_sessions.pop_front();
            }
            state.pending_layer_operation = None;
            QaTransition {
                effect: Some(QaEffect::StartLayerCheck {
                    session,
                    layer: preview.layer,
                    executable: preview.executable,
                    arguments: preview.arguments,
                }),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::LayerSessionRunning(id) => {
            if let Some(session) = layer_session_mut(state, id)
                && session.status == QaSessionStatus::Starting
            {
                session.status = QaSessionStatus::Running;
            }
            QaTransition::none()
        }
        QaAction::LayerSessionOutput {
            session: id,
            stream,
            line,
            truncated,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
                && !line.is_empty()
                && line.len() <= MAX_QA_TEXT_BYTES
                && !line.contains('\0')
            {
                if session.output.len() == MAX_QA_SESSION_OUTPUT {
                    session.output.pop_front();
                    session.dropped_output = session.dropped_output.saturating_add(1);
                }
                session.output.push_back(QaOutputLine {
                    stream,
                    line,
                    truncated,
                });
            }
            QaTransition::none()
        }
        QaAction::CompleteLayerSession {
            session: id,
            exit_code,
            result_paths,
            finished_at,
        } => {
            let paths = normalize_paths(result_paths);
            let Some(session) = layer_session_mut(state, id) else {
                return QaTransition::none();
            };
            if session.status.is_terminal() {
                return QaTransition::none();
            }
            session.finished_at = Some(finished_at);
            session.exit_code = Some(exit_code);
            session.result_paths = paths.clone();
            if exit_code != 0 {
                session.status = QaSessionStatus::Failed;
                session.message = Some(format!("yocto-check-layer exited with status {exit_code}"));
                return QaTransition::none();
            }
            session.status = QaSessionStatus::Succeeded;
            if paths.is_empty() {
                session.message = Some("no report supplied".into());
                return QaTransition::none();
            }
            match begin_report_request(state, paths) {
                Ok(effect) => QaTransition::effect(effect),
                Err(message) => QaTransition::notify(message),
            }
        }
        QaAction::FailLayerSession {
            session: id,
            exit_code,
            message,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Failed;
                session.finished_at = Some(finished_at);
                session.exit_code = exit_code;
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::TimeoutLayerSession {
            session: id,
            forced,
            exit_code,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::TimedOut;
                session.finished_at = Some(finished_at);
                session.exit_code = exit_code;
                session.message = Some(if forced {
                    "Layer QA timed out and required forced termination".into()
                } else {
                    "Layer QA timed out".into()
                });
            }
            QaTransition::none()
        }
        QaAction::LoseLayerSession {
            session: id,
            message,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Lost;
                session.finished_at = Some(finished_at);
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::BeginLayerCancellation => {
            let Some(session) = state.active_layer_session() else {
                return QaTransition::notify("No active layer QA operation can be cancelled.");
            };
            QaTransition {
                dialog: QaDialogUpdate::Open(Box::new(QaDialog::LayerCancellation(session.id))),
                ..QaTransition::none()
            }
        }
        QaAction::ConfirmLayerCancellation(id) => {
            let Some(session) = layer_session_mut(state, id) else {
                return QaTransition::notify("The layer QA cancellation target is stale.");
            };
            if session.status.is_terminal() {
                return QaTransition::notify(
                    "The layer QA cancellation target is already complete.",
                );
            }
            session.status = QaSessionStatus::Cancelling;
            QaTransition {
                effect: Some(QaEffect::CancelLayerCheck(id)),
                dialog: QaDialogUpdate::Close,
                notification: None,
            }
        }
        QaAction::RejectLayerCancellation {
            session: id,
            message,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && session.status == QaSessionStatus::Cancelling
            {
                session.status = QaSessionStatus::Running;
                session.message = Some(message);
            }
            QaTransition::none()
        }
        QaAction::CancelLayerSession {
            session: id,
            forced,
            exit_code,
            finished_at,
        } => {
            if let Some(session) = layer_session_mut(state, id)
                && !session.status.is_terminal()
            {
                session.status = QaSessionStatus::Cancelled;
                session.finished_at = Some(finished_at);
                session.exit_code = exit_code;
                session.message = forced.then(|| "Layer QA required forced termination".into());
            }
            QaTransition::none()
        }
        QaAction::OpenSelectedLayerRoot => state.selected_layer().map_or_else(
            || QaTransition::notify("No exact configured layer is selected."),
            |layer| QaTransition::effect(QaEffect::OpenLayerRoot(layer.identity.clone())),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, App, Dialog, Effect, FocusTarget, update};

    fn scope(name: &str) -> QaScope {
        QaScope::new(RecipeIdentity {
            name: name.into(),
            file: format!("/layers/meta/recipes/{name}/{name}_1.0.bb").into(),
        })
        .unwrap()
    }

    fn check(
        id: &str,
        family: QaCheckFamily,
        scope: QaScope,
        task: Option<&str>,
    ) -> QaCheckCapability {
        let availability = task.map_or_else(
            || QaCheckAvailability::Disabled("task not reported".into()),
            |_| QaCheckAvailability::Available,
        );
        QaCheckCapability::new(
            QaCheckId::new(id.into()).unwrap(),
            family,
            format!("{family:?}"),
            scope,
            task.map(str::to_owned),
            vec!["/build/tmp/log/qa".into()],
            availability,
            vec![],
        )
        .unwrap()
    }

    fn capability() -> QaCapabilitySnapshot {
        let kernel = scope("linux-yocto");
        let busybox = scope("busybox");
        QaCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            kernel.clone(),
            vec![kernel.clone(), busybox.clone()],
            vec![
                check(
                    "kernel-config",
                    QaCheckFamily::KernelConfiguration,
                    kernel.clone(),
                    Some("kernel_configcheck"),
                ),
                check(
                    "uri-check",
                    QaCheckFamily::UriFetch,
                    kernel.clone(),
                    Some("checkuri"),
                ),
                check("patch-check", QaCheckFamily::Patch, kernel.clone(), None),
                check(
                    "license-check",
                    QaCheckFamily::License,
                    busybox.clone(),
                    Some("populate_lic"),
                ),
                check(
                    "recipe-qa",
                    QaCheckFamily::RecipePackage,
                    busybox,
                    Some("package_qa"),
                ),
            ],
            vec![],
        )
        .unwrap()
    }

    fn load(state: &mut QaState) {
        let _ = update_qa(state, QaAction::CapabilityLoaded(capability()));
    }

    fn begin(state: &mut QaState) -> QaOperationPreview {
        let transition = update_qa(state, QaAction::BeginSelectedCheck);
        let QaDialogUpdate::Open(dialog) = transition.dialog else {
            panic!("expected operation preview")
        };
        let QaDialog::Operation(preview) = *dialog else {
            panic!("expected operation preview")
        };
        preview
    }

    fn start(state: &mut QaState) -> QaSessionId {
        let preview = begin(state);
        let transition = update_qa(state, QaAction::ConfirmOperation(preview));
        let Some(QaEffect::StartBuild { session, .. }) = transition.effect else {
            panic!("expected managed build")
        };
        session
    }

    fn report_identity(check: QaCheckId) -> QaReportIdentity {
        QaReportIdentity::new(
            "/build/tmp/log/qa/report.json".into(),
            100,
            SystemTime::UNIX_EPOCH,
            "abc123".into(),
            QaReportFormat::Json,
            Some(check),
            Some(QaFindingScope::Recipe(scope("linux-yocto"))),
        )
        .unwrap()
    }

    fn finding(status: QaFindingStatus, fingerprint: &str) -> QaFinding {
        QaFinding {
            identity: QaFindingIdentity::new(
                QaCheckId::new("kernel-config".into()).unwrap(),
                fingerprint.into(),
            )
            .unwrap(),
            status,
            severity: Some("warning".into()),
            message: "CONFIG_DEVMEM differs from policy".into(),
            scope: QaFindingScope::Recipe(scope("linux-yocto")),
            task: Some("kernel_configcheck".into()),
            test_name: None,
            source: Some(
                QaSourceLocation::new("/layers/meta/cfg/policy.cfg".into(), Some(7), None).unwrap(),
            ),
            rule: Some("CONFIG_DEVMEM".into()),
            suggestion: Some("disable the option".into()),
            metadata: vec![],
        }
    }

    fn report(findings: Vec<QaFinding>) -> QaReport {
        QaReport {
            identity: report_identity(QaCheckId::new("kernel-config".into()).unwrap()),
            findings,
            metadata: vec![],
            limitations: vec![],
        }
    }

    #[test]
    fn qa_check_workflow_capability_catalog_never_guesses_tasks() {
        assert!(
            QaScope::new(RecipeIdentity {
                name: "bad/name".into(),
                file: "/provider.bb".into(),
            })
            .is_err()
        );
        assert!(
            QaCheckCapability::new(
                QaCheckId::new("missing".into()).unwrap(),
                QaCheckFamily::Patch,
                "Patch".into(),
                scope("busybox"),
                None,
                vec![],
                QaCheckAvailability::Available,
                vec![],
            )
            .is_err()
        );
        assert!(
            QaReportRequest::new(
                1,
                vec![PathBuf::from("/reports"), PathBuf::from("../escape")]
            )
            .is_err()
        );
        let mut state = QaState::default();
        load(&mut state);
        assert_eq!(state.visible_checks().len(), 3);
        let _ = update_qa(&mut state, QaAction::SelectCheck(1));
        let transition = update_qa(&mut state, QaAction::BeginSelectedCheck);
        assert_eq!(
            transition.notification.as_deref(),
            Some("task not reported")
        );
        assert!(state.sessions.is_empty());

        let mut invalid = capability();
        invalid.checks[0].task = Some("guessed/task".into());
        let transition = update_qa(&mut state, QaAction::CapabilityLoaded(invalid));
        assert_eq!(
            transition.notification.as_deref(),
            Some("QA capability response is invalid.")
        );
        assert!(matches!(state.capability, QaCapability::Failed(_)));

        let mut partial = QaState::default();
        let _ = update_qa(
            &mut partial,
            QaAction::CapabilityPartial {
                snapshot: capability(),
                limitations: vec!["license metadata unavailable".into()],
            },
        );
        assert!(matches!(partial.capability, QaCapability::Partial { .. }));
    }

    #[test]
    fn qa_check_workflow_preview_is_indexed_exact_and_stale_safe() {
        let mut state = QaState::default();
        load(&mut state);
        let preview = begin(&mut state);
        assert_eq!(
            preview.indexed_arguments,
            [
                "0: bitbake",
                "1: linux-yocto",
                "2: -c",
                "3: kernel_configcheck"
            ]
        );
        let mut stale = preview.clone();
        stale.request.task = Some("guessed_task".into());
        let transition = update_qa(&mut state, QaAction::ConfirmOperation(stale));
        assert!(transition.effect.is_none());
        assert!(state.sessions.is_empty());

        let transition = update_qa(&mut state, QaAction::ConfirmOperation(preview.clone()));
        assert!(matches!(
            transition.effect,
            Some(QaEffect::StartBuild {
                request: BuildRequest {
                    ref targets,
                    task: Some(ref task),
                    force: false,
                },
                ..
            }) if targets == &["linux-yocto"] && task == "kernel_configcheck"
        ));
        assert!(matches!(transition.dialog, QaDialogUpdate::Close));
        assert_eq!(state.sessions.len(), 1);

        let duplicate = update_qa(&mut state, QaAction::ConfirmOperation(preview));
        assert!(duplicate.effect.is_none());
    }

    #[test]
    fn qa_check_workflow_session_lifecycle_output_bounds_and_cancellation_are_typed() {
        let mut state = QaState::default();
        load(&mut state);
        let id = start(&mut state);
        let job = BackgroundJobId(41);
        let _ = update_qa(
            &mut state,
            QaAction::AttachBackgroundJob {
                session: id,
                background_job: job,
            },
        );
        let _ = update_qa(&mut state, QaAction::SessionRunning(id));
        for index in 0..=MAX_QA_SESSION_OUTPUT {
            let _ = update_qa(
                &mut state,
                QaAction::SessionOutput {
                    session: id,
                    stream: QaOutputStream::Stdout,
                    line: format!("line {index}"),
                    truncated: false,
                },
            );
        }
        assert_eq!(state.sessions[0].output.len(), MAX_QA_SESSION_OUTPUT);
        assert_eq!(state.sessions[0].dropped_output, 1);

        let transition = update_qa(&mut state, QaAction::BeginCancellation);
        assert!(matches!(
            transition.dialog,
            QaDialogUpdate::Open(dialog)
                if matches!(
                    *dialog,
                    QaDialog::Cancellation {
                        session,
                        background_job
                    } if session == id && background_job == job
                )
        ));
        let transition = update_qa(&mut state, QaAction::ConfirmCancellation(id));
        assert_eq!(
            transition.effect,
            Some(QaEffect::CancelBuild {
                session: id,
                background_job: job
            })
        );
        let _ = update_qa(
            &mut state,
            QaAction::RejectCancellation {
                session: id,
                message: "coordinator busy".into(),
            },
        );
        assert_eq!(state.sessions[0].status, QaSessionStatus::Running);
        let _ = update_qa(
            &mut state,
            QaAction::TimeoutSession {
                session: id,
                forced: true,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(state.sessions[0].status, QaSessionStatus::TimedOut);
    }

    #[test]
    fn qa_check_workflow_keeps_failure_cancel_timeout_and_loss_distinct() {
        let mut failed = QaState::default();
        load(&mut failed);
        let id = start(&mut failed);
        let _ = update_qa(
            &mut failed,
            QaAction::FailSession {
                session: id,
                message: "task failed".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(failed.sessions[0].status, QaSessionStatus::Failed);

        let mut cancelled = QaState::default();
        load(&mut cancelled);
        let id = start(&mut cancelled);
        let _ = update_qa(
            &mut cancelled,
            QaAction::CancelSession {
                session: id,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(cancelled.sessions[0].status, QaSessionStatus::Cancelled);

        let mut timed_out = QaState::default();
        load(&mut timed_out);
        let id = start(&mut timed_out);
        let _ = update_qa(
            &mut timed_out,
            QaAction::TimeoutSession {
                session: id,
                forced: false,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(timed_out.sessions[0].status, QaSessionStatus::TimedOut);

        let mut lost = QaState::default();
        load(&mut lost);
        let id = start(&mut lost);
        let _ = update_qa(
            &mut lost,
            QaAction::LoseSession {
                session: id,
                message: "coordinator channel closed".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(lost.sessions[0].status, QaSessionStatus::Lost);
    }

    #[test]
    fn qa_check_workflow_session_history_is_bounded() {
        let mut state = QaState::default();
        load(&mut state);
        for _ in 0..=MAX_QA_SESSIONS {
            let id = start(&mut state);
            let _ = update_qa(
                &mut state,
                QaAction::CompleteSession {
                    session: id,
                    result_paths: vec![],
                    finished_at: SystemTime::UNIX_EPOCH,
                },
            );
        }
        assert_eq!(state.sessions.len(), MAX_QA_SESSIONS);
        assert_eq!(state.sessions.front().unwrap().id, QaSessionId(2));
    }

    #[test]
    fn qa_check_workflow_success_scans_exact_results_and_empty_is_honest() {
        let mut state = QaState::default();
        load(&mut state);
        let id = start(&mut state);
        let transition = update_qa(
            &mut state,
            QaAction::CompleteSession {
                session: id,
                result_paths: vec!["/build/tmp/log/qa/report.json".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let Some(QaEffect::ImportReports(request)) = transition.effect else {
            panic!("expected report scan")
        };
        assert_eq!(
            request.paths,
            [PathBuf::from("/build/tmp/log/qa/report.json")]
        );
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::Loading { .. }
        ));

        let mut other = QaState::default();
        load(&mut other);
        let id = start(&mut other);
        let transition = update_qa(
            &mut other,
            QaAction::CompleteSession {
                session: id,
                result_paths: vec![],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert!(transition.effect.is_none());
        assert_eq!(
            other.sessions[0].message.as_deref(),
            Some("no report supplied")
        );
    }

    #[test]
    fn qa_check_workflow_report_generations_bounds_partial_and_terminal_states() {
        let mut state = QaState::default();
        load(&mut state);
        let transition = update_qa(
            &mut state,
            QaAction::ConfirmImport("root = \"/reports\"\n".into()),
        );
        let Some(QaEffect::ImportReports(request)) = transition.effect else {
            panic!("expected report import")
        };
        let stale = QaReportRequest::new(request.generation + 1, request.paths.clone()).unwrap();
        let _ = update_qa(
            &mut state,
            QaAction::ReportsLoaded {
                request: stale,
                reports: vec![report(vec![finding(QaFindingStatus::Failed, "a")])],
                limitations: vec![],
            },
        );
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::Loading { .. }
        ));
        let _ = update_qa(
            &mut state,
            QaAction::ReportsLoaded {
                request: request.clone(),
                reports: vec![report(vec![
                    finding(QaFindingStatus::Failed, "a"),
                    finding(QaFindingStatus::Warning, "b"),
                ])],
                limitations: vec!["one input record was malformed".into()],
            },
        );
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::Partial { .. }
        ));
        assert_eq!(state.visible_findings().len(), 2);

        let refresh = update_qa(&mut state, QaAction::RefreshReports);
        let Some(QaEffect::ImportReports(refresh_request)) = refresh.effect else {
            panic!("expected refresh")
        };
        let _ = update_qa(
            &mut state,
            QaAction::ReportsFailed {
                request: refresh_request,
                kind: QaReportFailureKind::PermissionDenied,
                message: "permission denied".into(),
            },
        );
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::Failed {
                kind: QaReportFailureKind::PermissionDenied,
                ..
            }
        ));

        let mut findings = (0..=MAX_QA_FINDINGS)
            .map(|index| finding(QaFindingStatus::Warning, &format!("finding{index}")))
            .collect::<Vec<_>>();
        findings.push(findings[0].clone());
        let (reports, limitations) = normalize_qa_reports(
            vec![report(findings)],
            &[QaCheckId::new("kernel-config".into()).unwrap()],
            &[QaFindingScope::Recipe(scope("linux-yocto"))],
        );
        assert_eq!(reports[0].findings.len(), MAX_QA_FINDINGS);
        assert!(
            limitations
                .iter()
                .any(|value| value.contains("beyond the model bound"))
        );

        let cancelled =
            QaReportRequest::new(request.generation + 10, vec!["/reports".into()]).unwrap();
        state.inventory = QaReportInventoryState::Loading {
            request: cancelled.clone(),
        };
        let _ = update_qa(&mut state, QaAction::ReportsCancelled(cancelled.clone()));
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::Cancelled { .. }
        ));
        state.inventory = QaReportInventoryState::Loading {
            request: cancelled.clone(),
        };
        let _ = update_qa(&mut state, QaAction::ReportsTimedOut(cancelled.clone()));
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::TimedOut { .. }
        ));
        state.inventory = QaReportInventoryState::Loading {
            request: cancelled.clone(),
        };
        let _ = update_qa(
            &mut state,
            QaAction::ReportsLost {
                request: cancelled,
                message: "worker channel closed".into(),
            },
        );
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::Lost { .. }
        ));

        let empty = QaReportRequest::new(request.generation + 11, vec!["/reports".into()]).unwrap();
        state.inventory = QaReportInventoryState::Loading {
            request: empty.clone(),
        };
        let _ = update_qa(
            &mut state,
            QaAction::ReportsLoaded {
                request: empty,
                reports: vec![],
                limitations: vec![],
            },
        );
        assert!(matches!(
            state.inventory,
            QaReportInventoryState::AvailableEmpty { .. }
        ));
    }

    #[test]
    fn qa_check_workflow_search_filter_selection_drill_and_exact_opens() {
        let mut state = QaState::default();
        load(&mut state);
        let request = QaReportRequest::new(1, vec!["/reports".into()]).unwrap();
        state.inventory = QaReportInventoryState::Loading {
            request: request.clone(),
        };
        let _ = update_qa(
            &mut state,
            QaAction::ReportsLoaded {
                request,
                reports: vec![report(vec![
                    finding(QaFindingStatus::Failed, "failure"),
                    finding(QaFindingStatus::Warning, "warning"),
                ])],
                limitations: vec![],
            },
        );
        let _ = update_qa(&mut state, QaAction::Drill);
        assert!(state.drilled);
        let _ = update_qa(&mut state, QaAction::CycleStatusFilter);
        assert_eq!(state.status_filter, QaStatusFilter::Failed);
        assert_eq!(state.visible_findings().len(), 1);
        let _ = update_qa(&mut state, QaAction::BeginSearch);
        for character in "devmem".chars() {
            let _ = update_qa(&mut state, QaAction::AppendQuery(character));
        }
        assert_eq!(state.visible_findings().len(), 1);
        let source = state.selected_finding().unwrap().source.clone().unwrap();
        assert_eq!(
            update_qa(&mut state, QaAction::OpenSelectedSource).effect,
            Some(QaEffect::OpenSource(source))
        );
        assert!(matches!(
            update_qa(&mut state, QaAction::OpenSelectedReport).effect,
            Some(QaEffect::OpenReport(_))
        ));
        assert_eq!(
            update_qa(&mut state, QaAction::OpenProvider).effect,
            Some(QaEffect::OpenProvider(scope("linux-yocto").recipe))
        );
    }

    #[test]
    fn qa_check_workflow_scope_cycle_preserves_exact_provider_identity() {
        let mut state = QaState::default();
        load(&mut state);
        let first = state.scope.clone().unwrap();
        let _ = update_qa(&mut state, QaAction::CycleScope);
        assert_ne!(state.scope.as_ref(), Some(&first));
        assert_eq!(
            state.scope.as_ref().unwrap().recipe.file,
            PathBuf::from("/layers/meta/recipes/busybox/busybox_1.0.bb")
        );
        assert_eq!(state.visible_checks().len(), 2);
    }

    fn layer(name: &str) -> QaLayerIdentity {
        QaLayerIdentity::new(name.into(), format!("/layers/{name}").into()).unwrap()
    }

    fn layer_executable() -> QaExecutableIdentity {
        QaExecutableIdentity::new(
            "/poky/scripts/yocto-check-layer".into(),
            1_024,
            SystemTime::UNIX_EPOCH,
        )
        .unwrap()
    }

    fn layer_row(name: &str, available: bool) -> QaConfiguredLayerCapability {
        let identity = layer(name);
        let run = if available {
            QaLayerRunCapability::Available {
                executable: layer_executable(),
                arguments: vec![
                    "--layer".into(),
                    identity.root.to_string_lossy().into_owned(),
                ],
                report_roots: vec!["/build/tmp/log/qa-layer".into()],
            }
        } else {
            QaLayerRunCapability::Disabled("yocto-check-layer is unavailable".into())
        };
        QaConfiguredLayerCapability::new(
            QaCheckId::new("yocto-check-layer".into()).unwrap(),
            identity,
            vec!["whinlatter".into()],
            run,
            vec![],
        )
        .unwrap()
    }

    fn layer_capability() -> QaLayerCapabilitySnapshot {
        QaLayerCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            layer("meta"),
            vec![layer_row("meta", true), layer_row("meta-custom", false)],
            vec![],
        )
        .unwrap()
    }

    fn load_layer(state: &mut QaState) {
        state.view = QaView::LayerQa;
        let _ = update_qa(state, QaAction::LayerCapabilityLoaded(layer_capability()));
    }

    fn begin_layer(state: &mut QaState) -> QaLayerOperationPreview {
        let transition = update_qa(state, QaAction::BeginSelectedLayerCheck);
        let QaDialogUpdate::Open(dialog) = transition.dialog else {
            panic!("expected layer QA preview")
        };
        let QaDialog::LayerOperation(preview) = *dialog else {
            panic!("expected layer QA preview")
        };
        preview
    }

    fn start_layer(state: &mut QaState) -> QaLayerSessionId {
        let preview = begin_layer(state);
        let transition = update_qa(state, QaAction::ConfirmLayerOperation(preview));
        let Some(QaEffect::StartLayerCheck { session, .. }) = transition.effect else {
            panic!("expected layer QA start")
        };
        session
    }

    fn layer_finding(status: QaFindingStatus, fingerprint: &str) -> QaFinding {
        QaFinding {
            identity: QaFindingIdentity::new(
                QaCheckId::new("yocto-check-layer".into()).unwrap(),
                fingerprint.into(),
            )
            .unwrap(),
            status,
            severity: Some("warning".into()),
            message: "layer compatibility declaration is incomplete".into(),
            scope: QaFindingScope::Layer(layer("meta")),
            task: None,
            test_name: Some("LayerCompatibility".into()),
            source: Some(
                QaSourceLocation::new("/layers/meta/conf/layer.conf".into(), Some(12), None)
                    .unwrap(),
            ),
            rule: Some("LAYERSERIES_COMPAT".into()),
            suggestion: Some("declare the active release series".into()),
            metadata: vec![],
        }
    }

    fn layer_report(findings: Vec<QaFinding>) -> QaReport {
        let check = QaCheckId::new("yocto-check-layer".into()).unwrap();
        QaReport {
            identity: QaReportIdentity::new(
                "/build/tmp/log/qa-layer/report.json".into(),
                512,
                SystemTime::UNIX_EPOCH,
                "layerreport".into(),
                QaReportFormat::Json,
                Some(check),
                Some(QaFindingScope::Layer(layer("meta"))),
            )
            .unwrap(),
            findings,
            metadata: vec![],
            limitations: vec![],
        }
    }

    #[test]
    fn qa_layer_workflow_keeps_configured_disabled_layers_and_rejects_invalid_identities() {
        assert!(QaLayerIdentity::new("meta".into(), "../meta".into()).is_err());
        let identity = layer("meta");
        assert!(
            QaConfiguredLayerCapability::new(
                QaCheckId::new("yocto-check-layer".into()).unwrap(),
                identity,
                vec![],
                QaLayerRunCapability::Available {
                    executable: layer_executable(),
                    arguments: vec!["--layer".into(), "/arbitrary/not-configured".into()],
                    report_roots: vec![],
                },
                vec![],
            )
            .is_err()
        );

        let mut state = QaState::default();
        assert!(matches!(
            state.layer_capability,
            QaLayerCapability::NotInspected
        ));
        assert_eq!(
            update_qa(&mut state, QaAction::CycleView).effect,
            Some(QaEffect::InspectLayerCapability)
        );
        assert_eq!(state.view, QaView::LayerQa);
        assert!(matches!(
            state.layer_capability,
            QaLayerCapability::Inspecting
        ));
        load_layer(&mut state);
        assert_eq!(state.visible_layers().len(), 2);
        let _ = update_qa(&mut state, QaAction::SelectLayer(1));
        let transition = update_qa(&mut state, QaAction::BeginSelectedLayerCheck);
        assert_eq!(
            transition.notification.as_deref(),
            Some("yocto-check-layer is unavailable")
        );

        let mut partial = QaState::default();
        let _ = update_qa(
            &mut partial,
            QaAction::LayerCapabilityPartial {
                snapshot: layer_capability(),
                limitations: vec!["one compatibility series was unavailable".into()],
            },
        );
        assert!(matches!(
            partial.layer_capability,
            QaLayerCapability::Partial { .. }
        ));
        let _ = update_qa(
            &mut partial,
            QaAction::LayerCapabilityFailed("inspection failed".into()),
        );
        assert!(matches!(
            partial.layer_capability,
            QaLayerCapability::Failed(_)
        ));
    }

    #[test]
    fn qa_layer_workflow_preview_is_exact_indexed_and_stale_safe() {
        let mut state = QaState::default();
        load_layer(&mut state);
        let preview = begin_layer(&mut state);
        assert_eq!(
            preview.indexed_arguments,
            [
                "0: /poky/scripts/yocto-check-layer",
                "1: --layer",
                "2: /layers/meta"
            ]
        );
        let mut stale_layer = preview.clone();
        stale_layer.layer = layer("meta-custom");
        let transition = update_qa(&mut state, QaAction::ConfirmLayerOperation(stale_layer));
        assert!(transition.effect.is_none());
        assert!(state.layer_sessions.is_empty());
        let mut stale_tool = preview.clone();
        stale_tool.executable.byte_size += 1;
        let transition = update_qa(&mut state, QaAction::ConfirmLayerOperation(stale_tool));
        assert!(transition.effect.is_none());
        assert!(state.layer_sessions.is_empty());

        let transition = update_qa(&mut state, QaAction::ConfirmLayerOperation(preview.clone()));
        assert!(matches!(
            transition.effect,
            Some(QaEffect::StartLayerCheck {
                layer: QaLayerIdentity { ref name, .. },
                ref executable,
                ref arguments,
                ..
            }) if name == "meta"
                && executable == &layer_executable()
                && arguments == &["--layer", "/layers/meta"]
        ));
        assert!(matches!(transition.dialog, QaDialogUpdate::Close));
        assert!(
            update_qa(&mut state, QaAction::ConfirmLayerOperation(preview))
                .effect
                .is_none()
        );
    }

    #[test]
    fn qa_layer_workflow_lifecycle_output_cancellation_and_terminal_states_are_distinct() {
        let mut state = QaState::default();
        load_layer(&mut state);
        let id = start_layer(&mut state);
        let _ = update_qa(&mut state, QaAction::LayerSessionRunning(id));
        for index in 0..=MAX_QA_SESSION_OUTPUT {
            let _ = update_qa(
                &mut state,
                QaAction::LayerSessionOutput {
                    session: id,
                    stream: QaOutputStream::Stderr,
                    line: format!("line {index}"),
                    truncated: false,
                },
            );
        }
        assert_eq!(state.layer_sessions[0].output.len(), MAX_QA_SESSION_OUTPUT);
        assert_eq!(state.layer_sessions[0].dropped_output, 1);
        assert!(matches!(
            update_qa(&mut state, QaAction::BeginLayerCancellation).dialog,
            QaDialogUpdate::Open(dialog)
                if matches!(*dialog, QaDialog::LayerCancellation(session) if session == id)
        ));
        assert_eq!(
            update_qa(&mut state, QaAction::ConfirmLayerCancellation(id)).effect,
            Some(QaEffect::CancelLayerCheck(id))
        );
        let _ = update_qa(
            &mut state,
            QaAction::RejectLayerCancellation {
                session: id,
                message: "runner busy".into(),
            },
        );
        assert_eq!(state.layer_sessions[0].status, QaSessionStatus::Running);
        let _ = update_qa(
            &mut state,
            QaAction::CompleteLayerSession {
                session: id,
                exit_code: 2,
                result_paths: vec![],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(state.layer_sessions[0].status, QaSessionStatus::Failed);
        assert_eq!(state.layer_sessions[0].exit_code, Some(2));

        let mut cancelled = QaState::default();
        load_layer(&mut cancelled);
        let id = start_layer(&mut cancelled);
        let _ = update_qa(
            &mut cancelled,
            QaAction::CancelLayerSession {
                session: id,
                forced: true,
                exit_code: None,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            cancelled.layer_sessions[0].status,
            QaSessionStatus::Cancelled
        );

        let mut timed_out = QaState::default();
        load_layer(&mut timed_out);
        let id = start_layer(&mut timed_out);
        let _ = update_qa(
            &mut timed_out,
            QaAction::TimeoutLayerSession {
                session: id,
                forced: false,
                exit_code: None,
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(
            timed_out.layer_sessions[0].status,
            QaSessionStatus::TimedOut
        );

        let mut lost = QaState::default();
        load_layer(&mut lost);
        let id = start_layer(&mut lost);
        let _ = update_qa(
            &mut lost,
            QaAction::LoseLayerSession {
                session: id,
                message: "runner channel closed".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        assert_eq!(lost.layer_sessions[0].status, QaSessionStatus::Lost);
    }

    #[test]
    fn qa_layer_workflow_reports_counts_filters_drill_and_exact_opens() {
        let mut state = QaState::default();
        load_layer(&mut state);
        let id = start_layer(&mut state);
        let transition = update_qa(
            &mut state,
            QaAction::CompleteLayerSession {
                session: id,
                exit_code: 0,
                result_paths: vec!["/build/tmp/log/qa-layer/report.json".into()],
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let Some(QaEffect::ImportReports(request)) = transition.effect else {
            panic!("expected exact layer report scan")
        };
        let _ = update_qa(
            &mut state,
            QaAction::ReportsLoaded {
                request,
                reports: vec![layer_report(vec![
                    layer_finding(QaFindingStatus::Failed, "layer-fail"),
                    layer_finding(QaFindingStatus::Warning, "layer-warning"),
                ])],
                limitations: vec![],
            },
        );
        assert_eq!(
            state.layer_finding_counts(&layer("meta")),
            QaFindingCounts {
                failed: 1,
                warnings: 1,
                ..QaFindingCounts::default()
            }
        );
        let _ = update_qa(&mut state, QaAction::CycleStatusFilter);
        assert_eq!(state.status_filter, QaStatusFilter::Failed);
        assert_eq!(state.visible_findings().len(), 1);
        let _ = update_qa(&mut state, QaAction::BeginSearch);
        for character in "compatibility".chars() {
            let _ = update_qa(&mut state, QaAction::AppendQuery(character));
        }
        assert_eq!(state.visible_layers().len(), 1);
        let _ = update_qa(&mut state, QaAction::Drill);
        assert!(state.drilled);
        assert!(matches!(
            update_qa(&mut state, QaAction::OpenSelectedSource).effect,
            Some(QaEffect::OpenSource(_))
        ));
        assert_eq!(
            update_qa(&mut state, QaAction::OpenSelectedLayerRoot).effect,
            Some(QaEffect::OpenLayerRoot(layer("meta")))
        );
    }

    #[test]
    fn qa_layer_workflow_native_session_is_independent_from_managed_build_session() {
        let mut state = QaState::default();
        load(&mut state);
        let recipe_session = start(&mut state);
        load_layer(&mut state);
        let layer_session = start_layer(&mut state);
        assert_eq!(state.active_session().unwrap().id, recipe_session);
        assert_eq!(state.active_layer_session().unwrap().id, layer_session);
        assert!(matches!(
            update_qa(&mut state, QaAction::BeginLayerCancellation).dialog,
            QaDialogUpdate::Open(dialog)
                if matches!(*dialog, QaDialog::LayerCancellation(id) if id == layer_session)
        ));
        assert_eq!(
            update_qa(
                &mut state,
                QaAction::AttachBackgroundJob {
                    session: recipe_session,
                    background_job: BackgroundJobId(77),
                }
            )
            .effect,
            None
        );
        assert!(matches!(
            update_qa(&mut state, QaAction::BeginCancellation).dialog,
            QaDialogUpdate::Open(dialog)
                if matches!(
                    *dialog,
                    QaDialog::Cancellation {
                        session,
                        background_job
                    } if session == recipe_session && background_job == BackgroundJobId(77)
                )
        ));
    }

    #[test]
    fn qa_layer_workflow_history_is_bounded_and_dialog_focus_is_trapped() {
        let mut state = QaState::default();
        load_layer(&mut state);
        for _ in 0..=MAX_QA_SESSIONS {
            let id = start_layer(&mut state);
            let _ = update_qa(
                &mut state,
                QaAction::CompleteLayerSession {
                    session: id,
                    exit_code: 0,
                    result_paths: vec![],
                    finished_at: SystemTime::UNIX_EPOCH,
                },
            );
        }
        assert_eq!(state.layer_sessions.len(), MAX_QA_SESSIONS);
        assert_eq!(
            state.layer_sessions.front().unwrap().id,
            QaLayerSessionId(2)
        );

        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        app.qa.view = QaView::LayerQa;
        let _ = update(
            &mut app,
            Action::Qa(QaAction::LayerCapabilityLoaded(layer_capability())),
        );
        let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedLayerCheck));
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::Qa(QaDialog::LayerOperation(_)))
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::Qa(QaAction::CancelDialog));
        assert!(app.active_dialog().is_none());
        assert_eq!(app.focus, FocusTarget::Inspector);
    }

    #[test]
    fn qa_check_workflow_app_boundary_traps_dialog_focus_and_maps_typed_effects() {
        let mut app = App::new(10, 1_000);
        app.focus = FocusTarget::Inspector;
        assert_eq!(
            update(&mut app, Action::Qa(QaAction::InspectCapability)),
            Some(Effect::Qa(QaEffect::InspectCapability { scope: None }))
        );
        let _ = update(
            &mut app,
            Action::Qa(QaAction::CapabilityLoaded(capability())),
        );
        let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedCheck));
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::Qa(QaDialog::Operation(_)))
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);
        let _ = update(&mut app, Action::Qa(QaAction::CancelDialog));
        assert!(app.active_dialog().is_none());
        assert_eq!(app.focus, FocusTarget::Inspector);

        let _ = update(&mut app, Action::Qa(QaAction::BeginImport));
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::Qa(QaDialog::Import { editor, .. }))
                if editor.selected_text() == Some("")
        ));
        assert_eq!(app.focus, FocusTarget::Dialog);

        let transition = update_qa(
            &mut app.qa,
            QaAction::ConfirmImport("root = \"relative/report.json\"\n".into()),
        );
        assert!(matches!(
            transition.dialog,
            QaDialogUpdate::Open(dialog)
                if matches!(*dialog, QaDialog::Import {
                    validation_error: Some(ref message),
                    ..
                } if message.contains("absolute"))
        ));
        assert!(transition.effect.is_none());
    }
}
