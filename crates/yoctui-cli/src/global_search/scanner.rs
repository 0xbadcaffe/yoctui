use super::*;
use regex::RegexBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufReader, Read},
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
    on_hit: &'a (dyn Fn(&GlobalSearchHit) + Sync),
    target: yoctui_model::GlobalSearchTarget,
}

impl SharedScanner<'_> {
    fn done(&self) -> bool {
        self.cancellation.cancelled() || self.stop.load(Ordering::Acquire)
    }

    fn search_file(&self, path: &Path, kind: GlobalSearchContentKind) {
        let names = self.target == yoctui_model::GlobalSearchTarget::FileNames;
        let kind = if names {
            GlobalSearchContentKind::FileName
        } else {
            kind
        };
        let limit = if names {
            MAX_GLOBAL_SEARCH_HITS
        } else {
            MAX_HITS_PER_CONTENT_KIND
        };
        if self.done()
            || self
                .matches
                .lock()
                .expect("global search match state")
                .kind_counts
                .get(&kind)
                .copied()
                .unwrap_or(0)
                >= limit
        {
            return;
        }
        let bytes = if names {
            path.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .as_bytes()
                .to_vec()
        } else {
            let Ok(metadata) = fs::metadata(path) else {
                return;
            };
            if metadata.len() == 0 || metadata.len() > MAX_SEARCH_FILE_BYTES {
                return;
            }
            let Ok(file) = fs::File::open(path) else {
                return;
            };
            let mut reader = BufReader::new(file).take(MAX_SEARCH_FILE_BYTES + 1);
            let mut bytes = vec![0; metadata.len().min(8192) as usize];
            if reader.read_exact(&mut bytes).is_err() || bytes.contains(&0) {
                return;
            }
            if reader.read_to_end(&mut bytes).is_err()
                || bytes.len() as u64 > MAX_SEARCH_FILE_BYTES
                || bytes.contains(&0)
            {
                return;
            }
            bytes
        };
        let Ok(content) = std::str::from_utf8(&bytes) else {
            return;
        };
        for (index, line) in content.lines().enumerate() {
            if self.done() {
                return;
            }
            for found in self.expression.find_iter(line) {
                let line_number = index as u64 + 1;
                let column = if names {
                    1
                } else {
                    line[..found.start()].chars().count() as u64 + 1
                };
                let identity = (path.to_path_buf(), line_number, column);
                let mut matches = self.matches.lock().expect("global search match state");
                if matches.kind_counts.get(&kind).copied().unwrap_or(0) >= limit {
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
                    // Serialize publication with insertion so subsequent batches never reorder rows.
                    (self.on_hit)(matches.hits.last().expect("inserted search hit"));
                    let kind_limit = {
                        let count = matches.kind_counts.entry(kind).or_default();
                        *count += 1;
                        *count >= limit
                    };
                    if matches.hits.len() >= MAX_GLOBAL_SEARCH_HITS {
                        matches.truncated = true;
                        self.stop.store(true, Ordering::Release);
                        return;
                    }
                    if kind_limit {
                        matches.truncated = true;
                        let kinds = [
                            GlobalSearchContentKind::Recipe,
                            GlobalSearchContentKind::Configuration,
                            GlobalSearchContentKind::Class,
                            GlobalSearchContentKind::BuildLog,
                            GlobalSearchContentKind::GeneratedMetadata,
                            GlobalSearchContentKind::ImageRootfs,
                        ];
                        if kinds.iter().all(|kind| {
                            matches.kind_counts.get(kind).copied().unwrap_or(0)
                                >= MAX_HITS_PER_CONTENT_KIND
                        }) {
                            self.stop.store(true, Ordering::Release);
                        }
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
        // Configuration is small and useful; do not bury it behind a huge tmp/work tree.
        let entries: Box<dyn Iterator<Item = fs::DirEntry>> = if self.visited_directories == 1 {
            let mut entries = entries.flatten().collect::<Vec<_>>();
            entries.sort_by_key(|entry| if entry.file_name() == "conf" { 0 } else { 1 });
            Box::new(entries.into_iter())
        } else {
            Box::new(entries.flatten())
        };
        for entry in entries {
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

#[cfg(test)]
pub fn scan_global_content(
    plan: &GlobalSearchPlan,
    cancellation: &GlobalSearchCancellation,
) -> Result<GlobalSearchScanResult, String> {
    let mut result = scan_global_content_streaming(plan, cancellation, &|_| {})?;
    result.hits.sort_unstable_by(|left, right| {
        (&left.path, left.line, left.column).cmp(&(&right.path, right.line, right.column))
    });
    Ok(result)
}

pub fn scan_global_content_streaming(
    plan: &GlobalSearchPlan,
    cancellation: &GlobalSearchCancellation,
    on_hit: &(dyn Fn(&GlobalSearchHit) + Sync),
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
        on_hit,
        target: plan.target,
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
        if let Some(file) = &plan.file {
            let relative = file.strip_prefix(build_dir).ok();
            let mut path = build_dir.to_path_buf();
            let safe = relative.is_some_and(|relative| {
                relative.components().all(|part| {
                    if !matches!(part, std::path::Component::Normal(_)) {
                        return false;
                    }
                    path.push(part);
                    fs::symlink_metadata(&path)
                        .is_ok_and(|metadata| !metadata.file_type().is_symlink())
                })
            });
            if safe && file.is_file() {
                let _ = walker.sender.send((file.clone(), classify_content(file)));
            }
        } else {
            walker.walk(build_dir);
        }
        walker.truncated
    });
    let matches = scanner
        .matches
        .into_inner()
        .expect("global search match state");
    Ok(GlobalSearchScanResult {
        hits: matches.hits,
        truncated: matches.truncated || walker_truncated,
        searched_scopes: vec![format!("{}={}", plan.scope_label, build_dir.display())],
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
