use std::{
    collections::{BTreeSet, VecDeque},
    env, fs, io,
    path::{Path, PathBuf},
};
use thiserror::Error;
use yoctui_model::{PlatformFile, PlatformFileKind};

const MAX_FILES: usize = 4096;
const MAX_DIRECTORIES: usize = 16_384;
const MAX_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformArtifactScan {
    pub roots: Vec<PathBuf>,
    pub files: Vec<PlatformFile>,
    pub dtc: Option<PathBuf>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error)]
pub enum PlatformArtifactAdapterError {
    #[error("no absolute platform artifact directory exists")]
    NoRoots,
    #[error("platform artifact scan failed for {path}: {source}")]
    Scan { path: PathBuf, source: io::Error },
}

#[derive(Debug, Clone, Default)]
pub struct PlatformArtifactAdapter;

impl PlatformArtifactAdapter {
    pub fn scan(
        &self,
        candidates: Vec<PathBuf>,
    ) -> Result<PlatformArtifactScan, PlatformArtifactAdapterError> {
        let mut roots = candidates
            .into_iter()
            .filter(|path| path.is_absolute() && path.is_dir())
            .filter_map(|path| path.canonicalize().ok())
            .collect::<Vec<_>>();
        roots.sort();
        roots.dedup();
        if roots.is_empty() {
            return Err(PlatformArtifactAdapterError::NoRoots);
        }

        let mut files = Vec::new();
        let mut limitations = Vec::new();
        let mut directories = 0usize;
        for root in &roots {
            let mut pending = VecDeque::from([(root.clone(), 0usize)]);
            while let Some((directory, depth)) = pending.pop_front() {
                if directories >= MAX_DIRECTORIES || files.len() >= MAX_FILES {
                    limitations.push(format!(
                        "Scan stopped at {MAX_FILES} files or {MAX_DIRECTORIES} directories."
                    ));
                    break;
                }
                directories += 1;
                let entries = fs::read_dir(&directory).map_err(|source| {
                    PlatformArtifactAdapterError::Scan {
                        path: directory.clone(),
                        source,
                    }
                })?;
                for entry in entries.flatten() {
                    let Ok(file_type) = entry.file_type() else {
                        continue;
                    };
                    let path = entry.path();
                    if file_type.is_symlink() {
                        continue;
                    }
                    if file_type.is_dir() {
                        if depth < MAX_DEPTH && entry.file_name() != ".git" {
                            pending.push_back((path, depth + 1));
                        }
                        continue;
                    }
                    if !file_type.is_file() {
                        continue;
                    }
                    let Some(kind) = classify(&path) else {
                        continue;
                    };
                    let size_bytes = entry.metadata().map(|value| value.len()).unwrap_or(0);
                    files.push(PlatformFile {
                        path,
                        root: root.clone(),
                        kind,
                        size_bytes,
                    });
                    if files.len() >= MAX_FILES {
                        break;
                    }
                }
            }
        }
        files.sort_by(|left, right| left.path.cmp(&right.path));
        files.dedup_by(|left, right| left.path == right.path);
        if files.is_empty() {
            limitations.push(
                "No .config, DTS, DTSI, DTB, or DTBO files were found in the authoritative roots."
                    .into(),
            );
        }
        if roots.len() > 1 {
            let mut nested = BTreeSet::new();
            for root in &roots {
                if roots
                    .iter()
                    .any(|other| other != root && root.starts_with(other))
                {
                    nested.insert(root.clone());
                }
            }
            if !nested.is_empty() {
                limitations.push(
                    "Overlapping source/build roots were de-duplicated by exact file path.".into(),
                );
            }
        }
        Ok(PlatformArtifactScan {
            roots,
            files,
            dtc: resolve_executable("dtc"),
            limitations,
        })
    }
}

fn classify(path: &Path) -> Option<PlatformFileKind> {
    if path.file_name().is_some_and(|name| name == ".config") {
        return Some(PlatformFileKind::DotConfig);
    }
    match path.extension().and_then(|value| value.to_str()) {
        Some("dts") => Some(PlatformFileKind::Dts),
        Some("dtsi") => Some(PlatformFileKind::Dtsi),
        Some("dtb") => Some(PlatformFileKind::Dtb),
        Some("dtbo") => Some(PlatformFileKind::Dtbo),
        _ => None,
    }
}

fn resolve_executable(name: &str) -> Option<PathBuf> {
    env::var_os("PATH")
        .into_iter()
        .flat_map(|value| env::split_paths(&value).collect::<Vec<_>>())
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
        .and_then(|path| path.canonicalize().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scans_only_supported_platform_artifacts_without_following_symlinks() {
        let root = std::env::temp_dir().join(format!("yoctui-platform-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("arch/arm/boot/dts")).unwrap();
        fs::write(root.join(".config"), "CONFIG_TEST=y\n").unwrap();
        fs::write(root.join("arch/arm/boot/dts/board.dts"), "/dts-v1/;\n").unwrap();
        fs::write(root.join("ignored.txt"), "ignored\n").unwrap();
        let scan = PlatformArtifactAdapter.scan(vec![root.clone()]).unwrap();
        assert_eq!(scan.files.len(), 2);
        assert!(
            scan.files
                .iter()
                .any(|file| file.kind == PlatformFileKind::DotConfig)
        );
        assert!(
            scan.files
                .iter()
                .any(|file| file.kind == PlatformFileKind::Dts)
        );
        fs::remove_dir_all(root).unwrap();
    }
}
