//! Bounded background I/O for explicitly added development folders.
use std::{
    collections::VecDeque,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use yoctui_model::{
    Action, App, Effect, SavedWorkbenches, WorkbenchAction as W, WorkbenchRequest as R,
};

#[derive(Default)]
pub(crate) struct WorkbenchIo {
    worker: Option<tokio::task::JoinHandle<Action>>,
    queue: VecDeque<R>,
}
impl WorkbenchIo {
    pub fn submit(&mut self, app: &mut App, effect: Effect) {
        let Effect::Workbench(request) = effect else {
            return;
        };
        // Directory/read requests supersede obsolete navigation, not saves or creates.
        match &request {
            R::Directory { session, root, directory, .. } => self.queue.retain(|r| !matches!(r, R::Directory { session: s, root: p, directory: d, .. } if s == session && p == root && d == directory)),
            R::Read { .. } => self.queue.retain(|r| !matches!(r, R::Read { .. })),
            _ => {},
        }
        if self.queue.len() >= 32 {
            let _ = yoctui_model::update(app, Action::Workbench(rejected(request)));
            return;
        }
        self.queue.push_back(request);
        self.start();
    }
    fn start(&mut self) {
        if self.worker.is_none()
            && let Some(request) = self.queue.pop_front()
        {
            self.worker = Some(tokio::task::spawn_blocking(move || {
                Action::Workbench(execute(request))
            }));
        }
    }
    pub async fn poll(&mut self, app: &mut App) -> Option<Effect> {
        if !self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            return None;
        }
        let result = self.worker.take().expect("finished workbench worker").await;
        let effect = match result {
            Ok(action) => yoctui_model::update(app, action),
            Err(error) => {
                app.notification = Some(format!("Workbench worker failed: {error}"));
                None
            }
        };
        self.start();
        effect
    }
    pub fn busy(&self) -> bool {
        self.worker.is_some() || !self.queue.is_empty()
    }
    pub fn finished(&self) -> bool {
        self.worker.as_ref().is_some_and(|w| w.is_finished())
    }
}

fn rejected(request: R) -> W {
    let error =
        "Workspace I/O queue is full; operation was not started. Buffer retained; retry shortly."
            .to_string();
    match request {
        R::Folder {
            session,
            generation,
            ..
        } => W::FolderLoaded {
            session,
            generation,
            result: Err(error),
        },
        R::Directory {
            session,
            root,
            directory,
            generation,
        } => W::DirectoryLoaded {
            session,
            root,
            directory,
            generation,
            result: Err(error),
        },
        R::Read {
            session,
            root,
            path,
            generation,
        } => W::FileLoaded {
            session,
            root,
            path,
            generation,
            result: Err(error),
        },
        R::Save {
            session,
            path,
            content,
            ..
        } => W::Saved {
            session,
            path,
            content,
            result: Err(error),
        },
        R::NewFile {
            session,
            root,
            path,
        } => W::NewFileCreated {
            session,
            root,
            path,
            result: Err(error),
        },
    }
}

fn validate_path(root: &Path, path: &Path) -> anyhow::Result<PathBuf> {
    let canonical_root = root.canonicalize()?;
    anyhow::ensure!(
        canonical_root == root,
        "workspace root moved or became a symlink; add it again"
    );
    anyhow::ensure!(
        path.starts_with(root),
        "path is outside the workspace folder"
    );
    for part in path.strip_prefix(root)?.components() {
        anyhow::ensure!(
            matches!(part, std::path::Component::Normal(_)),
            "invalid relative workspace path"
        );
    }
    let canonical = path.canonicalize()?;
    anyhow::ensure!(
        canonical.starts_with(root),
        "symlink escapes the workspace folder"
    );
    Ok(canonical)
}

