use crate::{ImageArtifactIdentity, PackageIdentity};
use std::{
    cmp::Reverse,
    collections::{BTreeMap, BTreeSet},
    path::{Component, Path, PathBuf},
};

pub const MAX_ROOTFS_PACKAGES: usize = 8_192;
pub const MAX_ROOTFS_ENTRIES: usize = 65_536;
pub const MAX_ROOTFS_DEPTH: usize = 64;
pub const MAX_ROOTFS_LIMITATIONS: usize = 64;
pub const MAX_ROOTFS_TEXT_BYTES: usize = 512;
pub const MAX_ROOTFS_PATH_BYTES: usize = 4_096;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootfsCompositionRequest {
    pub generation: u64,
    pub image: ImageArtifactIdentity,
}

impl RootfsCompositionRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.generation == 0 {
            return Err("rootfs composition generations must be non-zero");
        }
        self.image.validate()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsInstalledPackage {
    pub identity: PackageIdentity,
    pub recipe: Option<String>,
    pub category: String,
    pub installed_size_bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RootfsEntryKind {
    Directory,
    RegularFile,
    Symlink,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RootfsPathIdentity(pub PathBuf);

impl RootfsPathIdentity {
    pub fn validate(&self) -> Result<(), &'static str> {
        if rootfs_path_is_valid(&self.0) {
            Ok(())
        } else {
            Err("rootfs entry paths must be normalized absolute logical paths")
        }
    }

    pub fn depth(&self) -> usize {
        self.0
            .components()
            .filter(|part| !matches!(part, Component::RootDir))
            .count()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootfsEntry {
    pub identity: RootfsPathIdentity,
    pub kind: RootfsEntryKind,
    pub size_bytes: u64,
    pub package: Option<PackageIdentity>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsPackageInventory {
    pub packages: Vec<RootfsInstalledPackage>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsFilesystemTree {
    pub entries: Vec<RootfsEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootfsAuthority<T> {
    Available(T),
    Partial { value: T, limitations: Vec<String> },
    Unavailable { reason: String },
}

impl<T> RootfsAuthority<T> {
    pub fn value(&self) -> Option<&T> {
        match self {
            Self::Available(value) | Self::Partial { value, .. } => Some(value),
            Self::Unavailable { .. } => None,
        }
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, Self::Partial { .. })
    }

    pub fn is_unavailable(&self) -> bool {
        matches!(self, Self::Unavailable { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsComposition {
    pub image: ImageArtifactIdentity,
    pub installed_packages: RootfsAuthority<RootfsPackageInventory>,
    pub filesystem_tree: RootfsAuthority<RootfsFilesystemTree>,
}

impl RootfsComposition {
    pub fn package_inventory(&self) -> Option<&RootfsPackageInventory> {
        self.installed_packages.value()
    }

    pub fn filesystem_tree(&self) -> Option<&RootfsFilesystemTree> {
        self.filesystem_tree.value()
    }

    pub fn is_empty(&self) -> bool {
        self.package_inventory()
            .is_none_or(|inventory| inventory.packages.is_empty())
            && self
                .filesystem_tree()
                .is_none_or(|tree| tree.entries.is_empty())
    }

    pub fn is_unavailable(&self) -> bool {
        self.installed_packages.is_unavailable() && self.filesystem_tree.is_unavailable()
    }

    pub fn is_partial(&self) -> bool {
        self.installed_packages.is_partial()
            || self.filesystem_tree.is_partial()
            || self.installed_packages.is_unavailable() != self.filesystem_tree.is_unavailable()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RootfsNormalizationReport {
    pub duplicate_packages: usize,
    pub invalid_packages: usize,
    pub truncated_packages: usize,
    pub duplicate_entries: usize,
    pub invalid_entries: usize,
    pub orphan_entries: usize,
    pub truncated_entries: usize,
    pub truncated_depth: usize,
    pub invalid_limitations: usize,
    pub truncated_limitations: usize,
    pub arithmetic_overflow: usize,
}

impl RootfsNormalizationReport {
    pub fn is_partial(&self) -> bool {
        self != &Self::default()
    }
}

pub fn normalize_rootfs_composition(
    request: &RootfsCompositionRequest,
    mut composition: RootfsComposition,
) -> (Option<RootfsComposition>, RootfsNormalizationReport) {
    let mut report = RootfsNormalizationReport::default();
    if request.validate().is_err()
        || composition.image.validate().is_err()
        || composition.image != request.image
    {
        report.invalid_entries = 1;
        return (None, report);
    }
    normalize_package_authority(&mut composition.installed_packages, &mut report);
    normalize_filesystem_authority(&mut composition.filesystem_tree, &mut report);
    (Some(composition), report)
}

fn normalize_package_authority(
    authority: &mut RootfsAuthority<RootfsPackageInventory>,
    report: &mut RootfsNormalizationReport,
) {
    match authority {
        RootfsAuthority::Available(inventory) => normalize_packages(inventory, report),
        RootfsAuthority::Partial { value, limitations } => {
            normalize_packages(value, report);
            normalize_limitations(limitations, report);
        }
        RootfsAuthority::Unavailable { reason } => normalize_reason(reason, report),
    }
}

fn normalize_packages(
    inventory: &mut RootfsPackageInventory,
    report: &mut RootfsNormalizationReport,
) {
    inventory.packages.sort();
    let mut normalized = BTreeMap::<PackageIdentity, RootfsInstalledPackage>::new();
    for package in inventory.packages.drain(..) {
        if package.identity.validate().is_err()
            || package.category.is_empty()
            || !rootfs_text_is_valid(&package.category)
            || package
                .recipe
                .as_deref()
                .is_some_and(|value| !rootfs_text_is_valid(value))
        {
            report.invalid_packages += 1;
            continue;
        }
        if let Some(existing) = normalized.get(&package.identity) {
            report.duplicate_packages += 1;
            if &package < existing {
                normalized.insert(package.identity.clone(), package);
            }
        } else {
            if normalized.len() == MAX_ROOTFS_PACKAGES {
                report.truncated_packages += 1;
            } else {
                normalized.insert(package.identity.clone(), package);
            }
        }
    }
    inventory.packages = normalized.into_values().collect();
}

fn normalize_filesystem_authority(
    authority: &mut RootfsAuthority<RootfsFilesystemTree>,
    report: &mut RootfsNormalizationReport,
) {
    match authority {
        RootfsAuthority::Available(tree) => normalize_entries(tree, report),
        RootfsAuthority::Partial { value, limitations } => {
            normalize_entries(value, report);
            normalize_limitations(limitations, report);
        }
        RootfsAuthority::Unavailable { reason } => normalize_reason(reason, report),
    }
}

fn normalize_entries(tree: &mut RootfsFilesystemTree, report: &mut RootfsNormalizationReport) {
    tree.entries.sort();
    let mut normalized = BTreeMap::<RootfsPathIdentity, RootfsEntry>::new();
    for entry in tree.entries.drain(..) {
        if entry.identity.validate().is_err()
            || entry
                .package
                .as_ref()
                .is_some_and(|package| package.validate().is_err())
        {
            report.invalid_entries += 1;
            continue;
        }
        if entry.identity.depth() > MAX_ROOTFS_DEPTH {
            report.truncated_depth += 1;
            continue;
        }
        if let Some(existing) = normalized.get(&entry.identity) {
            report.duplicate_entries += 1;
            if &entry < existing {
                normalized.insert(entry.identity.clone(), entry);
            }
        } else {
            if normalized.len() == MAX_ROOTFS_ENTRIES {
                report.truncated_entries += 1;
            } else {
                normalized.insert(entry.identity.clone(), entry);
            }
        }
    }
    let identities = normalized.keys().cloned().collect::<BTreeSet<_>>();
    report.orphan_entries += normalized
        .keys()
        .filter(|identity| {
            identity.0 != Path::new("/")
                && identity
                    .0
                    .parent()
                    .map(|parent| !identities.contains(&RootfsPathIdentity(parent.into())))
                    .unwrap_or(true)
        })
        .count();
    tree.entries = normalized.into_values().collect();
}

fn normalize_limitations(values: &mut Vec<String>, report: &mut RootfsNormalizationReport) {
    let before = values.len();
    values.retain(|value| !value.is_empty() && rootfs_text_is_valid(value));
    report.invalid_limitations += before - values.len();
    values.sort();
    values.dedup();
    if values.len() > MAX_ROOTFS_LIMITATIONS {
        report.truncated_limitations += values.len() - MAX_ROOTFS_LIMITATIONS;
        values.truncate(MAX_ROOTFS_LIMITATIONS);
    }
}

fn normalize_reason(reason: &mut String, report: &mut RootfsNormalizationReport) {
    if reason.is_empty() || !rootfs_text_is_valid(reason) {
        *reason = "authority unavailable without a valid reason".into();
        report.invalid_limitations += 1;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RootfsTotals {
    pub installed_package_bytes: u64,
    pub package_reported_files: u64,
    pub filesystem_bytes: u64,
    pub packages: u64,
    pub entries: u64,
    pub files: u64,
    pub directories: u64,
    pub symlinks: u64,
    pub other: u64,
}

impl RootfsComposition {
    pub fn totals(&self) -> (RootfsTotals, bool) {
        let mut totals = RootfsTotals::default();
        let mut overflowed = false;
        if let Some(inventory) = self.package_inventory() {
            totals.packages = inventory.packages.len() as u64;
            for package in &inventory.packages {
                totals.installed_package_bytes = checked_sum(
                    totals.installed_package_bytes,
                    package.installed_size_bytes,
                    &mut overflowed,
                );
                totals.package_reported_files = checked_sum(
                    totals.package_reported_files,
                    package.file_count,
                    &mut overflowed,
                );
            }
        }
        if let Some(tree) = self.filesystem_tree() {
            totals.entries = tree.entries.len() as u64;
            for entry in &tree.entries {
                totals.filesystem_bytes =
                    checked_sum(totals.filesystem_bytes, entry.size_bytes, &mut overflowed);
                match entry.kind {
                    RootfsEntryKind::RegularFile => totals.files += 1,
                    RootfsEntryKind::Directory => totals.directories += 1,
                    RootfsEntryKind::Symlink => totals.symlinks += 1,
                    RootfsEntryKind::Other => totals.other += 1,
                }
            }
        }
        (totals, overflowed)
    }
}

fn checked_sum(current: u64, value: u64, overflowed: &mut bool) -> u64 {
    current.checked_add(value).unwrap_or_else(|| {
        *overflowed = true;
        u64::MAX
    })
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RootfsGroupIdentity {
    Category(String),
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsGroupRow {
    pub identity: RootfsGroupIdentity,
    pub installed_size_bytes: u64,
    pub package_count: u64,
    pub percent_basis_points: u16,
    pub members: Vec<PackageIdentity>,
}

impl RootfsPackageInventory {
    pub fn grouped(&self, max_groups: usize) -> Vec<RootfsGroupRow> {
        let mut grouped = BTreeMap::<String, (u64, Vec<PackageIdentity>)>::new();
        let mut total = 0_u64;
        for package in &self.packages {
            total = total.saturating_add(package.installed_size_bytes);
            let group = grouped.entry(package.category.clone()).or_default();
            group.0 = group.0.saturating_add(package.installed_size_bytes);
            group.1.push(package.identity.clone());
        }
        let mut rows = grouped
            .into_iter()
            .map(|(category, (bytes, mut members))| {
                members.sort();
                RootfsGroupRow {
                    identity: RootfsGroupIdentity::Category(category),
                    installed_size_bytes: bytes,
                    package_count: members.len() as u64,
                    percent_basis_points: percentage_basis_points(bytes, total),
                    members,
                }
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| (Reverse(row.installed_size_bytes), row.identity.clone()));
        let limit = max_groups.max(1);
        if rows.len() > limit {
            let retained = limit.saturating_sub(1);
            let remainder = rows.split_off(retained);
            let mut members = remainder
                .iter()
                .flat_map(|row| row.members.iter().cloned())
                .collect::<Vec<_>>();
            members.sort();
            let bytes = remainder.iter().fold(0_u64, |sum, row| {
                sum.saturating_add(row.installed_size_bytes)
            });
            rows.push(RootfsGroupRow {
                identity: RootfsGroupIdentity::Other,
                installed_size_bytes: bytes,
                package_count: members.len() as u64,
                percent_basis_points: percentage_basis_points(bytes, total),
                members,
            });
        }
        rows
    }
}

impl RootfsFilesystemTree {
    pub fn children(&self, parent: &RootfsPathIdentity) -> Vec<&RootfsEntry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry.identity.0 != parent.0
                    && entry.identity.0.parent() == Some(parent.0.as_path())
            })
            .collect()
    }

    pub fn descendants(
        &self,
        parent: &RootfsPathIdentity,
        max_rows: usize,
    ) -> (Vec<&RootfsEntry>, usize) {
        let mut values = self
            .entries
            .iter()
            .filter(|entry| entry.identity.0 != parent.0 && entry.identity.0.starts_with(&parent.0))
            .collect::<Vec<_>>();
        let limit = max_rows.max(1);
        let truncated = values.len().saturating_sub(limit);
        values.truncate(limit);
        (values, truncated)
    }
}

fn percentage_basis_points(value: u64, total: u64) -> u16 {
    if total == 0 {
        0
    } else {
        ((u128::from(value) * 10_000) / u128::from(total)) as u16
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RootfsCompositionState {
    #[default]
    NotLoaded,
    Loading {
        request: RootfsCompositionRequest,
    },
    AvailableEmpty {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
    },
    Available {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
    },
    Partial {
        request: RootfsCompositionRequest,
        composition: RootfsComposition,
        limitations: Vec<String>,
    },
    Unavailable {
        request: RootfsCompositionRequest,
        reason: String,
    },
    Failed {
        request: RootfsCompositionRequest,
        message: String,
    },
}

impl RootfsCompositionState {
    pub fn request(&self) -> Option<&RootfsCompositionRequest> {
        match self {
            Self::NotLoaded => None,
            Self::Loading { request }
            | Self::AvailableEmpty { request, .. }
            | Self::Available { request, .. }
            | Self::Partial { request, .. }
            | Self::Unavailable { request, .. }
            | Self::Failed { request, .. } => Some(request),
        }
    }

    pub fn composition(&self) -> Option<&RootfsComposition> {
        match self {
            Self::AvailableEmpty { composition, .. }
            | Self::Available { composition, .. }
            | Self::Partial { composition, .. } => Some(composition),
            Self::NotLoaded
            | Self::Loading { .. }
            | Self::Unavailable { .. }
            | Self::Failed { .. } => None,
        }
    }
}

fn rootfs_text_is_valid(value: &str) -> bool {
    value.len() <= MAX_ROOTFS_TEXT_BYTES && !value.chars().any(char::is_control)
}

fn rootfs_path_is_valid(path: &Path) -> bool {
    path.is_absolute()
        && path.as_os_str().as_encoded_bytes().len() <= MAX_ROOTFS_PATH_BYTES
        && !path.components().any(|component| {
            matches!(
                component,
                Component::CurDir | Component::ParentDir | Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> ImageArtifactIdentity {
        ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4".into(),
        }
    }

    fn package(name: &str, category: &str, bytes: u64) -> RootfsInstalledPackage {
        RootfsInstalledPackage {
            identity: PackageIdentity::new(name),
            recipe: Some(name.into()),
            category: category.into(),
            installed_size_bytes: bytes,
            file_count: 1,
        }
    }

    #[test]
    fn ux_rootfs_normalizes_separate_authorities_bounds_and_correlates_image() {
        let request = RootfsCompositionRequest {
            generation: 7,
            image: image(),
        };
        let mut deep = PathBuf::from("/");
        for _ in 0..=MAX_ROOTFS_DEPTH {
            deep.push("d");
        }
        let composition = RootfsComposition {
            image: image(),
            installed_packages: RootfsAuthority::Partial {
                value: RootfsPackageInventory {
                    packages: vec![
                        package("busybox", "base", 10),
                        package("busybox", "base", 20),
                        package("bad name", "base", 1),
                    ],
                },
                limitations: vec!["manifest omitted versions".into()],
            },
            filesystem_tree: RootfsAuthority::Available(RootfsFilesystemTree {
                entries: vec![
                    RootfsEntry {
                        identity: RootfsPathIdentity("/".into()),
                        kind: RootfsEntryKind::Directory,
                        size_bytes: 0,
                        package: None,
                    },
                    RootfsEntry {
                        identity: RootfsPathIdentity("/usr/bin/busybox".into()),
                        kind: RootfsEntryKind::RegularFile,
                        size_bytes: 10,
                        package: Some(PackageIdentity::new("busybox")),
                    },
                    RootfsEntry {
                        identity: RootfsPathIdentity(deep),
                        kind: RootfsEntryKind::Directory,
                        size_bytes: 0,
                        package: None,
                    },
                ],
            }),
        };
        let (normalized, report) = normalize_rootfs_composition(&request, composition);
        let normalized = normalized.unwrap();
        assert_eq!(normalized.package_inventory().unwrap().packages.len(), 1);
        assert_eq!(normalized.filesystem_tree().unwrap().entries.len(), 2);
        assert_eq!(report.duplicate_packages, 1);
        assert_eq!(report.invalid_packages, 1);
        assert_eq!(report.truncated_depth, 1);
        assert_eq!(report.orphan_entries, 1);
        assert!(report.is_partial());
        let tree = normalized.filesystem_tree().unwrap();
        assert!(tree.children(&RootfsPathIdentity("/".into())).is_empty());
        let (descendants, truncated) = tree.descendants(&RootfsPathIdentity("/".into()), 1);
        assert_eq!(descendants.len(), 1);
        assert_eq!(truncated, 0);

        let wrong_request = RootfsCompositionRequest {
            generation: 8,
            image: ImageArtifactIdentity {
                image: "other".into(),
                ..image()
            },
        };
        assert!(
            normalize_rootfs_composition(&wrong_request, normalized)
                .0
                .is_none()
        );
    }

    #[test]
    fn ux_rootfs_groups_other_with_exact_totals_percentages_and_members() {
        let inventory = RootfsPackageInventory {
            packages: vec![
                package("a", "base", 50),
                package("b", "base", 10),
                package("c", "locale", 30),
                package("d", "debug", 10),
            ],
        };
        let rows = inventory.grouped(2);
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].identity,
            RootfsGroupIdentity::Category("base".into())
        );
        assert_eq!(rows[0].installed_size_bytes, 60);
        assert_eq!(rows[0].percent_basis_points, 6_000);
        assert_eq!(rows[1].identity, RootfsGroupIdentity::Other);
        assert_eq!(rows[1].installed_size_bytes, 40);
        assert_eq!(rows[1].package_count, 2);
        assert_eq!(rows[1].percent_basis_points, 4_000);
        assert_eq!(
            rows[1].members,
            [PackageIdentity::new("c"), PackageIdentity::new("d")]
        );
    }

    #[test]
    fn ux_rootfs_totals_are_overflow_safe_and_lifecycle_states_are_explicit() {
        let composition = RootfsComposition {
            image: image(),
            installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
                packages: vec![package("a", "base", u64::MAX), package("b", "base", 1)],
            }),
            filesystem_tree: RootfsAuthority::Unavailable {
                reason: "IMAGE_ROOTFS not reported".into(),
            },
        };
        let (totals, overflowed) = composition.totals();
        assert_eq!(totals.installed_package_bytes, u64::MAX);
        assert_eq!(totals.packages, 2);
        assert_eq!(totals.package_reported_files, 2);
        assert!(overflowed);
        assert!(composition.is_partial());
        assert!(!composition.is_unavailable());

        let request = RootfsCompositionRequest {
            generation: 1,
            image: image(),
        };
        for state in [
            RootfsCompositionState::Loading {
                request: request.clone(),
            },
            RootfsCompositionState::Unavailable {
                request: request.clone(),
                reason: "manifest unavailable".into(),
            },
            RootfsCompositionState::Failed {
                request: request.clone(),
                message: "adapter failed".into(),
            },
        ] {
            assert_eq!(state.request(), Some(&request));
            assert!(state.composition().is_none());
        }
    }
}
