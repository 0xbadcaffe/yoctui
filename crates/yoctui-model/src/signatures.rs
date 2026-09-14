//! Signatures.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureTarget {
    pub recipe: String,
    pub task: String,
}
impl SignatureTarget {
    pub fn validate(&self) -> Result<(), &'static str> {
        if signature_component_is_valid(&self.recipe) && signature_component_is_valid(&self.task) {
            Ok(())
        } else {
            Err("signature recipe and task must be non-empty tokens without whitespace or controls")
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureIdentity {
    pub target: SignatureTarget,
    pub hash: Option<String>,
    pub path: Option<PathBuf>,
}
impl SignatureIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.target.validate()?;
        if self.hash.as_ref().is_some_and(|hash| {
            hash.is_empty()
                || hash.len() > 256
                || hash.chars().any(char::is_whitespace)
                || hash.chars().any(char::is_control)
        }) {
            return Err("signature hashes must be bounded tokens");
        }
        if self.path.as_ref().is_some_and(|path| !path.is_absolute()) {
            return Err("signature paths must be absolute");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureValue {
    pub name: String,
    pub value: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureRecord {
    pub identity: SignatureIdentity,
    pub base_hash: Option<String>,
    pub task_hash: Option<String>,
    pub variables: Vec<SignatureValue>,
    pub dependencies: Vec<String>,
}
impl SignatureRecord {
    pub(crate) fn normalize(mut self) -> Self {
        self.variables.sort();
        self.variables
            .dedup_by(|left, right| left.name == right.name);
        self.dependencies.sort();
        self.dependencies.dedup();
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignatureNormalizationReport {
    pub duplicate_records: usize,
    pub invalid_records: usize,
    pub truncated_records: usize,
}
impl SignatureNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self.invalid_records > 0 || self.truncated_records > 0
    }
}
pub fn normalize_signature_records(
    target: &SignatureTarget,
    records: Vec<SignatureRecord>,
    max_records: usize,
) -> (Vec<SignatureRecord>, SignatureNormalizationReport) {
    let mut report = SignatureNormalizationReport::default();
    let mut normalized = BTreeMap::new();
    for record in records {
        if record.identity.target != *target || record.identity.validate().is_err() {
            report.invalid_records += 1;
            continue;
        }
        let record = record.normalize();
        if let Some(existing) = normalized.get(&record.identity) {
            report.duplicate_records += 1;
            if &record < existing {
                normalized.insert(record.identity.clone(), record);
            }
        } else {
            normalized.insert(record.identity.clone(), record);
        }
    }
    let mut records = normalized.into_values().collect::<Vec<_>>();
    if records.len() > max_records {
        report.truncated_records = records.len() - max_records;
        records.truncate(max_records);
    }
    (records, report)
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SignatureDumpState {
    #[default]
    NotLoaded,
    Loading {
        target: SignatureTarget,
    },
    AvailableEmpty {
        target: SignatureTarget,
    },
    Available {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
    },
    Partial {
        target: SignatureTarget,
        records: Vec<SignatureRecord>,
        limitations: Vec<String>,
    },
    Failed {
        target: SignatureTarget,
        message: String,
    },
}
impl SignatureDumpState {
    pub fn records(&self) -> Option<&[SignatureRecord]> {
        match self {
            Self::Available { records, .. } | Self::Partial { records, .. } => Some(records),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::AvailableEmpty { .. }
            | Self::Failed { .. } => None,
        }
    }

    pub fn target(&self) -> Option<&SignatureTarget> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { target }
            | Self::AvailableEmpty { target }
            | Self::Available { target, .. }
            | Self::Partial { target, .. }
            | Self::Failed { target, .. } => Some(target),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureComparisonSide {
    Left,
    Right,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureComparisonRequest {
    pub left: SignatureIdentity,
    pub right: SignatureIdentity,
}
impl SignatureComparisonRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.left.validate()?;
        self.right.validate()?;
        if self.left == self.right {
            return Err("signature comparison requires two distinct identities");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignatureDifferenceCategory {
    BaseHash,
    ChangedValue,
    Dependency,
    Unavailable,
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignatureDifference {
    pub category: SignatureDifferenceCategory,
    pub key: String,
    pub left: Option<String>,
    pub right: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SignatureDifferenceReport {
    pub duplicate_differences: usize,
    pub truncated_differences: usize,
}
impl SignatureDifferenceReport {
    pub fn is_partial(&self) -> bool {
        self.truncated_differences > 0
    }
}
pub fn normalize_signature_differences(
    mut differences: Vec<SignatureDifference>,
    max_differences: usize,
) -> (Vec<SignatureDifference>, SignatureDifferenceReport) {
    differences.sort();
    let before = differences.len();
    differences.dedup();
    let mut report = SignatureDifferenceReport {
        duplicate_differences: before - differences.len(),
        ..SignatureDifferenceReport::default()
    };
    if differences.len() > max_differences {
        report.truncated_differences = differences.len() - max_differences;
        differences.truncate(max_differences);
    }
    (differences, report)
}
pub fn compare_signature_records(
    left: &SignatureRecord,
    right: &SignatureRecord,
    max_differences: usize,
) -> (Vec<SignatureDifference>, SignatureDifferenceReport) {
    let mut differences = Vec::new();
    signature_hash_difference(
        &mut differences,
        "base_hash",
        left.base_hash.as_ref(),
        right.base_hash.as_ref(),
    );
    signature_hash_difference(
        &mut differences,
        "task_hash",
        left.task_hash.as_ref(),
        right.task_hash.as_ref(),
    );

    let left_values = left
        .variables
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    let right_values = right
        .variables
        .iter()
        .map(|value| (value.name.as_str(), value))
        .collect::<BTreeMap<_, _>>();
    for name in left_values
        .keys()
        .chain(right_values.keys())
        .copied()
        .collect::<BTreeSet<_>>()
    {
        let left = left_values.get(name).copied();
        let right = right_values.get(name).copied();
        if left.map(|value| &value.value) != right.map(|value| &value.value) {
            differences.push(SignatureDifference {
                category: if matches!((left, right), (Some(left), Some(right))
                    if left.value.is_some() && right.value.is_some())
                {
                    SignatureDifferenceCategory::ChangedValue
                } else {
                    SignatureDifferenceCategory::Unavailable
                },
                key: name.to_owned(),
                left: left.and_then(|value| value.value.clone()),
                right: right.and_then(|value| value.value.clone()),
            });
        }
    }

    let left_dependencies = left
        .dependencies
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let right_dependencies = right
        .dependencies
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for dependency in left_dependencies.symmetric_difference(&right_dependencies) {
        differences.push(SignatureDifference {
            category: SignatureDifferenceCategory::Dependency,
            key: (*dependency).to_owned(),
            left: left_dependencies
                .contains(dependency)
                .then(|| "present".into()),
            right: right_dependencies
                .contains(dependency)
                .then(|| "present".into()),
        });
    }
    normalize_signature_differences(differences, max_differences)
}
pub(crate) fn signature_hash_difference(
    differences: &mut Vec<SignatureDifference>,
    key: &str,
    left: Option<&String>,
    right: Option<&String>,
) {
    if left != right {
        differences.push(SignatureDifference {
            category: if left.is_some() && right.is_some() {
                SignatureDifferenceCategory::BaseHash
            } else {
                SignatureDifferenceCategory::Unavailable
            },
            key: key.into(),
            left: left.cloned(),
            right: right.cloned(),
        });
    }
}
pub(crate) fn signature_component_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SignatureComparisonState {
    #[default]
    NotSelected,
    Ready {
        left: Option<SignatureIdentity>,
        right: Option<SignatureIdentity>,
    },
    Loading {
        request: SignatureComparisonRequest,
    },
    AvailableEmpty {
        request: SignatureComparisonRequest,
    },
    Available {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
    },
    Partial {
        request: SignatureComparisonRequest,
        differences: Vec<SignatureDifference>,
        limitations: Vec<String>,
    },
    Failed {
        request: SignatureComparisonRequest,
        message: String,
    },
}
