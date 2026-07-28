use std::{
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::Duration,
};

use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command};
use yoctui_model::{
    MAX_WIC_KICKSTARTS, MAX_WIC_SOURCE_BYTES, WicCapability, WicCreatePreview, WicCreateRequest,
    WicKickstart, WicKickstartIdentity, WicPartitionSummary, normalize_wic_capability,
};

const MAX_WIC_LIST_BYTES: u64 = 256 * 1024;
const WIC_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum WicAdapterError {
    #[error("unsafe Wic executable: {0}")]
    UnsafeExecutable(PathBuf),
    #[error("unsafe Wic kickstart: {0}")]
    UnsafeKickstart(PathBuf),
    #[error("unsafe Wic output directory: {0}")]
    UnsafeOutputDirectory(PathBuf),
    #[error("Wic capability command failed: {0}")]
    Capability(String),
    #[error("invalid Wic request: {0}")]
    InvalidRequest(String),
    #[error("Wic preview does not match the independently validated command")]
    PreviewMismatch,
}

#[derive(Debug, Clone)]
pub struct WicCapabilityInspector {
    executable: PathBuf,
    configured_kickstarts: Vec<PathBuf>,
    canned_roots: Vec<PathBuf>,
}

impl Default for WicCapabilityInspector {
    fn default() -> Self {
        Self {
            executable: "wic".into(),
            configured_kickstarts: Vec::new(),
            canned_roots: Vec::new(),
        }
    }
}

impl WicCapabilityInspector {
    pub fn with_executable(executable: PathBuf) -> Self {
        Self {
            executable,
            ..Self::default()
        }
    }

    pub fn with_sources(
        mut self,
        configured_kickstarts: Vec<PathBuf>,
        canned_roots: Vec<PathBuf>,
    ) -> Self {
        self.configured_kickstarts = configured_kickstarts;
        self.canned_roots = canned_roots;
        self
    }

    pub async fn inspect(&self, image_targets: Vec<String>) -> WicCapability {
        let executable = match resolve_executable(&self.executable) {
            Ok(Some(executable)) => executable,
            Ok(None) => return WicCapability::MissingTool,
            Err(message) => return WicCapability::Failed { message },
        };
        let listed = match list_canned(&executable).await {
            Ok(listed) => listed,
            Err(error) => {
                return WicCapability::Failed {
                    message: error.to_string(),
                };
            }
        };
        let mut kickstarts = Vec::new();
        for path in &self.configured_kickstarts {
            match read_kickstart(path, None) {
                Ok(kickstart) => kickstarts.push(kickstart),
                Err(error) => {
                    return WicCapability::Failed {
                        message: error.to_string(),
                    };
                }
            }
        }
        for name in listed.into_iter().take(MAX_WIC_KICKSTARTS) {
            let path = self.canned_roots.iter().find_map(|root| {
                [
                    root.join(format!("{name}.wks")),
                    root.join(format!("{name}.wks.in")),
                ]
                .into_iter()
                .find(|path| path.exists())
            });
            match path {
                Some(path) => match read_kickstart(&path, Some(name)) {
                    Ok(kickstart) => kickstarts.push(kickstart),
                    Err(error) => {
                        return WicCapability::Failed {
                            message: error.to_string(),
                        };
                    }
                },
                None => kickstarts.push(WicKickstart {
                    identity: WicKickstartIdentity { name, path: None },
                    source: String::new(),
                    partitions: Vec::new(),
                    limitations: vec!["canned kickstart source is unavailable".into()],
                }),
            }
        }
        normalize_wic_capability(WicCapability::Available {
            executable,
            kickstarts,
            image_targets,
        })
    }
}

