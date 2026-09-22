use super::*;
use regex::RegexBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread,
};
use yoctui_model::{
    GlobalSearchContentKind, MAX_GLOBAL_SEARCH_HITS, MAX_GLOBAL_SEARCH_PREVIEW_CHARS,
};

const MAX_SEARCH_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_VISITED_DIRECTORIES: usize = 200_000;
pub(crate) const MAX_HITS_PER_CONTENT_KIND: usize = 60;
const SEARCH_QUEUE_CAPACITY: usize = 512;
const MAX_SEARCH_WORKERS: usize = 4;

struct ScanMatches {
    hits: Vec<GlobalSearchHit>,
    seen: BTreeSet<(PathBuf, u64, u64)>,
    kind_counts: BTreeMap<GlobalSearchContentKind, usize>,
    truncated: bool,
}

struct SharedScanner<'a> {
    expression: regex::Regex,
    cancellation: &'a GlobalSearchCancellation,
    matches: Mutex<ScanMatches>,
    stop: AtomicBool,
}

impl SharedScanner<'_> {
    fn done(&self) -> bool {
        self.cancellation.cancelled() || self.stop.load(Ordering::Acquire)
    }

    fn search_file(&self, path: &Path, kind: GlobalSearchContentKind) {
        if self.done()
            || self
                .matches
                .lock()
                .expect("global search match state")
                .kind_counts
                .get(&kind)
                .copied()
                .unwrap_or(0)
                >= MAX_HITS_PER_CONTENT_KIND
        {
            return;
        }
        let Ok(metadata) = fs::metadata(path) else {
            return;
        };
        if metadata.len() == 0 || metadata.len() > MAX_SEARCH_FILE_BYTES {
            return;
        }
        let Ok(bytes) = fs::read(path) else {
            return;
        };
        if bytes.contains(&0) {
            return;
        }
        let Ok(content) = std::str::from_utf8(&bytes) else {
            return;
        };
        for (index, line) in content.lines().enumerate() {
            if self.done() {
                return;
            }
            for found in self.expression.find_iter(line) {
                let line_number = index as u64 + 1;
                let column = line[..found.start()].chars().count() as u64 + 1;
                let identity = (path.to_path_buf(), line_number, column);
                let mut matches = self.matches.lock().expect("global search match state");
                if matches.kind_counts.get(&kind).copied().unwrap_or(0) >= MAX_HITS_PER_CONTENT_KIND
                {
                    return;
                }
                if matches.seen.insert(identity) {
                    matches.hits.push(GlobalSearchHit {
                        kind,
                        path: path.to_path_buf(),
                        line: line_number,
                        column,
                        preview: bounded_preview(line),
                        image: (kind == GlobalSearchContentKind::ImageRootfs)
                            .then(|| image_name_for_rootfs_path(path))
                            .flatten(),
                    });
                    let kind_limit = {
                        let count = matches.kind_counts.entry(kind).or_default();
                        *count += 1;
                        *count >= MAX_HITS_PER_CONTENT_KIND
                    };
                    if matches.hits.len() >= MAX_GLOBAL_SEARCH_HITS {
                        matches.truncated = true;
                        self.stop.store(true, Ordering::Release);
                        return;
                    }
                    if kind_limit {
                        matches.truncated = true;
                        return;
                    }
                }
            }
        }
    }
}

struct DirectoryWalker<'a> {
    sender: mpsc::SyncSender<(PathBuf, GlobalSearchContentKind)>,
    scanner: &'a SharedScanner<'a>,
    visited_directories: usize,
    truncated: bool,
}

impl DirectoryWalker<'_> {
    fn walk(&mut self, directory: &Path) {
        if self.scanner.done() {
            return;
        }
        self.visited_directories = self.visited_directories.saturating_add(1);
        if self.visited_directories > MAX_VISITED_DIRECTORIES {
            self.truncated = true;
            self.scanner.stop.store(true, Ordering::Release);
            return;
        }
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            if self.scanner.done() {
                break;
            }
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            let path = entry.path();
            if file_type.is_dir() {
                if search_directory(&entry.file_name().to_string_lossy()) {
                    self.walk(&path);
                }
            } else if file_type.is_file()
                && self
                    .sender
                    .send((path.clone(), classify_content(&path)))
                    .is_err()
            {
                break;
            }
        }
    }
}

