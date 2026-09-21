use regex::RegexBuilder;
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use yoctui_model::{
    App, GlobalSearchContentKind, GlobalSearchHit, MAX_GLOBAL_SEARCH_HITS,
    MAX_GLOBAL_SEARCH_PREVIEW_CHARS,
};

const MAX_SEARCH_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_VISITED_DIRECTORIES: usize = 200_000;
const MAX_HITS_PER_CONTENT_KIND: usize = 60;

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

fn safe_search_root(path: &Path) -> bool {
    path.is_absolute()
        && path.components().count() > 1
        && path.is_dir()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
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
    let mut scanner = Scanner {
        expression,
        cancellation,
        hits: Vec::new(),
        seen: BTreeSet::new(),
        kind_counts: BTreeMap::new(),
        visited_directories: 0,
        truncated: false,
        stop: false,
        searched_scopes: BTreeSet::new(),
    };
    if !scanner.done()
        && let Some(build_dir) = &plan.build_dir
    {
        scanner
            .searched_scopes
            .insert(format!("build={}", build_dir.display()));
        scanner.walk_tree(build_dir)?;
    }
    Ok(GlobalSearchScanResult {
        hits: scanner.hits,
        truncated: scanner.truncated,
        searched_scopes: scanner.searched_scopes.into_iter().collect(),
    })
}

struct Scanner<'a> {
    expression: regex::Regex,
    cancellation: &'a GlobalSearchCancellation,
    hits: Vec<GlobalSearchHit>,
    seen: BTreeSet<(PathBuf, u64, u64)>,
    kind_counts: BTreeMap<GlobalSearchContentKind, usize>,
    visited_directories: usize,
    truncated: bool,
    stop: bool,
    searched_scopes: BTreeSet<String>,
}

impl Scanner<'_> {
    fn done(&self) -> bool {
        self.cancellation.cancelled() || self.stop || self.hits.len() >= MAX_GLOBAL_SEARCH_HITS
    }

    fn walk_tree(&mut self, directory: &Path) -> Result<(), String> {
        if self.done() {
            return Ok(());
        }
        self.count_directory()?;
        for entry in directory_entries(directory) {
            if self.done() {
                break;
            }
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if search_directory(&name) {
                    self.walk_tree(&path)?;
                }
            } else if file_type.is_file() {
                let kind = classify_content(&path);
                self.search_file(&path, kind)?;
            }
        }
        Ok(())
    }

    fn count_directory(&mut self) -> Result<(), String> {
        self.visited_directories = self.visited_directories.saturating_add(1);
        if self.visited_directories > MAX_VISITED_DIRECTORIES {
            self.truncated = true;
            self.stop = true;
        }
        Ok(())
    }

    fn search_file(&mut self, path: &Path, kind: GlobalSearchContentKind) -> Result<(), String> {
        if self.kind_counts.get(&kind).copied().unwrap_or(0) >= MAX_HITS_PER_CONTENT_KIND {
            return Ok(());
        }
        let Ok(metadata) = fs::metadata(path) else {
            return Ok(());
        };
        if metadata.len() == 0 || metadata.len() > MAX_SEARCH_FILE_BYTES {
            return Ok(());
        }
        let Ok(bytes) = fs::read(path) else {
            return Ok(());
        };
        if bytes.contains(&0) {
            return Ok(());
        }
        let Ok(content) = std::str::from_utf8(&bytes) else {
            return Ok(());
        };
        for (index, line) in content.lines().enumerate() {
            for found in self.expression.find_iter(line) {
                let line_number = index as u64 + 1;
                let column = line[..found.start()].chars().count() as u64 + 1;
                let identity = (path.to_path_buf(), line_number, column);
                if self.seen.insert(identity) {
                    self.hits.push(GlobalSearchHit {
                        kind,
                        path: path.to_path_buf(),
                        line: line_number,
                        column,
                        preview: bounded_preview(line),
                        image: (kind == GlobalSearchContentKind::ImageRootfs)
                            .then(|| image_name_for_rootfs_path(path))
                            .flatten(),
                    });
                    let count = self.kind_counts.entry(kind).or_default();
                    *count += 1;
                    if *count >= MAX_HITS_PER_CONTENT_KIND {
                        self.truncated = true;
                        return Ok(());
                    }
                }
                if self.hits.len() >= MAX_GLOBAL_SEARCH_HITS {
                    self.truncated = true;
                    return Ok(());
                }
            }
        }
        Ok(())
    }
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

fn directory_entries(directory: &Path) -> Vec<fs::DirEntry> {
    let mut entries = fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .collect::<Vec<_>>();
    entries.sort_by_key(fs::DirEntry::file_name);
    entries
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

#[cfg(test)]
#[path = "tests/global_search/mod.rs"]
mod tests;