async fn list_canned(executable: &Path) -> Result<Vec<String>, WicAdapterError> {
    let mut child = Command::new(executable)
        .args(["list", "images"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| WicAdapterError::Capability(error.to_string()))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        WicAdapterError::Capability("wic list images stdout is unavailable".into())
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        WicAdapterError::Capability("wic list images stderr is unavailable".into())
    })?;
    let read = async move {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(MAX_WIC_LIST_BYTES + 1);
        let mut bounded_stderr = stderr.take(MAX_WIC_LIST_BYTES + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        stderr_result.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        let status = status.map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        if stdout_bytes.len() as u64 > MAX_WIC_LIST_BYTES
            || stderr_bytes.len() as u64 > MAX_WIC_LIST_BYTES
        {
            return Err(WicAdapterError::Capability(
                "wic list images output exceeded its safety bound".into(),
            ));
        }
        if !status.success() {
            return Err(WicAdapterError::Capability(
                String::from_utf8_lossy(&stderr_bytes).trim().to_owned(),
            ));
        }
        let output = String::from_utf8(stdout_bytes)
            .map_err(|error| WicAdapterError::Capability(error.to_string()))?;
        let mut names = Vec::new();
        for line in output.lines().filter(|line| !line.trim().is_empty()) {
            let Some(name) = line.split_ascii_whitespace().next() else {
                continue;
            };
            if name.len() <= 256
                && name.chars().all(|character| {
                    character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '+')
                })
            {
                names.push(name.to_owned());
            } else {
                return Err(WicAdapterError::Capability(
                    "wic list images returned a malformed name".into(),
                ));
            }
        }
        names.sort();
        names.dedup();
        Ok(names)
    };
    tokio::time::timeout(WIC_INSPECTION_TIMEOUT, read)
        .await
        .map_err(|_| WicAdapterError::Capability("wic list images timed out".into()))?
}

fn read_kickstart(
    path: &Path,
    canned_name: Option<String>,
) -> Result<WicKickstart, WicAdapterError> {
    let canonical =
        regular_canonical(path).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    let bytes = fs::read(&canonical).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    if bytes.len() > MAX_WIC_SOURCE_BYTES {
        return Err(WicAdapterError::UnsafeKickstart(path.into()));
    }
    let source =
        String::from_utf8(bytes).map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))?;
    let name = canned_name.unwrap_or_else(|| {
        canonical
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .trim_end_matches(".in")
            .trim_end_matches(".wks")
            .to_owned()
    });
    let (partitions, limitations) = parse_kickstart(&source);
    WicKickstart {
        identity: WicKickstartIdentity {
            name,
            path: Some(canonical),
        },
        source,
        partitions,
        limitations,
    }
    .normalize()
    .map_err(|_| WicAdapterError::UnsafeKickstart(path.into()))
}

