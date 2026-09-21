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

