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
#[cfg(test)]
pub use scanner::scan_global_content;
pub(crate) use scanner::scan_global_content_streaming;

#[derive(Debug, Clone, Default)]
pub struct GlobalSearchCancellation(Arc<AtomicBool>);

impl GlobalSearchCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub(crate) fn cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

#[derive(Debug, Clone)]
pub struct GlobalSearchPlan {
    pub query: String,
    pub build_dir: Option<PathBuf>,
    pub scope_label: String,
    pub file: Option<PathBuf>,
    pub target: yoctui_model::GlobalSearchTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalSearchScanResult {
    pub hits: Vec<GlobalSearchHit>,
    pub truncated: bool,
    pub searched_scopes: Vec<String>,
}

impl GlobalSearchPlan {
    pub fn for_app(app: &App, build_dir: &Path, query: String) -> Self {
        let (build_dir, scope) = app.global_search_root.as_deref().map_or_else(
            || {
                (
                    app.workspace.build_dir.as_deref().unwrap_or(build_dir),
                    "build",
                )
            },
            |root| (root, "workspace"),
        );
        Self {
            query,
            build_dir: safe_search_root(build_dir).then(|| build_dir.to_path_buf()),
            scope_label: scope.into(),
            target: app.global_search_target,
            file: app
                .global_search_root
                .as_ref()
                .and_then(|_| match app.active_dialog() {
                    Some(yoctui_model::Dialog::RecipeEditor(editor))
                        if matches!(
                            editor.context,
                            yoctui_model::SourceEditorContext::DeviceTree
                                | yoctui_model::SourceEditorContext::Rootfs
                        ) =>
                    {
                        editor.selected_path()
                    }
                    _ => None,
                }),
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