fn parse_kickstart(source: &str) -> (Vec<WicPartitionSummary>, Vec<String>) {
    let mut partitions = Vec::new();
    let mut limitations = Vec::new();
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut tokens = line.split_ascii_whitespace();
        let Some(command) = tokens.next() else {
            continue;
        };
        if !matches!(command, "part" | "partition") {
            if command != "bootloader" {
                limitations.push(format!("unsupported kickstart command: {command}"));
            }
            continue;
        }
        let mount_point = tokens
            .next()
            .filter(|value| !value.starts_with("--"))
            .map(str::to_owned);
        let mut partition = WicPartitionSummary {
            mount_point,
            filesystem: None,
            source_plugin: None,
            size_mib: None,
            alignment_kib: None,
        };
        for token in line.split_ascii_whitespace().skip(1) {
            if let Some(value) = token.strip_prefix("--fstype=") {
                partition.filesystem = Some(value.into());
            } else if let Some(value) = token.strip_prefix("--source=") {
                partition.source_plugin = Some(value.into());
            } else if let Some(value) = token.strip_prefix("--size=") {
                partition.size_mib = value.parse().ok();
                if partition.size_mib.is_none() {
                    limitations.push("dynamic or invalid partition size".into());
                }
            } else if let Some(value) = token.strip_prefix("--align=") {
                partition.alignment_kib = value.parse().ok();
                if partition.alignment_kib.is_none() {
                    limitations.push("dynamic or invalid partition alignment".into());
                }
            } else if token.contains("${") {
                limitations.push("variable-derived partition option".into());
            }
        }
        partitions.push(partition);
    }
    (partitions, limitations)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicCreateCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl WicCreateCommandSpec {
    pub fn from_preview(
        preview: &WicCreatePreview,
        capability: &WicCapability,
    ) -> Result<Self, WicAdapterError> {
        preview
            .request
            .validate()
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        let (inspected_executable, inspected_kickstart) = capability
            .resolve(&preview.request.kickstart, &preview.request.image)
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        if inspected_kickstart != &preview.kickstart
            || preview.argv.first().map(PathBuf::as_path) != Some(inspected_executable)
        {
            return Err(WicAdapterError::PreviewMismatch);
        }
        let executable = regular_executable(inspected_executable)?;
        if let Some(path) = &preview.request.kickstart.path {
            regular_canonical(path).map_err(|_| WicAdapterError::UnsafeKickstart(path.clone()))?;
        }
        canonical_directory(&preview.request.output_directory)?;
        let expected = create_arguments(&preview.request);
        if preview
            .argv
            .iter()
            .skip(1)
            .map(|argument| argument.as_os_str())
            .ne(expected.iter().map(OsString::as_os_str))
        {
            return Err(WicAdapterError::PreviewMismatch);
        }
        Ok(Self {
            executable,
            arguments: expected,
        })
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

fn create_arguments(request: &WicCreateRequest) -> Vec<OsString> {
    let mut arguments = vec![
        "create".into(),
        request.kickstart.argument().into_os_string(),
        "-e".into(),
        request.image.clone().into(),
        "-o".into(),
        request.output_directory.as_os_str().to_owned(),
    ];
    if request.generate_bmap {
        arguments.push("--bmap".into());
    }
    if let Some(compression) = request.compression.argument() {
        arguments.extend(["--compress-with".into(), compression.into()]);
    }
    arguments
}

fn resolve_executable(program: &Path) -> Result<Option<PathBuf>, String> {
    if program.is_absolute() {
        return if program.exists() {
            regular_executable(program)
                .map(Some)
                .map_err(|error| error.to_string())
        } else {
            Ok(None)
        };
    }
    if program.components().count() != 1
        || !matches!(program.components().next(), Some(Component::Normal(_)))
    {
        return Err("relative Wic executable candidates are ambiguous".into());
    }
    let Some(path) = std::env::var_os("PATH") else {
        return Ok(None);
    };
    for directory in std::env::split_paths(&path).filter(|path| path.is_absolute()) {
        let candidate = directory.join(program);
        if candidate.exists() {
            return regular_executable(&candidate)
                .map(Some)
                .map_err(|error| error.to_string());
        }
    }
    Ok(None)
}

fn regular_executable(path: &Path) -> Result<PathBuf, WicAdapterError> {
    let canonical =
        regular_canonical(path).map_err(|_| WicAdapterError::UnsafeExecutable(path.into()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if fs::metadata(&canonical)
            .map_err(|_| WicAdapterError::UnsafeExecutable(path.into()))?
            .permissions()
            .mode()
            & 0o111
            == 0
        {
            return Err(WicAdapterError::UnsafeExecutable(path.into()));
        }
    }
    Ok(canonical)
}

fn regular_canonical(path: &Path) -> Result<PathBuf, ()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| ())?;
    if !path.is_absolute() || metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(());
    }
    let canonical = fs::canonicalize(path).map_err(|_| ())?;
    (canonical == path).then_some(canonical).ok_or(())
}

