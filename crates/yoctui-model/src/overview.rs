use crate::{
    App, ImageArtifactIdentity, PackageDetailState, PackageField, RootfsComposition,
    RootfsCompositionState, SecurityReport, SignatureComparisonState, SignatureDifferenceCategory,
    TaskInfo, TaskState,
};
use std::{collections::BTreeMap, time::SystemTime};

pub const MAX_OVERVIEW_ROWS: usize = 256;
pub const MAX_OVERVIEW_IMAGE_SNAPSHOTS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OverviewView {
    #[default]
    Timeline,
    RebuildCauses,
    CacheAndDownloads,
    ImageSize,
    MetadataProvenance,
    PackageTopology,
    SupplyChain,
    DiskUsage,
}

impl OverviewView {
    pub const ALL: [Self; 8] = [
        Self::Timeline,
        Self::RebuildCauses,
        Self::CacheAndDownloads,
        Self::ImageSize,
        Self::MetadataProvenance,
        Self::PackageTopology,
        Self::SupplyChain,
        Self::DiskUsage,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Timeline => "Timeline",
            Self::RebuildCauses => "Rebuild causes",
            Self::CacheAndDownloads => "Sstate & downloads",
            Self::ImageSize => "Image size",
            Self::MetadataProvenance => "Metadata provenance",
            Self::PackageTopology => "Package topology",
            Self::SupplyChain => "Supply chain",
            Self::DiskUsage => "Disk usage",
        }
    }

    pub fn shifted(self, delta: isize) -> Self {
        let current = Self::ALL.iter().position(|view| *view == self).unwrap_or(0);
        Self::ALL[(current as isize + delta).rem_euclid(Self::ALL.len() as isize) as usize]
    }

    pub fn from_number(number: u8) -> Option<Self> {
        Self::ALL.get(usize::from(number.checked_sub(1)?)).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewTimelineRow {
    pub id: String,
    pub label: String,
    pub state: TaskState,
    pub start_millis: u64,
    pub duration_millis: u64,
    pub critical: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverviewCacheProjection {
    pub sstate_hits: usize,
    pub sstate_misses: usize,
    pub sstate_active: usize,
    pub fetch_completed: usize,
    pub fetch_failed: usize,
    pub fetch_active: usize,
    pub sstate_dir: Option<String>,
    pub downloads_dir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewSizedRow {
    pub label: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewImageSizeSnapshot {
    pub image: ImageArtifactIdentity,
    pub installed_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverviewImageSizeDelta {
    pub current_bytes: u64,
    pub previous_bytes: u64,
    pub delta_bytes: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewEdgeRow {
    pub source: String,
    pub relation: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OverviewSupplyChainProjection {
    pub cve_reports: usize,
    pub vulnerable: usize,
    pub spdx_documents: usize,
    pub cyclonedx_documents: usize,
    pub manifest_documents: usize,
    pub components: usize,
    pub limitations: Vec<String>,
}

impl App {
    pub(crate) fn record_overview_image_size(&mut self, composition: &RootfsComposition) {
        let Some(packages) = composition.package_inventory() else {
            return;
        };
        let installed_bytes = packages.packages.iter().fold(0_u64, |total, package| {
            total.saturating_add(package.installed_size_bytes)
        });
        let snapshot = OverviewImageSizeSnapshot {
            image: composition.image.clone(),
            installed_bytes,
        };
        if self.overview_image_size_history.back() == Some(&snapshot) {
            return;
        }
        self.overview_image_size_history.push_back(snapshot);
        while self.overview_image_size_history.len() > MAX_OVERVIEW_IMAGE_SNAPSHOTS {
            self.overview_image_size_history.pop_front();
        }
    }

    pub fn overview_timeline(&self, now: SystemTime) -> Vec<OverviewTimelineRow> {
        let mut tasks = self.tasks.values().collect::<Vec<_>>();
        tasks.sort_by(|left, right| left.id.0.cmp(&right.id.0));
        tasks.truncate(MAX_OVERVIEW_ROWS);
        let origin = tasks.iter().filter_map(|task| task.started).min();
        let by_id = tasks
            .iter()
            .map(|task| (task.id.0.clone(), *task))
            .collect::<BTreeMap<_, _>>();
        let mut longest = BTreeMap::<String, (u64, Option<String>)>::new();
        for task in &tasks {
            longest_path(
                &task.id.0,
                &by_id,
                now,
                &mut std::collections::BTreeSet::new(),
                &mut longest,
            );
        }
        let mut critical = std::collections::BTreeSet::new();
        let mut cursor = longest
            .iter()
            .max_by_key(|(id, value)| (value.0, std::cmp::Reverse(id.as_str())))
            .map(|(id, _)| id.clone());
        while let Some(id) = cursor {
            if !critical.insert(id.clone()) {
                break;
            }
            cursor = longest
                .get(&id)
                .and_then(|(_, predecessor)| predecessor.clone());
        }
        let mut rows = tasks
            .into_iter()
            .map(|task| OverviewTimelineRow {
                id: task.id.0.clone(),
                label: format!("{}:{}", task.recipe, task.task),
                state: task.state,
                start_millis: task
                    .started
                    .and_then(|started| {
                        origin.and_then(|origin| started.duration_since(origin).ok())
                    })
                    .map_or(0, |value| value.as_millis() as u64),
                duration_millis: task
                    .elapsed_at(now)
                    .map_or(0, |value| value.as_millis() as u64),
                critical: critical.contains(&task.id.0),
            })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| (row.start_millis, row.label.clone()));
        rows
    }

    pub fn overview_rebuild_causes(&self) -> Vec<OverviewEdgeRow> {
        let differences = match &self.signature_comparison {
            SignatureComparisonState::Available { differences, .. }
            | SignatureComparisonState::Partial { differences, .. } => differences,
            _ => return Vec::new(),
        };
        differences
            .iter()
            .take(MAX_OVERVIEW_ROWS)
            .map(|difference| OverviewEdgeRow {
                source: match difference.category {
                    SignatureDifferenceCategory::BaseHash => "signature".into(),
                    SignatureDifferenceCategory::ChangedValue => "variable".into(),
                    SignatureDifferenceCategory::Dependency => "dependency".into(),
                    SignatureDifferenceCategory::Unavailable => "unknown".into(),
                },
                relation: "changed".into(),
                target: difference.key.clone(),
            })
            .collect()
    }

    pub fn overview_cache(&self) -> OverviewCacheProjection {
        let mut projection = OverviewCacheProjection {
            sstate_dir: self.workspace.variables.get("SSTATE_DIR").cloned(),
            downloads_dir: self.workspace.variables.get("DL_DIR").cloned(),
            ..OverviewCacheProjection::default()
        };
        for task in self.tasks.values() {
            let is_sstate =
                task.task.contains("setscene") || task.task == "do_shared_workdir_setscene";
            let is_fetch = task.task == "do_fetch";
            if is_sstate {
                match task.state {
                    TaskState::Completed => projection.sstate_hits += 1,
                    TaskState::Failed | TaskState::Lost => projection.sstate_misses += 1,
                    TaskState::Active | TaskState::Queued | TaskState::Waiting => {
                        projection.sstate_active += 1
                    }
                    TaskState::Cancelled => {}
                }
            }
            if is_fetch {
                match task.state {
                    TaskState::Completed => projection.fetch_completed += 1,
                    TaskState::Failed | TaskState::Lost => projection.fetch_failed += 1,
                    TaskState::Active | TaskState::Queued | TaskState::Waiting => {
                        projection.fetch_active += 1
                    }
                    TaskState::Cancelled => {}
                }
            }
        }
        projection
    }

    pub fn overview_image_sizes(&self) -> Vec<OverviewSizedRow> {
        let composition = match &self.rootfs_composition {
            RootfsCompositionState::Available { composition, .. }
            | RootfsCompositionState::Partial { composition, .. } => composition,
            _ => return Vec::new(),
        };
        let Some(packages) = composition.package_inventory() else {
            return Vec::new();
        };
        let mut categories = BTreeMap::<String, u64>::new();
        for package in &packages.packages {
            *categories.entry(package.category.clone()).or_default() = categories
                .get(&package.category)
                .copied()
                .unwrap_or(0)
                .saturating_add(package.installed_size_bytes);
        }
        let mut rows = categories
            .into_iter()
            .map(|(label, bytes)| OverviewSizedRow { label, bytes })
            .collect::<Vec<_>>();
        rows.sort_by_key(|row| std::cmp::Reverse(row.bytes));
        rows.truncate(MAX_OVERVIEW_ROWS);
        rows
    }

    pub fn overview_image_size_delta(&self) -> Option<OverviewImageSizeDelta> {
        let composition = self.rootfs_composition.composition()?;
        let current = self.overview_image_size_history.back()?;
        if current.image != composition.image {
            return None;
        }
        let previous = self
            .overview_image_size_history
            .iter()
            .rev()
            .skip(1)
            .find(|snapshot| {
                snapshot.image.machine == current.image.machine
                    && snapshot.image.image == current.image.image
            })?;
        Some(OverviewImageSizeDelta {
            current_bytes: current.installed_bytes,
            previous_bytes: previous.installed_bytes,
            delta_bytes: i128::from(current.installed_bytes) - i128::from(previous.installed_bytes),
        })
    }

    pub fn overview_provenance(&self) -> Vec<OverviewEdgeRow> {
        let mut rows = Vec::new();
        for (variable, chain) in &self.workspace.variable_provenance_chain {
            let mut previous = variable.as_str();
            for source in chain {
                rows.push(OverviewEdgeRow {
                    source: previous.into(),
                    relation: "set by".into(),
                    target: source.clone(),
                });
                previous = source;
                if rows.len() == MAX_OVERVIEW_ROWS {
                    return rows;
                }
            }
        }
        rows
    }

    pub fn overview_package_topology(&self) -> Vec<OverviewEdgeRow> {
        let mut rows = Vec::new();
        for (identity, state) in &self.package_details {
            let (PackageDetailState::Available { detail, .. }
            | PackageDetailState::Partial { detail, .. }) = state
            else {
                continue;
            };
            if let PackageField::Available(dependencies) = &detail.runtime_dependencies {
                for dependency in dependencies {
                    rows.push(OverviewEdgeRow {
                        source: identity.name.clone(),
                        relation: "RDEPENDS".into(),
                        target: dependency.name.clone(),
                    });
                    if rows.len() == MAX_OVERVIEW_ROWS {
                        return rows;
                    }
                }
            }
        }
        rows.sort_by(|left, right| {
            (&left.source, &left.target).cmp(&(&right.source, &right.target))
        });
        rows
    }

    pub fn overview_supply_chain(&self) -> OverviewSupplyChainProjection {
        let mut projection = OverviewSupplyChainProjection::default();
        for report in self.security.inventory.reports().unwrap_or_default() {
            match report {
                SecurityReport::Cve(report) => {
                    projection.cve_reports += 1;
                    projection.vulnerable += report
                        .findings
                        .iter()
                        .filter(|finding| matches!(finding.status, crate::CveStatus::Vulnerable))
                        .count();
                }
                SecurityReport::Spdx(document) => {
                    projection.spdx_documents += 1;
                    projection.components += document.components.len();
                    projection
                        .limitations
                        .extend(document.limitations.iter().cloned());
                }
                SecurityReport::CycloneDx(document) => {
                    projection.cyclonedx_documents += 1;
                    projection.components += document.components.len();
                    projection
                        .limitations
                        .extend(document.limitations.iter().cloned());
                }
                SecurityReport::PackageManifest(document) => {
                    projection.manifest_documents += 1;
                    projection.components += document.components.len();
                    projection
                        .limitations
                        .extend(document.limitations.iter().cloned());
                }
            }
        }
        projection.limitations.sort();
        projection.limitations.dedup();
        projection.limitations.truncate(16);
        projection
    }
}

fn longest_path(
    id: &str,
    tasks: &BTreeMap<String, &TaskInfo>,
    now: SystemTime,
    visiting: &mut std::collections::BTreeSet<String>,
    memo: &mut BTreeMap<String, (u64, Option<String>)>,
) -> (u64, Option<String>) {
    if let Some(result) = memo.get(id) {
        return result.clone();
    }
    let Some(task) = tasks.get(id) else {
        return (0, None);
    };
    let own = task
        .elapsed_at(now)
        .map_or(0, |value| value.as_millis() as u64);
    if !visiting.insert(id.to_owned()) {
        return (own, None);
    }
    let mut predecessor: Option<(String, u64)> = None;
    for dependency in &task.dependencies {
        if !tasks.contains_key(&dependency.0) || visiting.contains(&dependency.0) {
            continue;
        }
        let candidate = (
            dependency.0.clone(),
            longest_path(&dependency.0, tasks, now, visiting, memo).0,
        );
        if predecessor.as_ref().is_none_or(|current| {
            candidate.1 > current.1 || (candidate.1 == current.1 && candidate.0 < current.0)
        }) {
            predecessor = Some(candidate);
        }
    }
    visiting.remove(id);
    let result = (
        own.saturating_add(predecessor.as_ref().map_or(0, |(_, duration)| *duration)),
        predecessor.map(|(id, _)| id),
    );
    memo.insert(id.to_owned(), result.clone());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        PackageIdentity, RootfsAuthority, RootfsCompositionRequest, RootfsInstalledPackage,
        RootfsPackageInventory, TaskId, Workspace,
    };
    use std::{
        path::PathBuf,
        time::{Duration, UNIX_EPOCH},
    };

    #[test]
    fn overview_views_cycle_and_select_by_number() {
        assert_eq!(OverviewView::Timeline.shifted(-1), OverviewView::DiskUsage);
        assert_eq!(
            OverviewView::from_number(7),
            Some(OverviewView::SupplyChain)
        );
        assert_eq!(OverviewView::from_number(9), None);
    }

    #[test]
    fn timeline_marks_a_deterministic_longest_dependency_path() {
        let mut app = App::new(16, 4096);
        let start = UNIX_EPOCH + Duration::from_secs(10);
        for (id, seconds, dependencies) in [
            ("a", 2, vec![]),
            ("b", 5, vec![TaskId("a".into())]),
            ("c", 1, vec![TaskId("a".into())]),
        ] {
            app.tasks.insert(
                TaskId(id.into()),
                TaskInfo {
                    id: TaskId(id.into()),
                    recipe: id.into(),
                    task: "do_build".into(),
                    state: TaskState::Completed,
                    started: Some(start),
                    finished: Some(start + Duration::from_secs(seconds)),
                    dependencies,
                    ..TaskInfo::default()
                },
            );
        }
        let rows = app.overview_timeline(start + Duration::from_secs(8));
        assert!(rows.iter().find(|row| row.id == "b").unwrap().critical);
        assert!(!rows.iter().find(|row| row.id == "c").unwrap().critical);
    }

    #[test]
    fn cache_projection_keeps_paths_and_observed_outcomes_separate() {
        let mut app = App::new(16, 4096);
        app.workspace = Workspace::default();
        app.workspace
            .variables
            .insert("SSTATE_DIR".into(), "/cache/sstate".into());
        app.tasks.insert(
            TaskId("s".into()),
            TaskInfo {
                id: TaskId("s".into()),
                task: "do_packagedata_setscene".into(),
                state: TaskState::Completed,
                ..TaskInfo::default()
            },
        );
        app.tasks.insert(
            TaskId("f".into()),
            TaskInfo {
                id: TaskId("f".into()),
                task: "do_fetch".into(),
                state: TaskState::Failed,
                ..TaskInfo::default()
            },
        );
        let projection = app.overview_cache();
        assert_eq!(projection.sstate_hits, 1);
        assert_eq!(projection.fetch_failed, 1);
        assert_eq!(projection.sstate_dir.as_deref(), Some("/cache/sstate"));
        assert_eq!(projection.downloads_dir, None);
    }

    #[test]
    fn image_size_delta_compares_consecutive_snapshots_for_the_same_target() {
        let mut app = App::new(16, 4096);
        let composition = |path: &str, installed_size_bytes| RootfsComposition {
            image: ImageArtifactIdentity {
                machine: "qemux86-64".into(),
                image: "core-image-minimal".into(),
                path: PathBuf::from(path),
            },
            installed_packages: RootfsAuthority::Available(RootfsPackageInventory {
                packages: vec![RootfsInstalledPackage {
                    identity: PackageIdentity::new("busybox"),
                    recipe: Some("busybox".into()),
                    category: "base".into(),
                    installed_size_bytes,
                    file_count: 10,
                }],
            }),
            filesystem_tree: RootfsAuthority::Unavailable {
                reason: "not loaded".into(),
            },
        };
        let previous = composition("/tmp/core-image-minimal-1.rootfs.tar", 1_000);
        let current = composition("/tmp/core-image-minimal-2.rootfs.tar", 1_250);
        app.record_overview_image_size(&previous);
        app.record_overview_image_size(&current);
        app.rootfs_composition = RootfsCompositionState::Available {
            request: RootfsCompositionRequest {
                generation: 2,
                image: current.image.clone(),
            },
            composition: current,
        };
        assert_eq!(
            app.overview_image_size_delta(),
            Some(OverviewImageSizeDelta {
                current_bytes: 1_250,
                previous_bytes: 1_000,
                delta_bytes: 250,
            })
        );
    }
}
