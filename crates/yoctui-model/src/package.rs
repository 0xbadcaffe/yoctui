use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub const MAX_PACKAGE_RECORDS: usize = 8_192;
pub const MAX_PACKAGE_FILES: usize = 4_096;
pub const MAX_PACKAGE_DEPENDENCIES: usize = 2_048;
pub const MAX_PACKAGE_IMAGE_MEMBERSHIPS: usize = 256;
pub const MAX_PACKAGE_LIMITATIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageIdentity {
    pub name: String,
}

impl PackageIdentity {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if package_token_is_valid(&self.name) {
            Ok(())
        } else {
            Err("package names must be bounded non-empty tokens without whitespace or controls")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum PackageField<T> {
    #[default]
    Unavailable,
    Available(T),
}

impl<T> PackageField<T> {
    pub fn as_ref(&self) -> PackageField<&T> {
        match self {
            Self::Unavailable => PackageField::Unavailable,
            Self::Available(value) => PackageField::Available(value),
        }
    }

    pub fn available(&self) -> Option<&T> {
        match self {
            Self::Unavailable => None,
            Self::Available(value) => Some(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageSummary {
    pub identity: PackageIdentity,
    pub recipe: PackageField<String>,
    pub provider: PackageField<PathBuf>,
    pub version: PackageField<String>,
    pub installed_size_bytes: PackageField<u64>,
    pub license: PackageField<String>,
    pub image_membership: PackageField<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PackageDetail {
    pub identity: PackageIdentity,
    pub files: PackageField<Vec<PathBuf>>,
    pub runtime_dependencies: PackageField<Vec<PackageIdentity>>,
    pub reverse_dependencies: PackageField<Vec<PackageIdentity>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageInventoryRequest {
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageDetailRequest {
    pub identity: PackageIdentity,
    pub generation: u64,
}

impl PackageDetailRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.identity.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PackageNormalizationReport {
    pub duplicate_records: usize,
    pub invalid_records: usize,
    pub invalid_fields: usize,
    pub truncated_records: usize,
    pub truncated_files: usize,
    pub truncated_dependencies: usize,
    pub truncated_image_memberships: usize,
}

impl PackageNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self.invalid_records > 0
            || self.invalid_fields > 0
            || self.truncated_records > 0
            || self.truncated_files > 0
            || self.truncated_dependencies > 0
            || self.truncated_image_memberships > 0
    }
}

pub fn normalize_package_summaries(
    summaries: Vec<PackageSummary>,
    max_records: usize,
) -> (Vec<PackageSummary>, PackageNormalizationReport) {
    let mut report = PackageNormalizationReport::default();
    let mut normalized = BTreeMap::new();
    for mut summary in summaries {
        if summary.identity.validate().is_err() {
            report.invalid_records += 1;
            continue;
        }
        normalize_text_field(&mut summary.recipe, &mut report);
        normalize_path_field(&mut summary.provider, &mut report);
        normalize_text_field(&mut summary.version, &mut report);
        normalize_text_field(&mut summary.license, &mut report);
        normalize_memberships(&mut summary.image_membership, &mut report);
        if let Some(existing) = normalized.get(&summary.identity) {
            report.duplicate_records += 1;
            if &summary < existing {
                normalized.insert(summary.identity.clone(), summary);
            }
        } else {
            normalized.insert(summary.identity.clone(), summary);
        }
    }
    let mut summaries = normalized.into_values().collect::<Vec<_>>();
    if summaries.len() > max_records {
        report.truncated_records = summaries.len() - max_records;
        summaries.truncate(max_records);
    }
    (summaries, report)
}

pub fn normalize_package_detail(
    expected: &PackageIdentity,
    mut detail: PackageDetail,
) -> (Option<PackageDetail>, PackageNormalizationReport) {
    let mut report = PackageNormalizationReport::default();
    if detail.identity != *expected || detail.identity.validate().is_err() {
        report.invalid_records = 1;
        return (None, report);
    }
    normalize_files(&mut detail.files, &mut report);
    normalize_dependencies(&mut detail.runtime_dependencies, &mut report);
    normalize_dependencies(&mut detail.reverse_dependencies, &mut report);
    (Some(detail), report)
}

fn normalize_text_field(field: &mut PackageField<String>, report: &mut PackageNormalizationReport) {
    if matches!(field, PackageField::Available(value) if !package_text_is_valid(value)) {
        *field = PackageField::Unavailable;
        report.invalid_fields += 1;
    }
}

fn normalize_path_field(
    field: &mut PackageField<PathBuf>,
    report: &mut PackageNormalizationReport,
) {
    if matches!(field, PackageField::Available(path) if !path.is_absolute()) {
        *field = PackageField::Unavailable;
        report.invalid_fields += 1;
    }
}

fn normalize_memberships(
    field: &mut PackageField<Vec<String>>,
    report: &mut PackageNormalizationReport,
) {
    let PackageField::Available(values) = field else {
        return;
    };
    let before = values.len();
    values.retain(|value| package_token_is_valid(value));
    report.invalid_fields += before - values.len();
    values.sort();
    values.dedup();
    if values.len() > MAX_PACKAGE_IMAGE_MEMBERSHIPS {
        report.truncated_image_memberships += values.len() - MAX_PACKAGE_IMAGE_MEMBERSHIPS;
        values.truncate(MAX_PACKAGE_IMAGE_MEMBERSHIPS);
    }
}

fn normalize_files(
    field: &mut PackageField<Vec<PathBuf>>,
    report: &mut PackageNormalizationReport,
) {
    let PackageField::Available(paths) = field else {
        return;
    };
    let before = paths.len();
    paths.retain(|path| path.is_absolute() && path != Path::new("/"));
    report.invalid_fields += before - paths.len();
    paths.sort();
    paths.dedup();
    if paths.len() > MAX_PACKAGE_FILES {
        report.truncated_files += paths.len() - MAX_PACKAGE_FILES;
        paths.truncate(MAX_PACKAGE_FILES);
    }
}

fn normalize_dependencies(
    field: &mut PackageField<Vec<PackageIdentity>>,
    report: &mut PackageNormalizationReport,
) {
    let PackageField::Available(identities) = field else {
        return;
    };
    let before = identities.len();
    identities.retain(|identity| identity.validate().is_ok());
    report.invalid_fields += before - identities.len();
    identities.sort();
    identities.dedup();
    if identities.len() > MAX_PACKAGE_DEPENDENCIES {
        report.truncated_dependencies += identities.len() - MAX_PACKAGE_DEPENDENCIES;
        identities.truncate(MAX_PACKAGE_DEPENDENCIES);
    }
}

fn package_token_is_valid(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 512
        && !value.chars().any(char::is_whitespace)
        && !value.chars().any(char::is_control)
}

fn package_text_is_valid(value: &str) -> bool {
    !value.is_empty() && value.len() <= 64 * 1024 && !value.chars().any(char::is_control)
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PackageInventoryState {
    #[default]
    NotLoaded,
    Loading {
        request: PackageInventoryRequest,
    },
    AvailableEmpty {
        request: PackageInventoryRequest,
    },
    Available {
        request: PackageInventoryRequest,
        packages: Vec<PackageSummary>,
    },
    Partial {
        request: PackageInventoryRequest,
        packages: Vec<PackageSummary>,
        limitations: Vec<String>,
    },
    Failed {
        request: PackageInventoryRequest,
        message: String,
    },
}

impl PackageInventoryState {
    pub fn request(&self) -> Option<PackageInventoryRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(*request),
        }
    }

    pub fn packages(&self) -> Option<&[PackageSummary]> {
        match self {
            Self::Available { packages, .. } | Self::Partial { packages, .. } => Some(packages),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::AvailableEmpty { .. }
            | Self::Failed { .. } => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum PackageDetailState {
    #[default]
    NotLoaded,
    Loading {
        request: PackageDetailRequest,
    },
    AvailableEmpty {
        request: PackageDetailRequest,
    },
    Available {
        request: PackageDetailRequest,
        detail: PackageDetail,
    },
    Partial {
        request: PackageDetailRequest,
        detail: PackageDetail,
        limitations: Vec<String>,
    },
    Failed {
        request: PackageDetailRequest,
        message: String,
    },
}

impl PackageDetailState {
    pub fn request(&self) -> Option<&PackageDetailRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Failed { request, .. } => Some(request),
        }
    }

    pub fn detail(&self) -> Option<&PackageDetail> {
        match self {
            Self::Available { detail, .. } | Self::Partial { detail, .. } => Some(detail),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::AvailableEmpty { .. }
            | Self::Failed { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(name: &str) -> PackageSummary {
        PackageSummary {
            identity: PackageIdentity::new(name),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Available("/layers/meta/busybox.bb".into()),
            version: PackageField::Available("1.0".into()),
            installed_size_bytes: PackageField::Available(1024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Available(vec!["core-image-minimal".into()]),
        }
    }

    #[test]
    fn pkgdata_model_normalizes_inventory_duplicates_bounds_and_unavailable_fields() {
        let mut preferred = summary("busybox");
        preferred.recipe = PackageField::Available("aaa".into());
        preferred.image_membership = PackageField::Available(vec![
            "image-z".into(),
            "image-a".into(),
            "image-a".into(),
            "bad image".into(),
        ]);
        let mut duplicate = preferred.clone();
        duplicate.recipe = PackageField::Available("zzz".into());
        let mut invalid_field = summary("libc");
        invalid_field.provider = PackageField::Available("relative.bb".into());
        let invalid = summary("bad package");
        let overflow = summary("zlib");

        let (packages, report) = normalize_package_summaries(
            vec![duplicate, invalid, overflow, invalid_field, preferred],
            2,
        );
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].identity.name, "busybox");
        assert_eq!(packages[0].recipe, PackageField::Available("aaa".into()));
        assert_eq!(
            packages[0].image_membership,
            PackageField::Available(vec!["image-a".into(), "image-z".into()])
        );
        assert_eq!(packages[1].provider, PackageField::Unavailable);
        assert_eq!(report.duplicate_records, 1);
        assert_eq!(report.invalid_records, 1);
        assert_eq!(report.invalid_fields, 3);
        assert_eq!(report.truncated_records, 1);
        assert!(report.is_partial());
    }

    #[test]
    fn pkgdata_model_normalizes_detail_collections_and_rejects_wrong_identity() {
        let expected = PackageIdentity::new("busybox");
        let detail = PackageDetail {
            identity: expected.clone(),
            files: PackageField::Available(vec![
                "/usr/bin/busybox".into(),
                "relative".into(),
                "/usr/bin/busybox".into(),
            ]),
            runtime_dependencies: PackageField::Available(vec![
                PackageIdentity::new("libc"),
                PackageIdentity::new("bad dep"),
                PackageIdentity::new("libc"),
            ]),
            reverse_dependencies: PackageField::Available(Vec::new()),
        };
        let (detail, report) = normalize_package_detail(&expected, detail);
        let detail = detail.unwrap();
        assert_eq!(
            detail.files,
            PackageField::Available(vec![PathBuf::from("/usr/bin/busybox")])
        );
        assert_eq!(
            detail.runtime_dependencies,
            PackageField::Available(vec![PackageIdentity::new("libc")])
        );
        assert_eq!(report.invalid_fields, 2);

        let wrong = PackageDetail {
            identity: PackageIdentity::new("other"),
            files: PackageField::Unavailable,
            runtime_dependencies: PackageField::Unavailable,
            reverse_dependencies: PackageField::Unavailable,
        };
        let (detail, report) = normalize_package_detail(&expected, wrong);
        assert!(detail.is_none());
        assert_eq!(report.invalid_records, 1);
    }
}
