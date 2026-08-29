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
    App, BuildEnvironmentState, GlobalSearchContentKind, GlobalSearchHit, MAX_GLOBAL_SEARCH_HITS,
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
    pub source_roots: Vec<PathBuf>,
    pub layer_roots: Vec<PathBuf>,
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
        let profile_source = match &app.build_environment {
            BuildEnvironmentState::Configured(profile)
            | BuildEnvironmentState::Connected(profile)
            | BuildEnvironmentState::Failed { profile, .. }
            | BuildEnvironmentState::Verifying { profile, .. } => Some(profile.source_dir.clone()),
            BuildEnvironmentState::Unconfigured => None,
        };
        let mut source_roots = app
            .workspace
            .source_dir
            .iter()
            .cloned()
            .chain(profile_source)
            .collect::<Vec<_>>();
        let mut layer_roots = app
            .workspace
            .layers
            .iter()
            .map(|layer| layer.path.clone())
            .collect::<Vec<_>>();
        normalize_roots(&mut source_roots);
        normalize_roots(&mut layer_roots);
        for layer in &layer_roots {
            if !source_roots.iter().any(|source| layer.starts_with(source)) {
                source_roots.push(layer.clone());
            }
        }
        normalize_roots(&mut source_roots);
        let build_dir = safe_search_root(build_dir).then(|| build_dir.to_path_buf());
        Self {
            query,
            source_roots,
            layer_roots,
            build_dir,
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

fn normalize_roots(roots: &mut Vec<PathBuf>) {
    roots.retain(|root| safe_search_root(root));
    roots.sort();
    roots.dedup();
    let snapshot = roots.clone();
    roots.retain(|root| {
        !snapshot
            .iter()
            .any(|other| other != root && root.starts_with(other))
    });
}

pub fn scan_global_content(
    plan: &GlobalSearchPlan,
    cancellation: &GlobalSearchCancellation,
) -> Result<GlobalSearchScanResult, String> {
    let expression = RegexBuilder::new(plan.query.trim())
        .case_insensitive(true)
        .build()
        .map_err(|error| format!("invalid regular expression: {error}"))?;
    let mut scanner = Scanner {
        plan,
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
    for root in &plan.source_roots {
        scanner
            .searched_scopes
            .insert(format!("source={}", root.display()));
        scanner.walk_source(root)?;
        if scanner.done() {
            break;
        }
    }
    if !scanner.done()
        && let Some(build_dir) = &plan.build_dir
    {
        scanner.search_build(build_dir)?;
    }
    Ok(GlobalSearchScanResult {
        hits: scanner.hits,
        truncated: scanner.truncated,
        searched_scopes: scanner.searched_scopes.into_iter().collect(),
    })
}

struct Scanner<'a> {
    plan: &'a GlobalSearchPlan,
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

    fn walk_source(&mut self, root: &Path) -> Result<(), String> {
        self.walk_tree(root, None, &excluded_source_directory)
    }

    fn search_build(&mut self, build_dir: &Path) -> Result<(), String> {
        for (relative, kind) in [
            ("conf", GlobalSearchContentKind::Configuration),
            ("tmp/log", GlobalSearchContentKind::BuildLog),
            ("tmp/pkgdata", GlobalSearchContentKind::GeneratedMetadata),
            ("tmp/deploy", GlobalSearchContentKind::GeneratedMetadata),
        ] {
            let root = build_dir.join(relative);
            if root.is_dir() {
                self.searched_scopes
                    .insert(format!("{}={}", kind.label(), root.display()));
                self.walk_tree(&root, Some(kind), &|_, _| true)?;
            }
            if self.done() {
                return Ok(());
            }
        }
        let work = build_dir.join("tmp/work");
        if work.is_dir() {
            self.searched_scopes
                .insert(format!("generated-work={}", work.display()));
            self.discover_work_outputs(&work)?;
        }
        Ok(())
    }

    fn discover_work_outputs(&mut self, directory: &Path) -> Result<(), String> {
        if self.done() {
            return Ok(());
        }
        self.count_directory()?;
        for entry in directory_entries(directory) {
            if self.done() {
                break;
            }
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_symlink() || !kind.is_dir() {
                continue;
            }
            match entry.file_name().to_str() {
                Some("rootfs") => {
                    self.walk_tree(
                        &path,
                        Some(GlobalSearchContentKind::ImageRootfs),
                        &|_, _| true,
                    )?;
                }
                Some("temp") => self.search_task_logs(&path)?,
                Some(name) if excluded_build_directory_name(name) => {}
                _ => self.discover_work_outputs(&path)?,
            }
        }
        Ok(())
    }

    fn search_task_logs(&mut self, directory: &Path) -> Result<(), String> {
        self.count_directory()?;
        for entry in directory_entries(directory) {
            if self.done() {
                break;
            }
            let path = entry.path();
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if entry.file_type().is_ok_and(|kind| kind.is_file())
                && (name.starts_with("log.") || name.starts_with("run."))
            {
                self.search_file(&path, GlobalSearchContentKind::BuildLog)?;
            }
        }
        Ok(())
    }

    fn walk_tree(
        &mut self,
        directory: &Path,
        forced_kind: Option<GlobalSearchContentKind>,
        descend: &dyn Fn(&Path, &str) -> bool,
    ) -> Result<(), String> {
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
                if descend(&path, &name) {
                    self.walk_tree(&path, forced_kind, descend)?;
                }
            } else if file_type.is_file() {
                let kind = forced_kind.unwrap_or_else(|| self.classify_source(&path));
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

    fn classify_source(&self, path: &Path) -> GlobalSearchContentKind {
        match path.extension().and_then(|extension| extension.to_str()) {
            Some("bb" | "bbappend" | "inc") => GlobalSearchContentKind::Recipe,
            Some("conf") => GlobalSearchContentKind::Configuration,
            Some("bbclass") => GlobalSearchContentKind::Class,
            _ if self
                .plan
                .layer_roots
                .iter()
                .any(|root| path.starts_with(root)) =>
            {
                GlobalSearchContentKind::LayerSource
            }
            _ => GlobalSearchContentKind::PokyBitBakeSource,
        }
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
        if bytes.iter().take(8_192).any(|byte| *byte == 0) {
            return Ok(());
        }
        let content = String::from_utf8_lossy(&bytes);
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

fn excluded_source_directory(_path: &Path, name: &str) -> bool {
    !matches!(
        name,
        ".git"
            | ".repo"
            | "target"
            | "tmp"
            | "cache"
            | "downloads"
            | "sstate-cache"
            | "__pycache__"
            | "node_modules"
    )
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

fn excluded_build_directory_name(name: &str) -> bool {
    matches!(
        name,
        "recipe-sysroot"
            | "recipe-sysroot-native"
            | "sysroot-destdir"
            | "packages-split"
            | "package"
            | "pseudo"
            | "build"
    )
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
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "yoctui-global-search-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn unified_search_finds_metadata_sources_logs_and_generated_image_files() {
        let root = fixture_root();
        let poky = root.join("poky");
        let layer = poky.join("meta-demo");
        let build = root.join("build");
        let rootfs = build.join("tmp/work/qemux86-64-poky-linux/core-image-demo/1.0/rootfs");
        fs::create_dir_all(layer.join("recipes-core/demo")).unwrap();
        fs::create_dir_all(layer.join("conf")).unwrap();
        fs::create_dir_all(layer.join("classes")).unwrap();
        fs::create_dir_all(poky.join("bitbake/lib/bb")).unwrap();
        fs::create_dir_all(build.join("tmp/work/x/demo/1.0/temp")).unwrap();
        fs::create_dir_all(build.join("tmp/deploy/images/qemux86-64")).unwrap();
        fs::create_dir_all(rootfs.join("usr/lib/systemd/system")).unwrap();
        fs::write(
            layer.join("recipes-core/demo/demo.bb"),
            "SEARCH_TOKEN recipe\n",
        )
        .unwrap();
        fs::write(layer.join("conf/layer.conf"), "SEARCH_TOKEN conf\n").unwrap();
        fs::write(layer.join("classes/demo.bbclass"), "SEARCH_TOKEN class\n").unwrap();
        fs::write(poky.join("bitbake/lib/bb/demo.py"), "SEARCH_TOKEN source\n").unwrap();
        fs::write(
            poky.join("bitbake/lib/bb/many.py"),
            "SEARCH_TOKEN repeated source\n".repeat(70),
        )
        .unwrap();
        fs::write(layer.join("setup-demo.sh"), "SEARCH_TOKEN layer script\n").unwrap();
        fs::write(
            build.join("tmp/work/x/demo/1.0/temp/log.do_compile"),
            "SEARCH_TOKEN log\n",
        )
        .unwrap();
        fs::write(
            build.join("tmp/deploy/images/qemux86-64/core-image-demo.manifest"),
            "SEARCH_TOKEN generated metadata\n",
        )
        .unwrap();
        fs::write(
            rootfs.join("usr/lib/systemd/system/demo.service"),
            "Description=SEARCH_TOKEN service\n",
        )
        .unwrap();
        let plan = GlobalSearchPlan {
            query: "search_token".into(),
            source_roots: vec![poky],
            layer_roots: vec![layer],
            build_dir: Some(build),
        };
        let result = scan_global_content(&plan, &GlobalSearchCancellation::default()).unwrap();
        assert!(result.truncated, "per-kind bounds must be disclosed");
        for kind in [
            GlobalSearchContentKind::Recipe,
            GlobalSearchContentKind::Configuration,
            GlobalSearchContentKind::Class,
            GlobalSearchContentKind::LayerSource,
            GlobalSearchContentKind::PokyBitBakeSource,
            GlobalSearchContentKind::BuildLog,
            GlobalSearchContentKind::GeneratedMetadata,
            GlobalSearchContentKind::ImageRootfs,
        ] {
            assert!(result.hits.iter().any(|hit| hit.kind == kind), "{kind:?}");
        }
        let service = result
            .hits
            .iter()
            .find(|hit| hit.path.ends_with("demo.service"))
            .unwrap();
        assert_eq!(service.image.as_deref(), Some("core-image-demo"));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn search_rejects_root_and_prunes_generated_caches() {
        assert!(!safe_search_root(Path::new("/")));
        assert!(!excluded_source_directory(Path::new("/x/.git"), ".git"));
        assert!(!excluded_source_directory(Path::new("/x/tmp"), "tmp"));
        assert!(excluded_source_directory(
            Path::new("/x/scripts"),
            "scripts"
        ));
    }
}