pub fn scan_global_content(
    plan: &GlobalSearchPlan,
    cancellation: &GlobalSearchCancellation,
) -> Result<GlobalSearchScanResult, String> {
    if plan.query.trim().is_empty() {
        return Ok(GlobalSearchScanResult {
            hits: Vec::new(),
            truncated: false,
            searched_scopes: Vec::new(),
        });
    }
    let expression = RegexBuilder::new(plan.query.trim())
        .case_insensitive(true)
        .build()
        .map_err(|error| format!("invalid regular expression: {error}"))?;
    let Some(build_dir) = plan.build_dir.as_deref() else {
        return Ok(GlobalSearchScanResult {
            hits: Vec::new(),
            truncated: false,
            searched_scopes: Vec::new(),
        });
    };
    let scanner = SharedScanner {
        expression,
        cancellation,
        matches: Mutex::new(ScanMatches {
            hits: Vec::new(),
            seen: BTreeSet::new(),
            kind_counts: BTreeMap::new(),
            truncated: false,
        }),
        stop: AtomicBool::new(false),
    };
    let workers =
        thread::available_parallelism().map_or(1, |count| count.get().min(MAX_SEARCH_WORKERS));
    let walker_truncated = thread::scope(|scope| {
        let (sender, receiver) =
            mpsc::sync_channel::<(PathBuf, GlobalSearchContentKind)>(SEARCH_QUEUE_CAPACITY);
        let receiver = Arc::new(Mutex::new(receiver));
        for _ in 0..workers {
            let receiver = Arc::clone(&receiver);
            let scanner = &scanner;
            scope.spawn(move || {
                loop {
                    let work = receiver.lock().expect("global search queue").recv();
                    let Ok((path, kind)) = work else {
                        break;
                    };
                    scanner.search_file(&path, kind);
                }
            });
        }
        let mut walker = DirectoryWalker {
            sender,
            scanner: &scanner,
            visited_directories: 0,
            truncated: false,
        };
        walker.walk(build_dir);
        walker.truncated
    });
    let mut matches = scanner
        .matches
        .into_inner()
        .expect("global search match state");
    matches.hits.sort_unstable_by(|left, right| {
        (&left.path, left.line, left.column).cmp(&(&right.path, right.line, right.column))
    });
    Ok(GlobalSearchScanResult {
        hits: matches.hits,
        truncated: matches.truncated || walker_truncated,
        searched_scopes: vec![format!("build={}", build_dir.display())],
    })
}

fn search_directory(name: &str) -> bool {
    !matches!(
        name,
        ".git" | ".repo" | "cache" | "downloads" | "sstate-cache" | "__pycache__" | "node_modules"
    )
}

fn classify_content(path: &Path) -> GlobalSearchContentKind {
    if path.components().any(|part| part.as_os_str() == "rootfs") {
        GlobalSearchContentKind::ImageRootfs
    } else if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("log.") || name.starts_with("run."))
    {
        GlobalSearchContentKind::BuildLog
    } else {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("conf") => GlobalSearchContentKind::Configuration,
            Some("bb" | "bbappend" | "inc") => GlobalSearchContentKind::Recipe,
            Some("bbclass") => GlobalSearchContentKind::Class,
            _ => GlobalSearchContentKind::GeneratedMetadata,
        }
    }
}

fn bounded_preview(line: &str) -> String {
    line.chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(MAX_GLOBAL_SEARCH_PREVIEW_CHARS)
        .collect::<String>()
        .trim()
        .to_owned()
}

fn image_name_for_rootfs_path(path: &Path) -> Option<String> {
    let components = path
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    let rootfs = components
        .iter()
        .position(|component| component == "rootfs")?;
    rootfs
        .checked_sub(2)
        .and_then(|index| components.get(index))
        .cloned()
}
