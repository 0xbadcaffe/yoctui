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
pub struct CycloneDxDocument {
    pub identity: SecurityReportIdentity,
    pub scope: Option<SecurityScope>,
    pub spec_version: Option<String>,
    pub serial_number: Option<String>,
    pub version: Option<u64>,
    pub components: Vec<SpdxComponent>,
    pub dependency_count: Option<u64>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManifestDocument {
    pub identity: SecurityReportIdentity,
    pub scope: Option<SecurityScope>,
    pub components: Vec<SpdxComponent>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityReport {
    Cve(CveReport),
    Spdx(SpdxDocument),
    CycloneDx(CycloneDxDocument),
    PackageManifest(PackageManifestDocument),
}

impl SecurityReport {
    pub fn identity(&self) -> &SecurityReportIdentity {
        match self {
            Self::Cve(report) => &report.identity,
            Self::Spdx(document) => &document.identity,
            Self::CycloneDx(document) => &document.identity,
            Self::PackageManifest(document) => &document.identity,
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
            SecurityReport::CycloneDx(document) => {
                normalize_sbom_components(&mut document.components, "CycloneDX", &mut limitations);
                document.limitations =
                    normalize_limitations(std::mem::take(&mut document.limitations));
            }
            SecurityReport::PackageManifest(document) => {
                normalize_sbom_components(&mut document.components, "manifest", &mut limitations);
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

fn normalize_sbom_components(
    components: &mut Vec<SpdxComponent>,
    format: &str,
    limitations: &mut Vec<String>,
) {
    components.retain(SpdxComponent::is_valid);
    components.sort();
    components.dedup();
    if components.len() > MAX_SECURITY_COMPONENTS {
        let dropped = components.len() - MAX_SECURITY_COMPONENTS;
        components.truncate(MAX_SECURITY_COMPONENTS);
        limitations.push(format!(
            "ignored {dropped} {format} components beyond the model bound"
        ));
    }
}
