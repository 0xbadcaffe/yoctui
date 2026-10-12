//! One blocking file worker; scans/comparisons never freeze the input loop.
use std::{collections::VecDeque, fs, io::Read, path::Path};
use yoctui_model::{Action, App, Effect, FILE_DIFF_MAX_BYTES, FileDiffAction, FileDiffRequest};

#[derive(Default)]
pub(crate) struct FileDiffIo {
    worker: Option<tokio::task::JoinHandle<Action>>,
    queued: VecDeque<FileDiffRequest>,
    active: Option<FileDiffRequest>,
}
impl FileDiffIo {
    pub fn submit(&mut self, effect: Effect) {
        let Effect::FileDiff(request) = effect else {
            return;
        };
        // Keep only the latest request per pane or comparison, including F5 spam.
        self.queued.retain(|old| match (old, &request) {
            (
                FileDiffRequest::Directory { pane: a, .. },
                FileDiffRequest::Directory { pane: b, .. },
            ) => a != b,
            (FileDiffRequest::Compare { .. }, FileDiffRequest::Compare { .. }) => false,
            _ => true,
        });
        self.queued.push_back(request);
        self.start();
    }
    fn start(&mut self) {
        if self.worker.is_some() {
            return;
        }
        if let Some(request) = self.queued.pop_front() {
            self.active = Some(request.clone());
            self.worker = Some(tokio::task::spawn_blocking(move || execute(request)));
        }
    }
    pub async fn poll(&mut self, app: &mut App) -> bool {
        if !self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            return false;
        }
        let result = self.worker.take().expect("finished worker").await;
        let action = result.unwrap_or_else(|error| {
            failed(
                self.active.clone().expect("active request"),
                error.to_string(),
            )
        });
        yoctui_model::update(app, action);
        self.active = None;
        self.start();
        true
    }
}

fn execute(request: FileDiffRequest) -> Action {
    match request {
        FileDiffRequest::Directory {
            pane,
            request,
            directory,
        } => {
            let result = crate::layer_browser::scan_layer_directory(&directory, false)
                .map_err(|e| format!("{}: {e}", directory.display()));
            Action::FileDiff(FileDiffAction::DirectoryLoaded {
                pane,
                request,
                directory,
                result,
            })
        }
        FileDiffRequest::Compare { generation, paths } => {
            let result = read_text(&paths[0]).and_then(|left| {
                read_text(&paths[1])
                    .and_then(|right| yoctui_model::compare_file_text(paths, &left, &right))
            });
            Action::FileDiff(FileDiffAction::Compared { generation, result })
        }
    }
}

fn failed(request: FileDiffRequest, error: String) -> Action {
    match request {
        FileDiffRequest::Directory {
            pane,
            request,
            directory,
        } => Action::FileDiff(FileDiffAction::DirectoryLoaded {
            pane,
            request,
            directory,
            result: Err(error),
        }),
        FileDiffRequest::Compare { generation, .. } => Action::FileDiff(FileDiffAction::Compared {
            generation,
            result: Err(error),
        }),
    }
}

fn read_text(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file.", path.display()));
    }
    if metadata.len() > FILE_DIFF_MAX_BYTES as u64 {
        return Err(format!(
            "{} exceeds the 2 MiB comparison limit.",
            path.display()
        ));
    }
    let file = fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(FILE_DIFF_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > FILE_DIFF_MAX_BYTES {
        return Err("File grew beyond the 2 MiB comparison limit.".into());
    }
    if bytes.contains(&0) {
        return Err(format!(
            "{} is binary; select UTF-8 text files.",
            path.display()
        ));
    }
    String::from_utf8(bytes).map_err(|_| format!("{} is not UTF-8 text.", path.display()))
}

#[cfg(test)]
#[path = "tests/file_diff_io.rs"]
mod tests;
