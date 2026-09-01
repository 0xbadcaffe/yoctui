use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, UNIX_EPOCH},
};

use thiserror::Error;
use yoctui_model::{
    ImageArtifact, ImageArtifactField, ImageArtifactIdentity, ImageArtifactInventory,
    ImageArtifactKind, ImageArtifactRequest, ImageChecksum,
};

const MAX_DEPLOY_ENTRIES: usize = 8_192;
const MAX_CHECKSUM_FILE_BYTES: u64 = 1024 * 1024;
const MAX_CHECKSUM_TOTAL_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CHECKSUM_LINES: usize = 4_096;
const IMAGE_ARTIFACT_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageArtifactResponse {
    pub request: ImageArtifactRequest,
    pub inventory: ImageArtifactInventory,
    pub limitations: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ImageArtifactAdapterError {
    #[error("invalid image artifact request: {0}")]
    InvalidRequest(String),
    #[error("configured DEPLOY_DIR_IMAGE must be an absolute directory: {0}")]
    InvalidDeployDirectory(PathBuf),
    #[error("configured DEPLOY_DIR_IMAGE must not be a symlink: {0}")]
    SymlinkDeployDirectory(PathBuf),
    #[error("configured DEPLOY_DIR_IMAGE does not match machine {machine}: {path}")]
    MachineMismatch { machine: String, path: PathBuf },
    #[error("image artifact scan timed out after {0} seconds")]
    Timeout(u64),
    #[error("image artifact scan was cancelled")]
    Cancelled,
    #[error("image artifact I/O failed: {0}")]
    Io(String),
}

#[derive(Debug, Clone, Default)]
pub struct ImageArtifactCancellation {
    requested: Arc<AtomicBool>,
}

impl ImageArtifactCancellation {
    pub fn cancel(&self) -> bool {
        !self.requested.swap(true, Ordering::SeqCst)
    }

    pub fn is_cancelled(&self) -> bool {
        self.requested.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone)]
pub struct ImageArtifactAdapter {
    deploy_directory: PathBuf,
    timeout: Duration,
}

impl ImageArtifactAdapter {
    pub fn new(deploy_directory: PathBuf) -> Self {
        Self {
            deploy_directory,
            timeout: IMAGE_ARTIFACT_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn scan(
        &self,
        request: ImageArtifactRequest,
    ) -> Result<ImageArtifactResponse, ImageArtifactAdapterError> {
        self.scan_with_cancellation(request, ImageArtifactCancellation::default())
            .await
    }

    pub async fn scan_with_cancellation(
        &self,
        request: ImageArtifactRequest,
        cancellation: ImageArtifactCancellation,
    ) -> Result<ImageArtifactResponse, ImageArtifactAdapterError> {
        request
            .validate()
            .map_err(|message| ImageArtifactAdapterError::InvalidRequest(message.into()))?;
        if cancellation.is_cancelled() {
            return Err(ImageArtifactAdapterError::Cancelled);
        }
        let deploy_directory = self.deploy_directory.clone();
        let deadline = Instant::now() + self.timeout;
        let task = tokio::task::spawn_blocking(move || {
            scan_deploy_directory(request, deploy_directory, cancellation, deadline)
        });
        match tokio::time::timeout(self.timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => Err(ImageArtifactAdapterError::Io(format!(
                "scan worker failed: {error}"
            ))),
            Err(_) => Err(ImageArtifactAdapterError::Timeout(self.timeout.as_secs())),
        }
    }
}

fn scan_deploy_directory(
    request: ImageArtifactRequest,
    deploy_directory: PathBuf,
    cancellation: ImageArtifactCancellation,
    deadline: Instant,
) -> Result<ImageArtifactResponse, ImageArtifactAdapterError> {
    if !deploy_directory.is_absolute() {
        return Err(ImageArtifactAdapterError::InvalidDeployDirectory(
            deploy_directory,
        ));
    }
    let root_metadata = fs::symlink_metadata(&deploy_directory)
        .map_err(|_| ImageArtifactAdapterError::InvalidDeployDirectory(deploy_directory.clone()))?;
    if root_metadata.file_type().is_symlink() {
        return Err(ImageArtifactAdapterError::SymlinkDeployDirectory(
            deploy_directory,
        ));
    }
    if !root_metadata.is_dir() {
        return Err(ImageArtifactAdapterError::InvalidDeployDirectory(
            deploy_directory,
        ));
    }
    let root = fs::canonicalize(&deploy_directory)
        .map_err(|error| ImageArtifactAdapterError::Io(error.to_string()))?;
    if root.file_name().and_then(|name| name.to_str()) != Some(request.machine.as_str()) {
        return Err(ImageArtifactAdapterError::MachineMismatch {
            machine: request.machine,
            path: root,
        });
    }
    check_scan_control(&cancellation, deadline)?;

    let mut limitations = Vec::new();
    let mut files = Vec::new();
    for (index, entry) in fs::read_dir(&root)
        .map_err(|error| ImageArtifactAdapterError::Io(error.to_string()))?
        .enumerate()
    {
        check_scan_control(&cancellation, deadline)?;
        if index >= MAX_DEPLOY_ENTRIES {
            limitations.push(format!(
                "deploy scan was limited to {MAX_DEPLOY_ENTRIES} entries"
            ));
            break;
        }
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                limitations.push(format!("one deploy entry was unreadable: {error}"));
                continue;
            }
        };
        let path = entry.path();
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                limitations.push(format!(
                    "metadata was unavailable for {}: {error}",
                    path.display()
                ));
                continue;
            }
        };
        if metadata.file_type().is_symlink() {
            limitations.push(format!(
                "symlink deploy entry was not followed: {}",
                path.display()
            ));
            continue;
        }
        if metadata.is_dir() {
            limitations.push(format!(
                "nested deploy directory was not traversed at depth limit 1: {}",
                path.display()
            ));
            continue;
        }
        if !metadata.is_file() {
            limitations.push(format!(
                "non-regular deploy entry was ignored: {}",
                path.display()
            ));
            continue;
        }
        let canonical = fs::canonicalize(&path)
            .map_err(|error| ImageArtifactAdapterError::Io(error.to_string()))?;
        if canonical.parent() != Some(root.as_path()) {
            return Err(ImageArtifactAdapterError::Io(format!(
                "deploy entry escaped configured directory: {}",
                canonical.display()
            )));
        }
        files.push((canonical, metadata));
    }
    files.sort_by(|left, right| left.0.cmp(&right.0));

    let mut artifacts = files
        .iter()
        .map(|(path, metadata)| {
            let kind = classify(path);
            ImageArtifact {
                identity: ImageArtifactIdentity {
                    machine: request.machine.clone(),
                    image: image_target(path, &request.machine, kind),
                    path: path.clone(),
                },
                kind,
                size_bytes: ImageArtifactField::Available(metadata.len()),
                modified_unix_seconds: metadata
                    .modified()
                    .ok()
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| ImageArtifactField::Available(duration.as_secs()))
                    .unwrap_or(ImageArtifactField::Unavailable),
                checksums: ImageArtifactField::Unavailable,
                manifests: ImageArtifactField::Unavailable,
                licenses: ImageArtifactField::Unavailable,
                spdx: ImageArtifactField::Unavailable,
                wic_files: ImageArtifactField::Unavailable,
            }
        })
        .collect::<Vec<_>>();

    associate_typed_files(&mut artifacts);
    parse_checksums(
        &mut artifacts,
        &files,
        &root,
        &cancellation,
        deadline,
        &mut limitations,
    )?;
    limitations.sort();
    limitations.dedup();
    Ok(ImageArtifactResponse {
        request: request.clone(),
        inventory: ImageArtifactInventory {
            machine: request.machine,
            deploy_directory: ImageArtifactField::Available(root),
            artifacts,
        },
        limitations,
    })
}