fn canonical_directory(path: &Path) -> Result<PathBuf, WicAdapterError> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|_| WicAdapterError::UnsafeOutputDirectory(path.into()))?;
    let canonical =
        fs::canonicalize(path).map_err(|_| WicAdapterError::UnsafeOutputDirectory(path.into()))?;
    if !path.is_absolute()
        || metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || canonical != path
    {
        return Err(WicAdapterError::UnsafeOutputDirectory(path.into()));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{WicCompression, WicCreateDraft};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    fn fixture(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yoctui-wic-capability-{}-{name}-{}",
            std::process::id(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::canonicalize(path).unwrap()
    }

    #[cfg(unix)]
    fn executable(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_capability_discovers_parses_and_constructs_exact_command() {
        let directory = fixture("exact");
        let program = directory.join("wic");
        executable(
            &program,
            "test \"$1 $2\" = 'list images' && printf 'directdisk  Direct disk\\ncustom Custom\\n'",
        );
        let canned = directory.join("canned");
        fs::create_dir(&canned).unwrap();
        let canned = fs::canonicalize(canned).unwrap();
        fs::write(
            canned.join("directdisk.wks"),
            "part / --source=rootfs --fstype=ext4 --size=64 --align=4\nbootloader --ptable gpt\n",
        )
        .unwrap();
        fs::write(
            canned.join("custom.wks.in"),
            "part /boot --source=bootimg --size=${BOOT_SIZE}\nunsupported value\n",
        )
        .unwrap();
        let capability = WicCapabilityInspector::with_executable(program)
            .with_sources(Vec::new(), vec![canned])
            .inspect(vec!["core-image-minimal".into()])
            .await;
        let WicCapability::Available { kickstarts, .. } = &capability else {
            panic!("available capability: {capability:?}");
        };
        assert_eq!(kickstarts.len(), 2);
        assert_eq!(
            kickstarts[1].partitions[0].mount_point.as_deref(),
            Some("/")
        );
        assert_eq!(kickstarts[1].partitions[0].size_mib, Some(64));
        assert!(
            kickstarts[0]
                .limitations
                .iter()
                .any(|limitation| limitation.contains("dynamic"))
        );

        let output = directory.join("output");
        fs::create_dir(&output).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let draft = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: kickstarts[1].identity.clone(),
            output_directory: output.display().to_string(),
            generate_bmap: true,
            compression: WicCompression::Gzip,
        };
        let preview = draft.preview(&capability).unwrap();
        let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
        assert_eq!(
            command.arguments(),
            &[
                OsString::from("create"),
                kickstarts[1]
                    .identity
                    .path
                    .as_ref()
                    .unwrap()
                    .as_os_str()
                    .to_owned(),
                "-e".into(),
                "core-image-minimal".into(),
                "-o".into(),
                output.as_os_str().to_owned(),
                "--bmap".into(),
                "--compress-with".into(),
                "gzip".into(),
            ]
        );
        let alternate = directory.join("alternate-wic");
        executable(&alternate, "exit 0");
        let alternate = fs::canonicalize(alternate).unwrap();
        let mut changed_capability = capability.clone();
        if let WicCapability::Available { executable, .. } = &mut changed_capability {
            *executable = alternate;
        }
        assert_eq!(
            WicCreateCommandSpec::from_preview(&preview, &changed_capability).unwrap_err(),
            WicAdapterError::PreviewMismatch
        );
        let mut tampered = preview;
        tampered.argv.push("--debug".into());
        assert_eq!(
            WicCreateCommandSpec::from_preview(&tampered, &capability).unwrap_err(),
            WicAdapterError::PreviewMismatch
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_capability_reports_missing_malformed_and_unsafe_sources() {
        assert_eq!(
            WicCapabilityInspector::with_executable("/missing/wic".into())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::MissingTool
        );
        let directory = fixture("unsafe");
        let program = directory.join("wic");
        executable(&program, "printf 'bad/name malformed\\n'");
        assert!(matches!(
            WicCapabilityInspector::with_executable(program.clone())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::Failed { .. }
        ));
        let target = directory.join("target.wks");
        fs::write(&target, "part /\n").unwrap();
        let link = directory.join("linked.wks");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        executable(&program, "exit 0");
        assert!(matches!(
            WicCapabilityInspector::with_executable(program)
                .with_sources(vec![link], Vec::new())
                .inspect(vec!["core-image-minimal".into()])
                .await,
            WicCapability::Failed { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }
}
