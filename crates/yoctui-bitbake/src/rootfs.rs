use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use serde::de::{Deserializer as _, IgnoredAny, MapAccess, Visitor};
use thiserror::Error;
use yoctui_model::{
    ImageArtifactIdentity, MAX_ROOTFS_DEPTH, MAX_ROOTFS_ENTRIES, MAX_ROOTFS_PACKAGES,
    PackageIdentity, RootfsAuthority, RootfsComposition, RootfsCompositionRequest, RootfsEntry,
    RootfsEntryKind, RootfsFilesystemTree, RootfsInstalledPackage, RootfsPackageInventory,
    RootfsPathIdentity,
};

const ROOTFS_SCAN_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PKGDATA_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_PKGDATA_LINE_BYTES: usize = 8 * 1024 * 1024;
const MAX_PKGDATA_TOTAL_BYTES: u64 = 32 * 1024 * 1024;
const MAX_ROOTFS_ACCOUNTED_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
const MAX_LIMITATIONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsCompositionSources {
    pub image: ImageArtifactIdentity,
    pub manifest: Option<PathBuf>,
    pub pkgdata_directory: Option<PathBuf>,
    pub image_rootfs: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsCompositionResponse {
    pub request: RootfsCompositionRequest,
    pub composition: RootfsComposition,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RootfsCompositionAdapterError {
    #[error("invalid rootfs composition request: {0}")]
    InvalidRequest(String),
    #[error("rootfs composition source belongs to another image")]
    ImageMismatch,
    #[error("rootfs composition generation is stale: expected {expected}, got {actual}")]
    StaleGeneration { expected: u64, actual: u64 },
    #[error("rootfs composition build directory is unavailable: {0}")]
    BuildDirectory(PathBuf),
    #[error("rootfs composition source is invalid or is a symlink: {0}")]
    InvalidSource(PathBuf),
    #[error("rootfs composition source escapes the active build: {0}")]
    PathEscape(PathBuf),
    #[error("rootfs composition scan timed out after {0} seconds")]
    Timeout(u64),
    #[error("rootfs composition scan was cancelled")]
    Cancelled,
    #[error("rootfs composition safety bound reached: {0}")]
    ResourceLimit(String),
    #[error("rootfs composition I/O failed: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Default)]
pub struct RootfsCompositionCancellation {
    requested: Arc<AtomicBool>,
}

impl RootfsCompositionCancellation {
    pub fn cancel(&self) -> bool {
        !self.requested.swap(true, Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct RootfsCompositionAdapter {
    build_directory: PathBuf,
    sources: RootfsCompositionSources,
    expected_generation: u64,
    timeout: Duration,
}

impl RootfsCompositionAdapter {
    pub fn new(
        build_directory: PathBuf,
        sources: RootfsCompositionSources,
        expected_generation: u64,
    ) -> Self {
        Self {
            build_directory,
            sources,
            expected_generation,
            timeout: ROOTFS_SCAN_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn scan(
        &self,
        request: RootfsCompositionRequest,
    ) -> Result<RootfsCompositionResponse, RootfsCompositionAdapterError> {
        self.scan_with_cancellation(request, RootfsCompositionCancellation::default())
            .await
    }

    pub async fn scan_with_cancellation(
        &self,
        request: RootfsCompositionRequest,
        cancellation: RootfsCompositionCancellation,
    ) -> Result<RootfsCompositionResponse, RootfsCompositionAdapterError> {
        request
            .validate()
            .map_err(|message| RootfsCompositionAdapterError::InvalidRequest(message.into()))?;
        if request.generation != self.expected_generation {
            return Err(RootfsCompositionAdapterError::StaleGeneration {
                expected: self.expected_generation,
                actual: request.generation,
            });
        }
        if request.image != self.sources.image {
            return Err(RootfsCompositionAdapterError::ImageMismatch);
        }
        if cancellation.is_cancelled() {
            return Err(RootfsCompositionAdapterError::Cancelled);
        }
        let build_directory = self.build_directory.clone();
        let sources = self.sources.clone();
        let deadline = Instant::now() + self.timeout;
        let timeout = self.timeout;
        let worker = tokio::task::spawn_blocking(move || {
            scan_sources(request, build_directory, sources, cancellation, deadline)
        });
        match tokio::time::timeout(timeout, worker).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(RootfsCompositionAdapterError::Io(format!(
                "scan worker failed: {error}"
            ))),
            Err(_) => Err(RootfsCompositionAdapterError::Timeout(timeout.as_secs())),
        }
    }
}

fn scan_sources(
    request: RootfsCompositionRequest,
    build_directory: PathBuf,
    sources: RootfsCompositionSources,
    cancellation: RootfsCompositionCancellation,
    deadline: Instant,
) -> Result<RootfsCompositionResponse, RootfsCompositionAdapterError> {
    let build = canonical_directory(&build_directory, None)?;
    let mut limitations = Vec::new();
    let installed_packages = match sources.manifest {
        Some(manifest) if source_is_missing(&manifest)? => RootfsAuthority::Unavailable {
            reason: "the exact selected image manifest has been cleaned or is unavailable".into(),
        },
        Some(manifest) => scan_manifest(
            &build,
            &manifest,
            sources.pkgdata_directory.as_deref(),
            &cancellation,
            deadline,
            &mut limitations,
        )?,
        None => RootfsAuthority::Unavailable {
            reason: "the exact selected image manifest was not reported by BitBake".into(),
        },
    };
    let filesystem_tree = match sources.image_rootfs {
        Some(root) if source_is_missing(&root)? => RootfsAuthority::Unavailable {
            reason: "the BitBake-reported IMAGE_ROOTFS has been cleaned or is unavailable".into(),
        },
        Some(root) => scan_filesystem(&build, &root, &cancellation, deadline, &mut limitations)?,
        None => RootfsAuthority::Unavailable {
            reason: "IMAGE_ROOTFS was not reported for the selected image".into(),
        },
    };
    limitations.sort();
    limitations.dedup();
    if limitations.len() > MAX_LIMITATIONS {
        limitations.truncate(MAX_LIMITATIONS - 1);
        limitations.push(format!(
            "rootfs limitation reporting was capped at {MAX_LIMITATIONS} records"
        ));
    }
    Ok(RootfsCompositionResponse {
        request: request.clone(),
        composition: RootfsComposition {
            image: request.image,
            installed_packages,
            filesystem_tree,
        },
        limitations,
    })
}

fn scan_manifest(
    build: &Path,
    manifest_path: &Path,
    pkgdata_directory: Option<&Path>,
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<RootfsAuthority<RootfsPackageInventory>, RootfsCompositionAdapterError> {
    check_control(cancellation, deadline)?;
    let manifest = canonical_regular_file(manifest_path, build)?;
    let metadata = fs::metadata(&manifest)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Ok(RootfsAuthority::Unavailable {
            reason: format!(
                "the selected image manifest exceeds the {MAX_MANIFEST_BYTES}-byte safety bound"
            ),
        });
    }
    let text = fs::read_to_string(&manifest)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    let mut package_names = BTreeSet::new();
    let mut package_limitations = Vec::new();
    for line in text.lines() {
        check_control(cancellation, deadline)?;
        let Some(name) = line.split_whitespace().next() else {
            continue;
        };
        let identity = PackageIdentity::new(name);
        if identity.validate().is_err() {
            push_limitation(
                &mut package_limitations,
                format!("manifest contained an invalid package identity: {name}"),
            );
            continue;
        }
        if package_names.len() == MAX_ROOTFS_PACKAGES && !package_names.contains(name) {
            push_limitation(
                &mut package_limitations,
                format!("image manifest was limited to {MAX_ROOTFS_PACKAGES} packages"),
            );
            break;
        }
        package_names.insert(name.to_owned());
    }

    let pkgdata = match pkgdata_directory {
        Some(path) if source_is_missing(path)? => {
            push_limitation(
                &mut package_limitations,
                "PKGDATA_DIR has been cleaned; package metadata is unavailable".into(),
            );
            None
        }
        Some(path) => Some(canonical_directory(path, Some(build))?),
        None => {
            push_limitation(
                &mut package_limitations,
                "PKGDATA_DIR was not reported; package size, recipe, category, and file counts are unavailable".into(),
            );
            None
        }
    };
    let mut pkgdata_bytes = 0_u64;
    let mut packages = Vec::with_capacity(package_names.len());
    for name in package_names {
        check_control(cancellation, deadline)?;
        let mut package = RootfsInstalledPackage {
            identity: PackageIdentity::new(&name),
            recipe: None,
            category: "uncategorized".into(),
            installed_size_bytes: 0,
            file_count: 0,
        };
        if let Some(pkgdata) = &pkgdata {
            let candidate = pkgdata.join("runtime").join(&name);
            match read_pkgdata(candidate.as_path(), pkgdata, &name, &mut pkgdata_bytes) {
                Ok(values) => populate_package(&mut package, &values, &mut package_limitations),
                Err(RootfsCompositionAdapterError::InvalidSource(_)) => push_limitation(
                    &mut package_limitations,
                    format!("generated pkgdata was unavailable for installed package {name}"),
                ),
                Err(RootfsCompositionAdapterError::ResourceLimit(message)) => push_limitation(
                    &mut package_limitations,
                    format!(
                        "generated pkgdata was limited for installed package {name}: {message}"
                    ),
                ),
                Err(error) => return Err(error),
            }
        }
        packages.push(package);
    }
    limitations.extend(package_limitations.iter().cloned());
    let inventory = RootfsPackageInventory { packages };
    if package_limitations.is_empty() {
        Ok(RootfsAuthority::Available(inventory))
    } else {
        Ok(RootfsAuthority::Partial {
            value: inventory,
            limitations: package_limitations,
        })
    }
}

fn read_pkgdata(
    path: &Path,
    root: &Path,
    package_name: &str,
    total_bytes: &mut u64,
) -> Result<PkgdataValues, RootfsCompositionAdapterError> {
    let path = canonical_regular_file(path, root)?;
    let length = fs::metadata(&path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?
        .len();
    if length > MAX_PKGDATA_FILE_BYTES
        || total_bytes.saturating_add(length) > MAX_PKGDATA_TOTAL_BYTES
    {
        return Err(RootfsCompositionAdapterError::ResourceLimit(format!(
            "generated pkgdata exceeded its {MAX_PKGDATA_FILE_BYTES}-byte file or {MAX_PKGDATA_TOTAL_BYTES}-byte total bound"
        )));
    }
    *total_bytes += length;
    let file = fs::File::open(path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    let mut reader = BufReader::new(file);
    let mut bytes = Vec::new();
    let mut values = PkgdataValues::default();
    loop {
        bytes.clear();
        let read = reader
            .read_until(b'\n', &mut bytes)
            .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
        if read == 0 {
            break;
        }
        if bytes.len() > MAX_PKGDATA_LINE_BYTES {
            return Err(RootfsCompositionAdapterError::ResourceLimit(format!(
                "generated pkgdata contained a line over the {MAX_PKGDATA_LINE_BYTES}-byte bound"
            )));
        }
        let line = std::str::from_utf8(&bytes)
            .map_err(|_| {
                RootfsCompositionAdapterError::Io("generated pkgdata is not UTF-8".into())
            })?
            .trim_end_matches(['\r', '\n']);
        let Some((key, raw_value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "PN" => values.recipe = Some(raw_value.trim().to_owned()),
            "SECTION" => values.category = Some(raw_value.trim().to_owned()),
            "PKGSIZE" => {
                values.installed_size = scoped_pkgdata_value(raw_value, package_name)
                    .parse::<u64>()
                    .ok();
            }
            "FILES_INFO" => {
                values.file_count =
                    count_json_object_entries(scoped_pkgdata_value(raw_value, package_name));
                values.files_info_seen = true;
            }
            _ => {}
        }
    }
    Ok(values)
}

#[derive(Debug, Default, PartialEq, Eq)]
struct PkgdataValues {
    recipe: Option<String>,
    category: Option<String>,
    installed_size: Option<u64>,
    file_count: Option<u64>,
    files_info_seen: bool,
}

fn scoped_pkgdata_value<'a>(raw_value: &'a str, package_name: &str) -> &'a str {
    let value = raw_value.trim();
    value
        .strip_prefix(package_name)
        .and_then(|value| value.strip_prefix(':'))
        .map_or(value, str::trim)
}

fn count_json_object_entries(value: &str) -> Option<u64> {
    struct EntryCountVisitor;

    impl<'de> Visitor<'de> for EntryCountVisitor {
        type Value = u64;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a generated pkgdata FILES_INFO object")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut count = 0_u64;
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {
                count = count.saturating_add(1);
            }
            Ok(count)
        }
    }

    let mut deserializer = serde_json::Deserializer::from_str(value);
    let count = deserializer.deserialize_map(EntryCountVisitor).ok()?;
    deserializer.end().ok()?;
    Some(count)
}

fn populate_package(
    package: &mut RootfsInstalledPackage,
    values: &PkgdataValues,
    limitations: &mut Vec<String>,
) {
    package.recipe = values
        .recipe
        .as_ref()
        .filter(|value| valid_text(value))
        .cloned();
    package.category = values
        .category
        .as_ref()
        .filter(|value| valid_text(value) && !value.is_empty())
        .cloned()
        .or_else(|| package.recipe.clone())
        .unwrap_or_else(|| "uncategorized".into());
    match values.installed_size {
        Some(value) => package.installed_size_bytes = value,
        None => push_limitation(
            limitations,
            format!(
                "installed size was unavailable for package {}",
                package.identity.name
            ),
        ),
    }
    match (values.files_info_seen, values.file_count) {
        (_, Some(value)) => package.file_count = value,
        (true, None) => push_limitation(
            limitations,
            format!(
                "file count was malformed for package {}",
                package.identity.name
            ),
        ),
        (false, None) => push_limitation(
            limitations,
            format!(
                "file count was unavailable for package {}",
                package.identity.name
            ),
        ),
    }
}

fn scan_filesystem(
    build: &Path,
    image_rootfs: &Path,
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<RootfsAuthority<RootfsFilesystemTree>, RootfsCompositionAdapterError> {
    check_control(cancellation, deadline)?;
    let root = canonical_directory(image_rootfs, Some(build))?;
    let mut entries = Vec::new();
    let mut stack = vec![(root.clone(), PathBuf::from("/"), 0_usize)];
    let mut local_limitations = Vec::new();
    let mut accounted_bytes = 0_u64;
    #[cfg(unix)]
    let mut hardlinks = BTreeSet::<(u64, u64)>::new();

    while let Some((host_path, logical_path, depth)) = stack.pop() {
        check_control(cancellation, deadline)?;
        if entries.len() == MAX_ROOTFS_ENTRIES {
            push_limitation(
                &mut local_limitations,
                format!("filesystem traversal was limited to {MAX_ROOTFS_ENTRIES} entries"),
            );
            break;
        }
        let metadata = fs::symlink_metadata(&host_path)
            .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
        let kind = classify_file_type(&metadata.file_type());
        let mut size = if matches!(kind, RootfsEntryKind::Directory) {
            0
        } else {
            metadata.len()
        };
        #[cfg(unix)]
        if matches!(kind, RootfsEntryKind::RegularFile)
            && metadata.nlink() > 1
            && !hardlinks.insert((metadata.dev(), metadata.ino()))
        {
            size = 0;
        }
        if accounted_bytes.saturating_add(size) > MAX_ROOTFS_ACCOUNTED_BYTES {
            push_limitation(
                &mut local_limitations,
                format!(
                    "filesystem byte accounting was limited to {MAX_ROOTFS_ACCOUNTED_BYTES} bytes"
                ),
            );
            size = 0;
        } else {
            accounted_bytes += size;
        }
        entries.push(RootfsEntry {
            identity: RootfsPathIdentity(logical_path.clone()),
            kind,
            size_bytes: size,
            package: None,
        });

        if matches!(kind, RootfsEntryKind::Directory) {
            if depth == MAX_ROOTFS_DEPTH {
                push_limitation(
                    &mut local_limitations,
                    format!("filesystem traversal was limited to depth {MAX_ROOTFS_DEPTH}"),
                );
                continue;
            }
            let mut children = fs::read_dir(&host_path)
                .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
            children.sort_by_key(fs::DirEntry::file_name);
            for child in children.into_iter().rev() {
                let file_name = child.file_name();
                let Some(name) = file_name.to_str() else {
                    push_limitation(
                        &mut local_limitations,
                        "one filesystem entry had a non-UTF-8 name".into(),
                    );
                    continue;
                };
                let mut child_logical = logical_path.clone();
                child_logical.push(name);
                stack.push((child.path(), child_logical, depth + 1));
            }
        }
    }
    if !entries.is_empty() {
        push_limitation(
            &mut local_limitations,
            "filesystem package ownership is unavailable from IMAGE_ROOTFS traversal".into(),
        );
    }
    limitations.extend(local_limitations.iter().cloned());
    let tree = RootfsFilesystemTree { entries };
    if local_limitations.is_empty() {
        Ok(RootfsAuthority::Available(tree))
    } else {
        Ok(RootfsAuthority::Partial {
            value: tree,
            limitations: local_limitations,
        })
    }
}

fn canonical_directory(
    path: &Path,
    containment_root: Option<&Path>,
) -> Result<PathBuf, RootfsCompositionAdapterError> {
    if !path.is_absolute() {
        return Err(RootfsCompositionAdapterError::InvalidSource(path.into()));
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| {
        if containment_root.is_none() {
            RootfsCompositionAdapterError::BuildDirectory(path.into())
        } else {
            RootfsCompositionAdapterError::InvalidSource(path.into())
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(if containment_root.is_none() {
            RootfsCompositionAdapterError::BuildDirectory(path.into())
        } else {
            RootfsCompositionAdapterError::InvalidSource(path.into())
        });
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    if containment_root.is_some_and(|root| !canonical.starts_with(root) || canonical == root) {
        return Err(RootfsCompositionAdapterError::PathEscape(canonical));
    }
    Ok(canonical)
}

fn canonical_regular_file(
    path: &Path,
    containment_root: &Path,
) -> Result<PathBuf, RootfsCompositionAdapterError> {
    if !path.is_absolute() {
        return Err(RootfsCompositionAdapterError::InvalidSource(path.into()));
    }
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| RootfsCompositionAdapterError::InvalidSource(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(RootfsCompositionAdapterError::InvalidSource(path.into()));
    }
    let canonical = fs::canonicalize(path)
        .map_err(|error| RootfsCompositionAdapterError::Io(error.to_string()))?;
    if !canonical.starts_with(containment_root) || canonical == containment_root {
        return Err(RootfsCompositionAdapterError::PathEscape(canonical));
    }
    Ok(canonical)
}

fn source_is_missing(path: &Path) -> Result<bool, RootfsCompositionAdapterError> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(false),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(RootfsCompositionAdapterError::Io(error.to_string())),
    }
}

fn classify_file_type(file_type: &fs::FileType) -> RootfsEntryKind {
    if file_type.is_dir() {
        RootfsEntryKind::Directory
    } else if file_type.is_file() {
        RootfsEntryKind::RegularFile
    } else if file_type.is_symlink() {
        RootfsEntryKind::Symlink
    } else {
        RootfsEntryKind::Other
    }
}

fn check_control(
    cancellation: &RootfsCompositionCancellation,
    deadline: Instant,
) -> Result<(), RootfsCompositionAdapterError> {
    if cancellation.is_cancelled() {
        Err(RootfsCompositionAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(RootfsCompositionAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

fn push_limitation(limitations: &mut Vec<String>, limitation: String) {
    if limitations.len() < MAX_LIMITATIONS && !limitations.contains(&limitation) {
        limitations.push(limitation);
    }
}

fn valid_text(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    fn fixture() -> (PathBuf, RootfsCompositionRequest, RootfsCompositionSources) {
        let build = std::env::temp_dir().join(format!(
            "yoctui-rootfs-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let deploy = build.join("tmp/deploy/images/qemux86-64");
        let pkgdata = build.join("tmp/pkgdata/qemux86-64/runtime");
        let rootfs = build.join("tmp/work/qemux86-64/image/1.0-r0/rootfs");
        fs::create_dir_all(&deploy).unwrap();
        fs::create_dir_all(&pkgdata).unwrap();
        fs::create_dir_all(rootfs.join("usr/bin")).unwrap();
        let artifact = deploy.join("core-image-minimal.rootfs.ext4");
        fs::write(&artifact, b"image").unwrap();
        let manifest = deploy.join("core-image-minimal.rootfs.manifest");
        fs::write(
            &manifest,
            "busybox qemux86_64 1.0\nbase-files qemux86_64 1.0\n",
        )
        .unwrap();
        fs::write(
            pkgdata.join("busybox"),
            "PN: busybox\nSECTION: base\nPKGSIZE:busybox: 12\nFILES_INFO:busybox: {\"/usr/bin/busybox\":{}}\n",
        )
        .unwrap();
        fs::write(
            pkgdata.join("base-files"),
            "PN: base-files\nSECTION: base\nPKGSIZE: 5\nFILES_INFO: {\"/etc/os-release\":{},\"/etc/passwd\":{}}\n",
        )
        .unwrap();
        fs::write(rootfs.join("usr/bin/busybox"), b"busybox").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("busybox", rootfs.join("usr/bin/sh")).unwrap();
        let image = ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: artifact,
        };
        let request = RootfsCompositionRequest {
            generation: 4,
            image: image.clone(),
        };
        let sources = RootfsCompositionSources {
            image,
            manifest: Some(manifest),
            pkgdata_directory: Some(build.join("tmp/pkgdata/qemux86-64")),
            image_rootfs: Some(rootfs),
        };
        (build, request, sources)
    }

    #[tokio::test]
    async fn ux_rootfs_acquires_exact_manifest_pkgdata_and_no_follow_tree() {
        let (build, request, sources) = fixture();
        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request.clone())
            .await
            .unwrap();
        assert_eq!(response.request, request);
        let packages = response.composition.package_inventory().unwrap();
        assert_eq!(packages.packages.len(), 2);
        assert_eq!(packages.packages[0].identity.name, "base-files");
        assert_eq!(packages.packages[0].file_count, 2);
        assert_eq!(packages.packages[1].installed_size_bytes, 12);
        let entries = &response.composition.filesystem_tree().unwrap().entries;
        assert!(entries.iter().any(|entry| {
            entry.identity.0 == Path::new("/usr/bin/busybox")
                && entry.kind == RootfsEntryKind::RegularFile
                && entry.size_bytes == 7
        }));
        #[cfg(unix)]
        assert!(entries.iter().any(|entry| {
            entry.identity.0 == Path::new("/usr/bin/sh") && entry.kind == RootfsEntryKind::Symlink
        }));
        assert!(
            response
                .limitations
                .iter()
                .any(|value| value.contains("ownership"))
        );
        fs::remove_dir_all(build).unwrap();
    }

    #[test]
    fn ux_rootfs_streams_large_scoped_wrynose_pkgdata_and_counts_files() {
        let (build, _, sources) = fixture();
        let pkgdata = sources.pkgdata_directory.unwrap();
        let runtime = pkgdata.join("runtime");
        let files = (0..20_000)
            .map(|index| format!("\"/usr/src/kernel/file-{index}\":{{}}"))
            .collect::<Vec<_>>()
            .join(",");
        let content = format!(
            "PN: kernel-devsrc\nSECTION: kernel\nFILES_INFO:kernel-devsrc: {{{files}}}\nPKGSIZE:kernel-devsrc: 74306744\n"
        );
        assert!(content.len() > 256 * 1024);
        let path = runtime.join("kernel-devsrc");
        fs::write(&path, content).unwrap();

        let mut total = 0;
        let values = read_pkgdata(&path, &pkgdata, "kernel-devsrc", &mut total).unwrap();
        assert_eq!(values.recipe.as_deref(), Some("kernel-devsrc"));
        assert_eq!(values.category.as_deref(), Some("kernel"));
        assert_eq!(values.installed_size, Some(74_306_744));
        assert_eq!(values.file_count, Some(20_000));
        assert!(values.files_info_seen);
        assert_eq!(total, fs::metadata(path).unwrap().len());
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn ux_rootfs_oversized_single_pkgdata_is_partial_not_a_screen_failure() {
        let (build, request, mut sources) = fixture();
        let manifest = sources.manifest.as_ref().unwrap();
        fs::OpenOptions::new()
            .append(true)
            .open(manifest)
            .unwrap()
            .write_all(b"oversized qemux86_64 1.0\n")
            .unwrap();
        let oversized = sources
            .pkgdata_directory
            .as_ref()
            .unwrap()
            .join("runtime/oversized");
        fs::File::create(&oversized)
            .unwrap()
            .set_len(MAX_PKGDATA_FILE_BYTES + 1)
            .unwrap();
        sources.image_rootfs = None;

        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request)
            .await
            .unwrap();
        assert!(matches!(
            response.composition.installed_packages,
            RootfsAuthority::Partial { .. }
        ));
        assert!(
            response
                .limitations
                .iter()
                .any(|value| value.contains("oversized") && value.contains("limited"))
        );
        fs::remove_dir_all(build).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn ux_rootfs_deduplicates_hardlink_bytes_and_accounts_special_files() {
        use std::os::unix::net::UnixListener;

        let (build, request, sources) = fixture();
        let root = sources.image_rootfs.as_ref().unwrap();
        fs::hard_link(
            root.join("usr/bin/busybox"),
            root.join("usr/bin/busybox.link"),
        )
        .unwrap();
        let socket = root.join("run.sock");
        let _listener = UnixListener::bind(&socket).unwrap();
        let response = RootfsCompositionAdapter::new(build.clone(), sources, 4)
            .scan(request)
            .await
            .unwrap();
        let tree = response.composition.filesystem_tree().unwrap();
        let hardlink_bytes = tree
            .entries
            .iter()
            .filter(|entry| entry.identity.0.to_string_lossy().contains("busybox"))
            .map(|entry| entry.size_bytes)
            .sum::<u64>();
        assert_eq!(hardlink_bytes, 7);
        assert!(tree.entries.iter().any(|entry| {
            entry.identity.0 == Path::new("/run.sock") && entry.kind == RootfsEntryKind::Other
        }));
        drop(_listener);
        fs::remove_dir_all(build).unwrap();
    }

    #[tokio::test]
    async fn ux_rootfs_denies_stale_cancelled_mismatched_and_escaping_sources() {
        let (build, request, sources) = fixture();
        let adapter = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 5);
        assert!(matches!(
            adapter.scan(request.clone()).await,
            Err(RootfsCompositionAdapterError::StaleGeneration { .. })
        ));
        let cancellation = RootfsCompositionCancellation::default();
        cancellation.cancel();
        let adapter = RootfsCompositionAdapter::new(build.clone(), sources.clone(), 4);
        assert_eq!(
            adapter
                .scan_with_cancellation(request.clone(), cancellation)
                .await,
            Err(RootfsCompositionAdapterError::Cancelled)
        );
        let mut mismatch = request.clone();
        mismatch.image.image = "another-image".into();
        assert_eq!(
            adapter.scan(mismatch).await,
            Err(RootfsCompositionAdapterError::ImageMismatch)
        );
        let outside = std::env::temp_dir().join(format!("yoctui-outside-{}", std::process::id()));
        fs::create_dir_all(&outside).unwrap();
        let mut escaping = sources;
        escaping.image_rootfs = Some(outside.clone());
        let adapter = RootfsCompositionAdapter::new(build.clone(), escaping, 4);
        assert!(matches!(
            adapter.scan(request).await,
            Err(RootfsCompositionAdapterError::PathEscape(_))
        ));
        fs::remove_dir_all(build).unwrap();
        fs::remove_dir_all(outside).unwrap();
    }
}