fn check_scan_control(
    cancellation: &ImageArtifactCancellation,
    deadline: Instant,
) -> Result<(), ImageArtifactAdapterError> {
    if cancellation.is_cancelled() {
        Err(ImageArtifactAdapterError::Cancelled)
    } else if Instant::now() >= deadline {
        Err(ImageArtifactAdapterError::Timeout(0))
    } else {
        Ok(())
    }
}

fn classify(path: &Path) -> ImageArtifactKind {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.ends_with(".sha256")
        || name.ends_with(".sha256sum")
        || name.ends_with(".sha512")
        || name.ends_with(".md5")
    {
        ImageArtifactKind::Checksum
    } else if name.contains("spdx") || name.contains("sbom") {
        ImageArtifactKind::Spdx
    } else if name.contains("license") {
        ImageArtifactKind::LicenseManifest
    } else if name.ends_with(".manifest") {
        ImageArtifactKind::Manifest
    } else if name.ends_with(".wic") || name.ends_with(".direct") {
        ImageArtifactKind::Wic
    } else if name.starts_with("bzimage")
        || name.starts_with("vmlinuz")
        || name.starts_with("vmlinux")
        || name.starts_with("image-")
    {
        ImageArtifactKind::Kernel
    } else if name.starts_with("u-boot") || name.starts_with("grub") {
        ImageArtifactKind::Bootloader
    } else if name.contains(".rootfs.")
        || [
            ".ext4",
            ".tar",
            ".tar.gz",
            ".tar.xz",
            ".cpio",
            ".squashfs",
            ".jffs2",
            ".ubi",
        ]
        .iter()
        .any(|suffix| name.ends_with(suffix))
    {
        ImageArtifactKind::RootFilesystem
    } else {
        ImageArtifactKind::Other
    }
}

