use std::path::PathBuf;

pub const MAX_GLOBAL_SEARCH_HITS: usize = 500;
pub const MAX_GLOBAL_SEARCH_PREVIEW_CHARS: usize = 320;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GlobalSearchContentKind {
    Recipe,
    Configuration,
    Class,
    LayerSource,
    PokyBitBakeSource,
    BuildLog,
    GeneratedMetadata,
    ImageRootfs,
}

impl GlobalSearchContentKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Recipe => "Recipe",
            Self::Configuration => "Configuration",
            Self::Class => "Class",
            Self::LayerSource => "Layer source",
            Self::PokyBitBakeSource => "Poky / BitBake source",
            Self::BuildLog => "Build log",
            Self::GeneratedMetadata => "Generated metadata",
            Self::ImageRootfs => "Generated image rootfs",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSearchHit {
    pub kind: GlobalSearchContentKind,
    pub path: PathBuf,
    pub line: u64,
    pub column: u64,
    pub preview: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum GlobalSearchContentState {
    #[default]
    Idle,
    Loading {
        generation: u64,
        query: String,
    },
    Ready {
        generation: u64,
        query: String,
        hits: Vec<GlobalSearchHit>,
        truncated: bool,
        searched_scopes: Vec<String>,
    },
    Failed {
        generation: u64,
        query: String,
        message: String,
    },
}

impl GlobalSearchContentState {
    pub fn hits(&self) -> &[GlobalSearchHit] {
        match self {
            Self::Ready { hits, .. } => hits,
            Self::Idle | Self::Loading { .. } | Self::Failed { .. } => &[],
        }
    }

    pub const fn loading(&self) -> bool {
        matches!(self, Self::Loading { .. })
    }
}
