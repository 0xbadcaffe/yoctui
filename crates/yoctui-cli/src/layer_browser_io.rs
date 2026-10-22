//! Client-local, bounded browsing workers. Input handlers enqueue, never await
//! filesystem/Git I/O. Superseded operations cannot install stale results.
use crate::{Action, App, Effect, LayerBrowserEntry, PathBuf, PreviewKind, Screen, update};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Semaphore, mpsc};

mod git;
mod worker;

#[derive(Clone, PartialEq, Eq)]
struct Context {
    screen: Screen,
    source: Option<PathBuf>,
    build: Option<PathBuf>,
    daemon: Option<yoctui_model::DaemonModelInstanceId>,
    machine: Option<String>,
}

impl Context {
    fn capture(app: &App) -> Self {
        Self {
            screen: app.screen,
            source: app.workspace.source_dir.clone(),
            build: app.workspace.build_dir.clone(),
            daemon: app.daemon.instance_id,
            machine: app.workspace.variables.get("MACHINE").cloned(),
        }
    }
}

enum ResultData {
    Directory {
        entries: Vec<LayerBrowserEntry>,
        complete: bool,
    },
    Preview {
        content: String,
        kind: PreviewKind,
        truncated: bool,
    },
}

struct Operation {
    context: Context,
    layer: String,
    root: PathBuf,
    path: PathBuf,
    cancelled: Arc<AtomicBool>,
    task: tokio::task::JoinHandle<()>,
    results: mpsc::Receiver<Result<ResultData, String>>,
    published: bool,
}

impl Operation {
    fn current(&self, app: &App) -> bool {
        self.context == Context::capture(app)
            && app
                .layer_browser
                .as_ref()
                .is_some_and(|browser| browser.layer == self.layer && browser.root == self.root)
    }
}

impl Drop for Operation {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        self.task.abort();
    }
}

pub(crate) struct LayerBrowserIo {
    directory: Option<Operation>,
    preview: Option<Operation>,
    workers: Arc<Semaphore>,
    git: Arc<std::sync::Mutex<git::GitCache>>,
    git_epoch: Arc<std::sync::atomic::AtomicU64>,
}

impl Default for LayerBrowserIo {
    fn default() -> Self {
        let git = git::GitCache::default();
        let git_epoch = Arc::clone(&git.epoch);
        Self {
            directory: None,
            preview: None,
            workers: Arc::new(Semaphore::new(2)),
            git: Arc::new(std::sync::Mutex::new(git)),
            git_epoch,
        }
    }
}

impl LayerBrowserIo {
    #[cfg(test)]
    pub(crate) fn pending(&self) -> bool {
        self.directory.is_some() || self.preview.is_some()
    }

    pub(crate) fn directory(&mut self, app: &mut App, layer: String, root: PathBuf, path: PathBuf) {
        self.directory = None;
        self.preview = None;
        if app
            .layer_browser
            .as_ref()
            .is_none_or(|browser| browser.layer != layer || browser.root != root)
        {
            let _ = update(
                app,
                Action::LoadLayerBrowserDirectory {
                    layer: layer.clone(),
                    root: root.clone(),
                    directory: root.clone(),
                    entries: Vec::new(),
                },
            );
        }
        if let Some(browser) = &mut app.layer_browser {
            if browser.nodes.contains_key(&path) {
                self.git_epoch.fetch_add(1, Ordering::Relaxed);
            }
            browser.incomplete.insert(path.clone());
        }
        self.directory = Some(worker::directory(
            app,
            layer,
            root,
            path,
            Arc::clone(&self.workers),
            Arc::clone(&self.git),
        ));
    }

    pub(crate) fn preview(&mut self, app: &mut App, path: PathBuf) {
        self.preview = None;
        if let Some(browser) = &mut app.layer_browser {
            // Input no longer awaits the read: never show the previous file's
            // content beneath the newly selected filename while it is pending.
            browser.preview.clear();
            browser.preview_kind = PreviewKind::Unavailable;
            browser.preview_truncated = false;
        }
        if let Some(browser) = &app.layer_browser {
            self.preview = Some(worker::preview(
                app,
                browser.layer.clone(),
                browser.root.clone(),
                path,
                Arc::clone(&self.workers),
            ));
        }
    }

    pub(crate) fn poll(&mut self, app: &mut App) -> bool {
        let mut changed = false;
        if self.directory.as_ref().is_some_and(|op| {
            !op.current(app)
                || (op.published
                    && op.path != op.root
                    && app
                        .layer_browser
                        .as_ref()
                        .is_some_and(|browser| !browser.expanded.contains(&op.path)))
        }) {
            self.directory = None;
        }
        // Drain only a bounded number of chunks per UI tick.
        for _ in 0..2 {
            let Some(operation) = &mut self.directory else {
                break;
            };
            let data = match operation.results.try_recv() {
                Ok(data) => data,
                Err(mpsc::error::TryRecvError::Empty) => break,
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    self.directory = None;
                    break;
                }
            };
            changed = true;
            match data {
                Ok(ResultData::Directory { entries, complete }) => {
                    operation.published = true;
                    let path = operation.path.clone();
                    let effect = update(
                        app,
                        Action::LoadLayerBrowserDirectory {
                            layer: operation.layer.clone(),
                            root: operation.root.clone(),
                            directory: path.clone(),
                            entries,
                        },
                    );
                    if !complete && let Some(browser) = &mut app.layer_browser {
                        browser.incomplete.insert(path);
                    }
                    if let Some(Effect::LoadLayerBrowserPreview(path)) = effect {
                        self.preview(app, path);
                    }
                    if complete {
                        self.directory = None;
                        break;
                    }
                }
                Err(message) => {
                    app.notification = Some(format!("Could not read layer directory: {message}"));
                    self.directory = None;
                    break;
                }
                _ => {}
            }
        }
        if self.preview.as_ref().is_some_and(|op| {
            !op.current(app)
                || app
                    .layer_browser
                    .as_ref()
                    .and_then(|browser| browser.selected_entry())
                    .is_none_or(|entry| entry.path != op.path)
        }) {
            self.preview = None;
        }
        if let Some(operation) = &mut self.preview {
            match operation.results.try_recv() {
                Ok(Ok(ResultData::Preview {
                    content,
                    kind,
                    truncated,
                })) => {
                    let _ = update(
                        app,
                        Action::LoadLayerBrowserPreview {
                            path: operation.path.clone(),
                            content,
                            kind,
                            truncated,
                        },
                    );
                    self.preview = None;
                    changed = true;
                }
                Ok(Err(message)) => {
                    app.notification = Some(format!("Could not preview layer file: {message}"));
                    self.preview = None;
                    changed = true;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => self.preview = None,
                _ => {}
            }
        }
        changed
    }
}

#[cfg(test)]
#[path = "tests/layer_browser_io.rs"]
mod tests;