fn image_target(path: &Path, machine: &str, kind: ImageArtifactKind) -> String {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown");
    match kind {
        ImageArtifactKind::Kernel => return "kernel".into(),
        ImageArtifactKind::Bootloader => return "bootloader".into(),
        _ => {}
    }
    let marker = format!("-{machine}");
    if let Some(index) = name.find(&marker)
        && index > 0
    {
        return name[..index].to_owned();
    }
    name.split_once('.')
        .map_or(name, |(stem, _)| stem)
        .to_owned()
}

fn associate_typed_files(artifacts: &mut [ImageArtifact]) {
    let mut grouped: BTreeMap<String, Vec<(ImageArtifactKind, PathBuf)>> = BTreeMap::new();
    for artifact in artifacts.iter() {
        grouped
            .entry(artifact.identity.image.clone())
            .or_default()
            .push((artifact.kind, artifact.identity.path.clone()));
    }
    for artifact in artifacts {
        let Some(files) = grouped.get(&artifact.identity.image) else {
            continue;
        };
        artifact.manifests = associated(files, ImageArtifactKind::Manifest);
        artifact.licenses = associated(files, ImageArtifactKind::LicenseManifest);
        artifact.spdx = associated(files, ImageArtifactKind::Spdx);
        artifact.wic_files = associated(files, ImageArtifactKind::Wic);
    }
}

fn associated(
    files: &[(ImageArtifactKind, PathBuf)],
    kind: ImageArtifactKind,
) -> ImageArtifactField<Vec<PathBuf>> {
    let paths = files
        .iter()
        .filter_map(|(candidate, path)| (*candidate == kind).then_some(path.clone()))
        .collect::<Vec<_>>();
    if paths.is_empty() {
        ImageArtifactField::Unavailable
    } else {
        ImageArtifactField::Available(paths)
    }
}

