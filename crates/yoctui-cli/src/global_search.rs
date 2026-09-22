#[cfg(test)]
use std::fs;
use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
#[cfg(test)]
use yoctui_model::GlobalSearchContentKind;
use yoctui_model::{App, GlobalSearchHit};

mod scanner;

#[cfg(test)]
pub(crate) use scanner::MAX_HITS_PER_CONTENT_KIND;
pub use scanner::scan_global_content;

#[derive(Debug, Clone, Default)]
pub struct GlobalSearchCancellation(Arc<AtomicBool>);

impl GlobalSearchCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalSearchPlan {
    pub query: String,
    pub build_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSearchScanResult {
    pub hits: Vec<GlobalSearchHit>,
    pub truncated: bool,
    pub searched_scopes: Vec<String>,
}

impl GlobalSearchPlan {
    pub fn for_app(app: &App, build_dir: &Path, query: String) -> Self {
        let build_dir = app.workspace.build_dir.as_deref().unwrap_or(build_dir);
        Self {
            query,
            build_dir: safe_search_root(build_dir).then(|| build_dir.to_path_buf()),
        }
    }
}

pub(super) fn safe_search_root(path: &Path) -> bool {
    path.is_absolute()
        && path.components().count() > 1
        && path.is_dir()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

#[cfg(test)]
#[path = "tests/global_search/mod.rs"]
mod tests;
