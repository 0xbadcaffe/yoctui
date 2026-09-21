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