fn parse_checksums(
    artifacts: &mut [ImageArtifact],
    files: &[(PathBuf, fs::Metadata)],
    root: &Path,
    cancellation: &ImageArtifactCancellation,
    deadline: Instant,
    limitations: &mut Vec<String>,
) -> Result<(), ImageArtifactAdapterError> {
    let by_path = artifacts
        .iter()
        .enumerate()
        .map(|(index, artifact)| (artifact.identity.path.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut checksum_bytes = 0_u64;
    for (path, metadata) in files
        .iter()
        .filter(|(path, _)| classify(path) == ImageArtifactKind::Checksum)
    {
        check_scan_control(cancellation, deadline)?;
        if metadata.len() > MAX_CHECKSUM_FILE_BYTES
            || checksum_bytes.saturating_add(metadata.len()) > MAX_CHECKSUM_TOTAL_BYTES
        {
            limitations.push(format!(
                "checksum file exceeded scan bounds: {}",
                path.display()
            ));
            continue;
        }
        checksum_bytes += metadata.len();
        let bytes =
            fs::read(path).map_err(|error| ImageArtifactAdapterError::Io(error.to_string()))?;
        let Ok(text) = std::str::from_utf8(&bytes) else {
            limitations.push(format!("checksum file was not UTF-8: {}", path.display()));
            continue;
        };
        let algorithm = path
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("checksum")
            .trim_end_matches("sum")
            .to_owned();
        for (line_index, line) in text.lines().enumerate() {
            if line_index >= MAX_CHECKSUM_LINES {
                limitations.push(format!("checksum lines were truncated: {}", path.display()));
                break;
            }
            let mut fields = line.split_whitespace();
            let Some(digest) = fields.next() else {
                continue;
            };
            let Some(file_name) = fields.next().map(|name| name.trim_start_matches('*')) else {
                limitations.push(format!("malformed checksum record in {}", path.display()));
                continue;
            };
            if fields.next().is_some()
                || digest.is_empty()
                || !digest
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                limitations.push(format!("malformed checksum record in {}", path.display()));
                continue;
            }
            let source = root.join(file_name);
            if source.parent() != Some(root) {
                limitations.push(format!(
                    "checksum record escaped deploy directory in {}",
                    path.display()
                ));
                continue;
            }
            let Ok(source) = fs::canonicalize(&source) else {
                limitations.push(format!(
                    "checksum record referenced a missing file in {}",
                    path.display()
                ));
                continue;
            };
            let Some(index) = by_path.get(&source).copied() else {
                limitations.push(format!(
                    "checksum record was not associated in {}",
                    path.display()
                ));
                continue;
            };
            let checksum = ImageChecksum {
                algorithm: algorithm.clone(),
                digest: digest.to_owned(),
                source: path.clone(),
            };
            match &mut artifacts[index].checksums {
                ImageArtifactField::Unavailable => {
                    artifacts[index].checksums = ImageArtifactField::Available(vec![checksum]);
                }
                ImageArtifactField::Available(checksums) => checksums.push(checksum),
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::File,
        io::Write,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "yoctui-image-artifacts-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn fixture() -> TestDirectory {
        TestDirectory::new()
    }

    fn request() -> ImageArtifactRequest {
        ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        }
    }

    #[test]
    fn image_artifact_adapter_classifies_only_uncompressed_wic_images() {
        assert_eq!(
            classify(Path::new("/deploy/image.wic")),
            ImageArtifactKind::Wic
        );
        assert_eq!(
            classify(Path::new("/deploy/image.direct")),
            ImageArtifactKind::Wic
        );
        assert_eq!(
            classify(Path::new("/deploy/image.wic.gz")),
            ImageArtifactKind::Other
        );
    }

    #[test]
    fn boot_artifact_identity_never_becomes_a_bitbake_recipe_target() {
        assert_eq!(
            image_target(
                Path::new("/deploy/bzImage--6.18.24+git0-r0-qemux86-64-20260831042030.bin"),
                "qemux86-64",
                ImageArtifactKind::Kernel,
            ),
            "kernel"
        );
        assert_eq!(
            image_target(
                Path::new("/deploy/u-boot-qemux86-64-20260831042030.bin"),
                "qemux86-64",
                ImageArtifactKind::Bootloader,
            ),
            "bootloader"
        );
    }

    #[tokio::test]
    async fn image_artifact_adapter_scans_and_classifies_deterministically() {
        let fixture = fixture();
        let deploy = fixture.path().join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        fs::write(
            deploy.join("core-image-minimal-qemux86-64.rootfs.ext4"),
            b"rootfs",
        )
        .unwrap();
        fs::write(
            deploy.join("core-image-minimal-qemux86-64.manifest"),
            b"busybox",
        )
        .unwrap();
        fs::write(deploy.join("core-image-minimal-qemux86-64.wic"), b"wic").unwrap();
        fs::write(
            deploy.join("core-image-minimal-qemux86-64.rootfs.ext4.sha256"),
            b"abcdef  core-image-minimal-qemux86-64.rootfs.ext4\n",
        )
        .unwrap();

        let response = ImageArtifactAdapter::new(deploy.clone())
            .scan(request())
            .await
            .unwrap();
        let paths = response
            .inventory
            .artifacts
            .iter()
            .map(|artifact| artifact.identity.path.clone())
            .collect::<Vec<_>>();
        assert!(paths.windows(2).all(|pair| pair[0] < pair[1]));
        let rootfs = response
            .inventory
            .artifacts
            .iter()
            .find(|artifact| artifact.kind == ImageArtifactKind::RootFilesystem)
            .unwrap();
        assert_eq!(rootfs.identity.image, "core-image-minimal");
        assert!(matches!(
            rootfs.checksums,
            ImageArtifactField::Available(ref checksums) if checksums.len() == 1
        ));
        assert!(matches!(
            rootfs.manifests,
            ImageArtifactField::Available(ref paths) if paths.len() == 1
        ));
        assert!(matches!(
            rootfs.wic_files,
            ImageArtifactField::Available(ref paths) if paths.len() == 1
        ));
    }

    #[tokio::test]
    async fn image_artifact_adapter_reports_empty_partial_malformed_and_oversized_inputs() {
        let fixture = fixture();
        let deploy = fixture.path().join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let empty = ImageArtifactAdapter::new(deploy.clone())
            .scan(request())
            .await
            .unwrap();
        assert!(empty.inventory.artifacts.is_empty());
        assert!(empty.limitations.is_empty());

        fs::write(deploy.join("image-qemux86-64.ext4"), b"image").unwrap();
        fs::write(deploy.join("image-qemux86-64.ext4.sha256"), b"not-a-record").unwrap();
        let mut oversized = File::create(deploy.join("image-qemux86-64.ext4.md5")).unwrap();
        oversized.set_len(MAX_CHECKSUM_FILE_BYTES + 1).unwrap();
        oversized.flush().unwrap();
        fs::create_dir(deploy.join("nested")).unwrap();
        let partial = ImageArtifactAdapter::new(deploy)
            .scan(request())
            .await
            .unwrap();
        assert!(
            partial
                .limitations
                .iter()
                .any(|message| message.contains("malformed checksum"))
        );
        assert!(
            partial
                .limitations
                .iter()
                .any(|message| message.contains("exceeded scan bounds"))
        );
        assert!(
            partial
                .limitations
                .iter()
                .any(|message| message.contains("depth limit"))
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn image_artifact_adapter_rejects_symlink_missing_escape_and_machine_mismatch() {
        use std::os::unix::fs::symlink;

        let fixture = fixture();
        let missing = fixture.path().join("qemux86-64");
        assert!(matches!(
            ImageArtifactAdapter::new(missing).scan(request()).await,
            Err(ImageArtifactAdapterError::InvalidDeployDirectory(_))
        ));

        let real = fixture.path().join("real");
        fs::create_dir(&real).unwrap();
        let linked = fixture.path().join("qemux86-64");
        symlink(&real, &linked).unwrap();
        assert!(matches!(
            ImageArtifactAdapter::new(linked).scan(request()).await,
            Err(ImageArtifactAdapterError::SymlinkDeployDirectory(_))
        ));

        let wrong = fixture.path().join("qemuarm64");
        fs::create_dir(&wrong).unwrap();
        assert!(matches!(
            ImageArtifactAdapter::new(wrong).scan(request()).await,
            Err(ImageArtifactAdapterError::MachineMismatch { .. })
        ));

        let deploy = fixture.path().join("machine");
        fs::create_dir(&deploy).unwrap();
        let outside = fixture.path().join("outside");
        fs::write(&outside, b"outside").unwrap();
        symlink(&outside, deploy.join("escaped")).unwrap();
        let mut machine_request = request();
        machine_request.machine = "machine".into();
        let response = ImageArtifactAdapter::new(deploy)
            .scan(machine_request)
            .await
            .unwrap();
        assert!(
            response
                .limitations
                .iter()
                .any(|message| message.contains("symlink deploy entry"))
        );
    }

    #[tokio::test]
    async fn image_artifact_adapter_supports_timeout_and_cancellation() {
        let fixture = fixture();
        let deploy = fixture.path().join("qemux86-64");
        fs::create_dir(&deploy).unwrap();
        let cancellation = ImageArtifactCancellation::default();
        cancellation.cancel();
        assert_eq!(
            ImageArtifactAdapter::new(deploy.clone())
                .scan_with_cancellation(request(), cancellation)
                .await,
            Err(ImageArtifactAdapterError::Cancelled)
        );
        assert!(matches!(
            ImageArtifactAdapter::new(deploy)
                .with_timeout(Duration::ZERO)
                .scan(request())
                .await,
            Err(ImageArtifactAdapterError::Timeout(_))
        ));
    }
}
