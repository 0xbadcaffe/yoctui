//! Short-lived decoration cache, invalidated by nonrecursive directory/index
//! watches. File listings and target ownership are always acquired afresh.
use super::*;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use std::{
    collections::VecDeque,
    path::Path,
    sync::atomic::AtomicU64,
    time::{Duration, Instant},
};

const MAX_ENTRIES: usize = 16;
const MAX_WATCHES: usize = 32;
const MAX_CACHE_BYTES: usize = 8 * 1024 * 1024;
const TTL: Duration = Duration::from_secs(2);

struct Entry {
    directory: PathBuf,
    output: Vec<u8>,
    at: Instant,
    epoch: u64,
}

pub(super) struct GitCache {
    entries: VecDeque<Entry>,
    bytes: usize,
    pub(super) epoch: Arc<AtomicU64>,
    watcher: Option<RecommendedWatcher>,
    watched: VecDeque<PathBuf>,
}

impl Default for GitCache {
    fn default() -> Self {
        Self {
            entries: VecDeque::new(),
            bytes: 0,
            epoch: Arc::new(AtomicU64::new(0)),
            watcher: None,
            watched: VecDeque::new(),
        }
    }
}

impl GitCache {
    // Called only inside an already bounded blocking directory worker, never
    // while the input loop holds the cache lock.
    pub(super) fn observe(&mut self, directory: &Path) {
        if self.watcher.is_none() {
            let epoch = Arc::clone(&self.epoch);
            self.watcher =
                notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                    if !matches!(
                        event.as_ref().map(|event| event.kind),
                        Ok(notify::EventKind::Access(_))
                    ) {
                        epoch.fetch_add(1, Ordering::Relaxed);
                    }
                })
                .ok();
        }
        self.watch(directory);
        for ancestor in directory.ancestors() {
            let git = ancestor.join(".git");
            if git.exists() {
                self.watch(&git);
                break;
            }
        }
    }

    fn watch(&mut self, path: &Path) {
        if self.watched.iter().any(|existing| existing == path) {
            return;
        }
        let Some(watcher) = &mut self.watcher else {
            return;
        };
        if self.watched.len() == MAX_WATCHES {
            if let Some(oldest) = self.watched.pop_front() {
                let _ = watcher.unwatch(&oldest);
            }
            self.epoch.fetch_add(1, Ordering::Relaxed);
        }
        if watcher.watch(path, RecursiveMode::NonRecursive).is_ok() {
            self.watched.push_back(path.to_owned());
        }
    }

    fn get(&mut self, directory: &Path, now: Instant) -> Option<Vec<u8>> {
        let epoch = self.epoch.load(Ordering::Relaxed);
        let index = self.entries.iter().position(|entry| {
            entry.directory == directory
                && entry.epoch == epoch
                && now.saturating_duration_since(entry.at) < TTL
        })?;
        let entry = self.entries.remove(index)?;
        let output = entry.output.clone();
        self.entries.push_back(entry);
        Some(output)
    }

    fn put(&mut self, directory: PathBuf, output: Vec<u8>, epoch: u64) {
        if output.len() > MAX_CACHE_BYTES {
            return;
        }
        self.entries.retain(|entry| entry.directory != directory);
        self.bytes = self.entries.iter().map(|entry| entry.output.len()).sum();
        while self.entries.len() >= MAX_ENTRIES || self.bytes + output.len() > MAX_CACHE_BYTES {
            let oldest = self.entries.pop_front().expect("bounded cache eviction");
            self.bytes -= oldest.output.len();
        }
        self.bytes += output.len();
        self.entries.push_back(Entry {
            directory,
            output,
            at: Instant::now(),
            epoch,
        });
    }
}

pub(super) async fn status(
    directory: &Path,
    cache: Arc<std::sync::Mutex<GitCache>>,
) -> Option<Vec<u8>> {
    let epoch = {
        let mut cache = cache.lock().unwrap_or_else(|error| error.into_inner());
        if let Some(output) = cache.get(directory, Instant::now()) {
            return Some(output);
        }
        cache.epoch.load(Ordering::Relaxed)
    };
    let output = crate::source_git::directory_git_status(directory).await?;
    cache.lock().unwrap_or_else(|error| error.into_inner()).put(
        directory.to_owned(),
        output.clone(),
        epoch,
    );
    Some(output)
}

#[cfg(test)]
#[path = "../tests/layer_browser_git_cache.rs"]
mod tests;