pub(crate) fn execute(request: R) -> W {
    match request {
        R::Folder {
            session,
            generation,
            path,
        } => W::FolderLoaded {
            session,
            generation,
            result: (|| -> anyhow::Result<_> {
                let root = path.canonicalize()?;
                anyhow::ensure!(root.is_dir(), "choose a directory");
                Ok(root)
            })()
            .map_err(|e| e.to_string()),
        },
        R::Directory {
            session,
            root,
            directory,
            generation,
        } => {
            let result = (|| -> anyhow::Result<_> {
                let canonical = validate_path(&root, &directory)?;
                anyhow::ensure!(
                    canonical == directory,
                    "directory symlinks are not traversed"
                );
                let mut entries =
                    crate::layer_browser::scan_layer_directory_stream(&directory, |_| true)?;
                entries.retain(|entry| {
                    fs::symlink_metadata(&entry.path).is_ok_and(|m| !m.file_type().is_symlink())
                });
                Ok(entries)
            })()
            .map_err(|e| e.to_string());
            W::DirectoryLoaded {
                session,
                root,
                directory,
                generation,
                result,
            }
        }
        R::Read {
            session,
            root,
            path,
            generation,
        } => {
            let result = (|| -> anyhow::Result<_> {
                let canonical = validate_path(&root, &path)?;
                anyhow::ensure!(
                    canonical == path && fs::symlink_metadata(&path)?.is_file(),
                    "only regular, non-symlink text files can be edited"
                );
                let mut bytes = Vec::new();
                fs::File::open(&path)?
                    .take((yoctui_model::TEXTAREA_MAX_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)?;
                anyhow::ensure!(
                    bytes.len() <= yoctui_model::TEXTAREA_MAX_BYTES,
                    "file exceeds the 1 MiB editor limit"
                );
                let text = String::from_utf8(bytes)?;
                anyhow::ensure!(
                    yoctui_model::hardware_source_is_text(&text),
                    "binary/control data cannot be edited as text"
                );
                Ok(text)
            })()
            .map_err(|e| e.to_string());
            W::FileLoaded {
                session,
                root,
                path,
                generation,
                result,
            }
        }
        R::Save {
            session,
            root,
            path,
            content,
            expected,
        } => {
            let result = (|| -> anyhow::Result<_> {
                anyhow::ensure!(
                    validate_path(&root, &path)? == path,
                    "file symlinks cannot be replaced"
                );
                crate::recipe_editor::write_recipe_editor_file_atomically(
                    &root, &path, &content, expected,
                )
            })()
            .map_err(|e| e.to_string());
            W::Saved {
                session,
                path,
                content,
                result,
            }
        }
        R::NewFile {
            session,
            root,
            path,
        } => {
            let result = (|| -> anyhow::Result<_> {
                let parent = path
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("file has no parent"))?;
                anyhow::ensure!(
                    validate_path(&root, parent)? == parent,
                    "file parent must not be a symlink"
                );
                fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&path)?
                    .sync_all()?;
                Ok(())
            })()
            .map_err(|e| e.to_string());
            W::NewFileCreated {
                session,
                root,
                path,
                result,
            }
        }
    }
}

pub(crate) fn store_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs_home().join(".local/state"))
        .join("yoctui/workbenches.json")
}
fn dirs_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
}
pub(crate) fn load(path: &Path) -> anyhow::Result<yoctui_model::WorkbenchState> {
    if !path.exists() {
        return Ok(yoctui_model::WorkbenchState::default());
    }
    let metadata = fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_file()
            && !metadata.file_type().is_symlink()
            && metadata.len() <= 20 * 1024 * 1024,
        "invalid or oversized Workbench store"
    );
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(20 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 20 * 1024 * 1024, "oversized Workbench store");
    let saved: SavedWorkbenches = serde_json::from_slice(&bytes)?;
    yoctui_model::WorkbenchState::restore(saved).map_err(anyhow::Error::msg)
}
pub(crate) fn save(path: &Path, saved: &SavedWorkbenches) -> anyhow::Result<()> {
    anyhow::ensure!(
        path.is_absolute(),
        "Workbench store requires an absolute state directory"
    );
    if let Ok(metadata) = fs::symlink_metadata(path) {
        anyhow::ensure!(
            metadata.is_file() && !metadata.file_type().is_symlink(),
            "invalid Workbench store target"
        );
    }
    saved.validate().map_err(anyhow::Error::msg)?;
    let data = serde_json::to_vec(saved)?;
    anyhow::ensure!(
        data.len() <= 20 * 1024 * 1024,
        "Workbench store exceeds 20 MiB"
    );
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("Workbench store has no parent"))?;
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&data)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path)?;
    fs::File::open(parent)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
#[path = "tests/workbench_io.rs"]
mod tests;
