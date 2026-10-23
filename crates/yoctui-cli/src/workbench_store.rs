//! Debounced recovery storage. Disk access never runs on the input/render path.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use yoctui_model::{App, WorkbenchState};

pub(crate) struct Store {
    path: PathBuf,
    loading: Option<tokio::task::JoinHandle<anyhow::Result<(WorkbenchState, std::fs::File)>>>,
    lock: Option<std::fs::File>,
    saving: Option<(u64, tokio::task::JoinHandle<anyhow::Result<()>>)>,
    ready: bool,
    observed: u64,
    next_save: Instant,
    last_attempt: Instant,
}
impl Store {
    pub fn start() -> Self {
        Self::start_at(crate::workbench_io::store_path())
    }
    fn start_at(path: PathBuf) -> Self {
        let source = path.clone();
        Self {
            path,
            loading: Some(tokio::task::spawn_blocking(move || {
                anyhow::ensure!(
                    source.is_absolute(),
                    "Workbench requires an absolute XDG_STATE_HOME or HOME"
                );
                let parent = source
                    .parent()
                    .ok_or_else(|| anyhow::anyhow!("Workbench store has no directory"))?;
                std::fs::create_dir_all(parent)?;
                let lock = std::fs::OpenOptions::new()
                    .create(true)
                    .truncate(false)
                    .read(true)
                    .write(true)
                    .open(source.with_extension("lock"))?;
                lock.try_lock().map_err(|_| anyhow::anyhow!("Another Yoctui client is editing the saved Workbench sessions; close that client first"))?;
                Ok((crate::workbench_io::load(&source)?, lock))
            })),
            saving: None,
            lock: None,
            ready: false,
            observed: 0,
            next_save: Instant::now(),
            last_attempt: Instant::now(),
        }
    }
    pub fn ready(&self) -> bool {
        self.ready
    }
    pub async fn poll(&mut self, app: &mut App) -> bool {
        let mut changed = false;
        if self.loading.as_ref().is_some_and(|h| h.is_finished()) {
            let result = self.loading.take().expect("finished recovery load").await;
            match result {
                Ok(Ok((state, lock))) => {
                    let deferred = app.workbench.deferred_recipe.take();
                    app.workbench = state;
                    app.workbench.deferred_recipe = deferred;
                    self.lock = Some(lock);
                    self.ready = true;
                }
                result => {
                    app.workbench.persistence_error = Some(format!(
                        "Workspace recovery unavailable; original file retained: {result:?}"
                    ));
                }
            }
            changed = true;
        }
        if self.saving.as_ref().is_some_and(|(_, h)| h.is_finished()) {
            let (revision, worker) = self.saving.take().expect("finished recovery save");
            match worker.await {
                Ok(Ok(())) => {
                    app.workbench.persisted_revision = revision;
                    app.workbench.persistence_error = None;
                }
                result => {
                    app.workbench.persistence_error =
                        Some(format!("Workspace recovery save failed: {result:?}"));
                    self.next_save = Instant::now() + Duration::from_secs(5);
                }
            }
            changed = true;
        }
        if self.observed != app.workbench.revision {
            self.observed = app.workbench.revision;
            self.next_save = Instant::now() + Duration::from_millis(750);
        }
        if self.ready
            && self.saving.is_none()
            && self.observed != app.workbench.persisted_revision
            && (Instant::now() >= self.next_save
                || self.last_attempt.elapsed() >= Duration::from_secs(5))
        {
            self.last_attempt = Instant::now();
            let snapshot = app.workbench.saved();
            let target = self.path.clone();
            self.saving = Some((
                self.observed,
                tokio::task::spawn_blocking(move || crate::workbench_io::save(&target, &snapshot)),
            ));
        }
        changed
    }
    pub async fn flush(&mut self, app: &mut App) -> anyhow::Result<()> {
        if let Some(worker) = self.loading.take() {
            let _ = worker.await;
        }
        if let Some((revision, worker)) = self.saving.take() {
            worker.await??;
            app.workbench.persisted_revision = revision;
        }
        if self.ready && app.workbench.revision != app.workbench.persisted_revision {
            let snapshot = app.workbench.saved();
            let target = self.path.clone();
            tokio::task::spawn_blocking(move || crate::workbench_io::save(&target, &snapshot))
                .await??;
            app.workbench.persisted_revision = app.workbench.revision;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "tests/workbench_store.rs"]
mod tests;
