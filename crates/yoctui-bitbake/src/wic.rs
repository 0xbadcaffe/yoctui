use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};

use crate::{WicRunnerEvent, WicRunnerOutputStream, output_text};
use serde::Deserialize;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader},
    process::{Child, Command},
};
use yoctui_model::{
    MAX_WIC_DEVICE_MOUNTS, MAX_WIC_DEVICES, MAX_WIC_KICKSTARTS, MAX_WIC_LIMITATIONS,
    MAX_WIC_SOURCE_BYTES, WicCapability, WicCreatePreview, WicCreateRequest, WicDevice,
    WicDeviceIdentity, WicDeviceInventoryRequest, WicKickstart, WicKickstartIdentity, WicOutput,
    WicOutputIdentity, WicOutputKind, WicPartitionSummary, WicWriteRequest,
    normalize_wic_capability, normalize_wic_devices, normalize_wic_limitations,
};

const MAX_WIC_LIST_BYTES: u64 = 256 * 1024;
const WIC_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_WIC_LINE_BYTES: usize = 64 * 1024;
const MAX_WIC_OUTPUT_ENTRIES: usize = 4_096;
const WIC_EVENT_CHANNEL_CAPACITY: usize = 256;
const MAX_WIC_DEVICE_JSON_BYTES: u64 = 1024 * 1024;
const MAX_WIC_DEVICE_RECORDS: usize = 512;
const MAX_WIC_DEVICE_PATH_BYTES: usize = 4_096;
const WIC_DEVICE_INSPECTION_TIMEOUT: Duration = Duration::from_secs(10);
const WIC_SPAWN_ATTEMPTS: usize = 4;
const WIC_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);
type WicOutputSnapshot = BTreeMap<PathBuf, (u64, u128)>;
type WicOutputScan = (WicOutputSnapshot, Vec<String>);

#[cfg(unix)]
fn is_transient_spawn_error(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_spawn_error(_error: &std::io::Error) -> bool {
    false
}

async fn spawn_async_command(command: &mut Command) -> std::io::Result<Child> {
    for attempt in 1..=WIC_SPAWN_ATTEMPTS {
        match command.spawn() {
            Ok(child) => return Ok(child),
            Err(error) if attempt < WIC_SPAWN_ATTEMPTS && is_transient_spawn_error(&error) => {
                tokio::time::sleep(WIC_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded Wic spawn loop always returns")
}

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
    #[error("a Wic process or unconsumed terminal event is already active")]
    Busy,
    #[error("could not start Wic: {0}")]
    Spawn(String),
    #[error("Wic runner is not active")]
    NotRunning,
    #[error("Wic process control failed: {0}")]
    ProcessControl(String),
    #[error("Wic output scan failed: {0}")]
    OutputScan(String),
    #[error("Wic device discovery tool is unavailable: {0}")]
    MissingDeviceTool(PathBuf),
    #[error("Wic device discovery failed: {0}")]
    DeviceDiscovery(String),
    #[error("unsafe Wic image identity: {0}")]
    UnsafeImage(PathBuf),
    #[error("unsafe Wic device identity: {0}")]
    UnsafeDevice(PathBuf),
    #[error("the Wic device identity changed since discovery")]
    StaleDevice,
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
    let mut command = Command::new(executable);
    command
        .args(["list", "images"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = spawn_async_command(&mut command)
        .await
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicDeviceInventoryResponse {
    pub request: WicDeviceInventoryRequest,
    pub devices: Vec<WicDevice>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WicDeviceInspector {
    lsblk_program: PathBuf,
    validate_device_nodes: bool,
    inspection_timeout: Duration,
    unwritable_device_nodes: BTreeSet<PathBuf>,
}

impl Default for WicDeviceInspector {
    fn default() -> Self {
        Self {
            lsblk_program: "lsblk".into(),
            validate_device_nodes: true,
            inspection_timeout: WIC_DEVICE_INSPECTION_TIMEOUT,
            unwritable_device_nodes: BTreeSet::new(),
        }
    }
}

impl WicDeviceInspector {
    pub fn with_program(program: PathBuf) -> Self {
        Self {
            lsblk_program: program,
            ..Self::default()
        }
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    #[doc(hidden)]
    pub fn without_device_node_validation_for_tests(mut self) -> Self {
        self.validate_device_nodes = false;
        self
    }

    #[cfg(test)]
    fn with_inspection_timeout(mut self, timeout: Duration) -> Self {
        self.inspection_timeout = timeout;
        self
    }

    #[cfg(test)]
    fn with_unwritable_device(mut self, path: PathBuf) -> Self {
        self.unwritable_device_nodes.insert(path);
        self
    }

    pub async fn discover(
        &self,
        request: WicDeviceInventoryRequest,
    ) -> Result<WicDeviceInventoryResponse, WicAdapterError> {
        request
            .validate()
            .map_err(|message| WicAdapterError::InvalidRequest(message.into()))?;
        validate_wic_image(&request.image)?;
        let executable = resolve_executable(&self.lsblk_program)
            .map_err(WicAdapterError::DeviceDiscovery)?
            .ok_or_else(|| WicAdapterError::MissingDeviceTool(self.lsblk_program.clone()))?;
        let bytes = run_lsblk(&executable, self.inspection_timeout).await?;
        let (devices, limitations) = parse_lsblk_devices(
            &bytes,
            &request.image,
            self.validate_device_nodes,
            &self.unwritable_device_nodes,
        )?;
        Ok(WicDeviceInventoryResponse {
            request,
            devices,
            limitations,
        })
    }

    pub async fn command_for(
        &self,
        request: &WicWriteRequest,
    ) -> Result<WicWriteCommandSpec, WicAdapterError> {
        let inventory = self
            .discover(WicDeviceInventoryRequest {
                generation: 1,
                image: request.image.clone(),
            })
            .await?;
        let device = inventory
            .devices
            .iter()
            .find(|device| device.identity == request.device)
            .ok_or(WicAdapterError::StaleDevice)?;
        device
            .eligible_for(&request.image)
            .map_err(|_| WicAdapterError::UnsafeDevice(request.device.path.clone()))?;
        let executable = regular_executable(&request.executable)?;
        if executable != request.executable {
            return Err(WicAdapterError::UnsafeExecutable(
                request.executable.clone(),
            ));
        }
        Ok(WicWriteCommandSpec {
            executable,
            arguments: vec![
                "write".into(),
                request.image.path.as_os_str().to_owned(),
                request.device.path.as_os_str().to_owned(),
            ],
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WicWriteCommandSpec {
    executable: PathBuf,
    arguments: Vec<OsString>,
}

impl WicWriteCommandSpec {
    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }
}

#[derive(Debug, Deserialize)]
struct LsblkDocument {
    blockdevices: Vec<LsblkNode>,
}

#[derive(Debug, Deserialize)]
struct LsblkNode {
    path: serde_json::Value,
    #[serde(rename = "type")]
    kind: serde_json::Value,
    #[serde(rename = "maj:min")]
    major_minor: serde_json::Value,
    size: serde_json::Value,
    model: serde_json::Value,
    serial: serde_json::Value,
    tran: serde_json::Value,
    rm: serde_json::Value,
    ro: serde_json::Value,
    mountpoints: serde_json::Value,
    #[serde(default)]
    children: Vec<LsblkNode>,
}

async fn run_lsblk(executable: &Path, timeout: Duration) -> Result<Vec<u8>, WicAdapterError> {
    let mut command = Command::new(executable);
    command
        .args([
            "--json",
            "--bytes",
            "--paths",
            "--output",
            "PATH,TYPE,MAJ:MIN,SIZE,MODEL,SERIAL,TRAN,RM,RO,MOUNTPOINTS",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = spawn_async_command(&mut command)
        .await
        .map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| WicAdapterError::DeviceDiscovery("lsblk stdout is unavailable".into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| WicAdapterError::DeviceDiscovery("lsblk stderr is unavailable".into()))?;
    let collect = async move {
        let mut stdout_bytes = Vec::new();
        let mut stderr_bytes = Vec::new();
        let mut bounded_stdout = stdout.take(MAX_WIC_DEVICE_JSON_BYTES + 1);
        let mut bounded_stderr = stderr.take(MAX_WIC_DEVICE_JSON_BYTES + 1);
        let stdout_read = bounded_stdout.read_to_end(&mut stdout_bytes);
        let stderr_read = bounded_stderr.read_to_end(&mut stderr_bytes);
        let (stdout_result, stderr_result, status) =
            tokio::join!(stdout_read, stderr_read, child.wait());
        stdout_result.map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
        stderr_result.map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
        let status = status.map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
        Ok::<_, WicAdapterError>((stdout_bytes, stderr_bytes, status))
    };
    let (stdout, stderr, status) = tokio::time::timeout(timeout, collect)
        .await
        .map_err(|_| WicAdapterError::DeviceDiscovery("lsblk timed out".into()))??;
    if stdout.len() as u64 > MAX_WIC_DEVICE_JSON_BYTES
        || stderr.len() as u64 > MAX_WIC_DEVICE_JSON_BYTES
    {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk output exceeded its safety bound".into(),
        ));
    }
    if !status.success() {
        return Err(WicAdapterError::DeviceDiscovery(
            String::from_utf8_lossy(&stderr).trim().to_owned(),
        ));
    }
    Ok(stdout)
}

fn json_string(value: &serde_json::Value, field: &str) -> Result<String, WicAdapterError> {
    value
        .as_str()
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(|| WicAdapterError::DeviceDiscovery(format!("invalid lsblk {field}")))
}

fn json_bounded_string(
    value: &serde_json::Value,
    field: &str,
    max_bytes: usize,
) -> Result<String, WicAdapterError> {
    let value = json_string(value, field)?;
    if value.len() > max_bytes {
        return Err(WicAdapterError::DeviceDiscovery(format!(
            "lsblk {field} exceeded its safety bound"
        )));
    }
    Ok(value)
}

fn json_optional_string(
    value: &serde_json::Value,
    field: &str,
) -> Result<Option<String>, WicAdapterError> {
    if value.is_null() {
        return Ok(None);
    }
    json_bounded_string(value, field, 256).map(Some)
}

fn json_u64(value: &serde_json::Value, field: &str) -> Result<u64, WicAdapterError> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|value| value.parse().ok()))
        .ok_or_else(|| WicAdapterError::DeviceDiscovery(format!("invalid lsblk {field}")))
}

fn json_bool(value: &serde_json::Value, field: &str) -> Result<bool, WicAdapterError> {
    value
        .as_bool()
        .or_else(|| {
            value.as_u64().and_then(|value| match value {
                0 => Some(false),
                1 => Some(true),
                _ => None,
            })
        })
        .or_else(|| {
            value.as_str().and_then(|value| match value {
                "0" | "false" => Some(false),
                "1" | "true" => Some(true),
                _ => None,
            })
        })
        .ok_or_else(|| WicAdapterError::DeviceDiscovery(format!("invalid lsblk {field}")))
}

fn mountpoints(value: &serde_json::Value) -> Result<Vec<PathBuf>, WicAdapterError> {
    let values: Vec<&serde_json::Value> = match value {
        serde_json::Value::Null => Vec::new(),
        serde_json::Value::String(_) => vec![value],
        serde_json::Value::Array(values) => values.iter().collect(),
        _ => {
            return Err(WicAdapterError::DeviceDiscovery(
                "invalid lsblk mountpoints".into(),
            ));
        }
    };
    let mut paths = Vec::new();
    for value in values {
        if value.is_null() {
            continue;
        }
        let path = PathBuf::from(json_bounded_string(
            value,
            "mountpoint",
            MAX_WIC_DEVICE_PATH_BYTES,
        )?);
        if !path.is_absolute() {
            return Err(WicAdapterError::DeviceDiscovery(
                "lsblk returned a relative mountpoint".into(),
            ));
        }
        paths.push(path);
    }
    paths.sort();
    paths.dedup();
    if paths.len() > MAX_WIC_DEVICE_MOUNTS {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk returned too many mountpoints".into(),
        ));
    }
    Ok(paths)
}

fn subtree_mounts(node: &LsblkNode, output: &mut Vec<PathBuf>) -> Result<(), WicAdapterError> {
    output.extend(mountpoints(&node.mountpoints)?);
    for child in &node.children {
        subtree_mounts(child, output)?;
    }
    Ok(())
}

fn subtree_has_root_mount(node: &LsblkNode) -> Result<bool, WicAdapterError> {
    if mountpoints(&node.mountpoints)?
        .iter()
        .any(|mount| mount == Path::new("/"))
    {
        return Ok(true);
    }
    for child in &node.children {
        if subtree_has_root_mount(child)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn validate_lsblk_records(
    nodes: &[LsblkNode],
    seen_paths: &mut BTreeMap<PathBuf, String>,
    seen_major_minor: &mut BTreeMap<String, PathBuf>,
    count: &mut usize,
) -> Result<(), WicAdapterError> {
    for node in nodes {
        *count = count.saturating_add(1);
        if *count > MAX_WIC_DEVICE_RECORDS {
            return Err(WicAdapterError::DeviceDiscovery(
                "lsblk returned too many device records".into(),
            ));
        }
        let path = PathBuf::from(json_bounded_string(
            &node.path,
            "path",
            MAX_WIC_DEVICE_PATH_BYTES,
        )?);
        let _ = json_bounded_string(&node.kind, "type", 32)?;
        let major_minor = json_bounded_string(&node.major_minor, "major:minor", 32)?;
        let _ = json_u64(&node.size, "size")?;
        let _ = json_optional_string(&node.model, "model")?;
        let _ = json_optional_string(&node.serial, "serial")?;
        let _ = json_optional_string(&node.tran, "transport")?;
        let _ = json_bool(&node.rm, "removable")?;
        let _ = json_bool(&node.ro, "read-only")?;
        let _ = mountpoints(&node.mountpoints)?;
        WicDeviceIdentity {
            path: path.clone(),
            major_minor: major_minor.clone(),
            size_bytes: 0,
            model: None,
            serial: None,
            transport: None,
        }
        .validate()
        .map_err(|message| WicAdapterError::DeviceDiscovery(message.into()))?;
        if seen_paths
            .insert(path.clone(), major_minor.clone())
            .is_some()
            || seen_major_minor.insert(major_minor, path).is_some()
        {
            return Err(WicAdapterError::DeviceDiscovery(
                "lsblk returned duplicate device identities".into(),
            ));
        }
        validate_lsblk_records(&node.children, seen_paths, seen_major_minor, count)?;
    }
    Ok(())
}

fn validate_device_node(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileTypeExt;
        let Ok(metadata) = fs::symlink_metadata(path) else {
            return false;
        };
        !metadata.file_type().is_symlink()
            && metadata.file_type().is_block_device()
            && fs::canonicalize(path).is_ok_and(|canonical| canonical == path)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        false
    }
}

fn device_is_writable(
    path: &Path,
    validate_device_nodes: bool,
    unwritable_device_nodes: &BTreeSet<PathBuf>,
) -> bool {
    !unwritable_device_nodes.contains(path)
        && (!validate_device_nodes || fs::OpenOptions::new().write(true).open(path).is_ok())
}

fn parse_lsblk_devices(
    bytes: &[u8],
    image: &WicOutputIdentity,
    validate_device_nodes: bool,
    unwritable_device_nodes: &BTreeSet<PathBuf>,
) -> Result<(Vec<WicDevice>, Vec<String>), WicAdapterError> {
    if bytes.len() as u64 > MAX_WIC_DEVICE_JSON_BYTES {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk output exceeded its safety bound".into(),
        ));
    }
    let document: LsblkDocument = serde_json::from_slice(bytes)
        .map_err(|error| WicAdapterError::DeviceDiscovery(error.to_string()))?;
    let mut seen_paths = BTreeMap::<PathBuf, String>::new();
    let mut seen_major_minor = BTreeMap::<String, PathBuf>::new();
    let mut record_count = 0;
    validate_lsblk_records(
        &document.blockdevices,
        &mut seen_paths,
        &mut seen_major_minor,
        &mut record_count,
    )?;
    let mut root_devices = Vec::new();
    for node in &document.blockdevices {
        if json_bounded_string(&node.kind, "type", 32)? == "disk" && subtree_has_root_mount(node)? {
            root_devices.push(json_bounded_string(
                &node.path,
                "path",
                MAX_WIC_DEVICE_PATH_BYTES,
            )?);
        }
    }
    root_devices.sort();
    root_devices.dedup();
    if root_devices.len() != 1 {
        return Err(WicAdapterError::DeviceDiscovery(
            "the root backing whole device could not be identified uniquely".into(),
        ));
    }
    let root_device = &root_devices[0];
    let mut devices = Vec::new();
    let mut limitations = Vec::new();
    for node in &document.blockdevices {
        let path = PathBuf::from(json_bounded_string(
            &node.path,
            "path",
            MAX_WIC_DEVICE_PATH_BYTES,
        )?);
        let kind = json_bounded_string(&node.kind, "type", 32)?;
        let major_minor = json_bounded_string(&node.major_minor, "major:minor", 32)?;
        if kind != "disk" {
            limitations.push(format!("{}: excluded device type {kind}", path.display()));
            continue;
        }
        let size_bytes = json_u64(&node.size, "size")?;
        let removable = json_bool(&node.rm, "removable")?;
        let read_only = json_bool(&node.ro, "read-only")?;
        let mut descendant_mounts = Vec::new();
        subtree_mounts(node, &mut descendant_mounts)?;
        descendant_mounts.sort();
        descendant_mounts.dedup();
        let reason = if path.to_str() == Some(root_device) {
            Some("backs the current root filesystem")
        } else if !path.is_absolute() || !path.starts_with("/dev") {
            Some("path is not an absolute /dev identity")
        } else if validate_device_nodes && !validate_device_node(&path) {
            Some("path is not a canonical whole block-device node")
        } else if !removable {
            Some("device is not removable")
        } else if read_only {
            Some("device is read-only")
        } else if !descendant_mounts.is_empty() {
            Some("device has mounted descendants")
        } else if size_bytes < image.size_bytes {
            Some("device is smaller than the selected image")
        } else if !device_is_writable(&path, validate_device_nodes, unwritable_device_nodes) {
            Some("device cannot be opened for writing")
        } else {
            None
        };
        if let Some(reason) = reason {
            limitations.push(format!("{}: {reason}", path.display()));
            continue;
        }
        devices.push(WicDevice {
            identity: WicDeviceIdentity {
                path,
                major_minor,
                size_bytes,
                model: json_optional_string(&node.model, "model")?,
                serial: json_optional_string(&node.serial, "serial")?,
                transport: json_optional_string(&node.tran, "transport")?,
            },
            removable,
            writable: true,
            read_only,
            descendant_mounts,
            unavailable_reason: None,
        });
    }
    if devices.len() > MAX_WIC_DEVICES {
        return Err(WicAdapterError::DeviceDiscovery(
            "lsblk returned too many eligible devices".into(),
        ));
    }
    if limitations.len() > MAX_WIC_LIMITATIONS {
        let omitted = limitations.len() - (MAX_WIC_LIMITATIONS - 1);
        limitations.truncate(MAX_WIC_LIMITATIONS - 1);
        limitations.push(format!(
            "{omitted} additional unsafe device exclusions were omitted"
        ));
    }
    Ok((
        normalize_wic_devices(devices),
        normalize_wic_limitations(limitations),
    ))
}

fn validate_wic_image(image: &WicOutputIdentity) -> Result<(), WicAdapterError> {
    image
        .validate()
        .map_err(|_| WicAdapterError::UnsafeImage(image.path.clone()))?;
    let canonical = regular_canonical(&image.path)
        .map_err(|_| WicAdapterError::UnsafeImage(image.path.clone()))?;
    let size = fs::metadata(&canonical)
        .map_err(|_| WicAdapterError::UnsafeImage(image.path.clone()))?
        .len();
    if canonical != image.path || size != image.size_bytes {
        return Err(WicAdapterError::UnsafeImage(image.path.clone()));
    }
    Ok(())
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

#[derive(Debug)]
enum WicPipeEvent {
    Output {
        stream: WicRunnerOutputStream,
        line: String,
        truncated: bool,
    },
    Failed(String),
}

async fn read_wic_output<R>(
    stream: R,
    kind: WicRunnerOutputStream,
    sender: tokio::sync::mpsc::Sender<WicPipeEvent>,
) where
    R: AsyncRead + Unpin,
{
    let mut reader = BufReader::new(stream);
    let mut bytes = Vec::new();
    let mut truncated = false;
    loop {
        let buffer = match reader.fill_buf().await {
            Ok(buffer) => buffer,
            Err(error) => {
                let _ = sender.send(WicPipeEvent::Failed(error.to_string())).await;
                return;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(WicPipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            return;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_WIC_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(WicPipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                return;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

pub struct WicJobRunner {
    build_dir: PathBuf,
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<WicPipeEvent>>,
    start_events_pending: u8,
    terminal_pending: VecDeque<WicRunnerEvent>,
    output_root: Option<PathBuf>,
    before: WicOutputSnapshot,
    cancellation_timeout: Duration,
    execution_timeout: Duration,
    started_at: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl WicJobRunner {
    pub fn new(build_dir: PathBuf) -> Self {
        Self {
            build_dir,
            child: None,
            output: None,
            start_events_pending: 0,
            terminal_pending: VecDeque::new(),
            output_root: None,
            before: BTreeMap::new(),
            cancellation_timeout: Duration::from_secs(5),
            execution_timeout: Duration::from_secs(60 * 60),
            started_at: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_execution_timeout(mut self, timeout: Duration) -> Self {
        self.execution_timeout = timeout;
        self
    }

    pub async fn start(
        &mut self,
        command: WicCreateCommandSpec,
        output_directory: PathBuf,
    ) -> Result<(), WicAdapterError> {
        self.ensure_idle()?;
        let output_root = canonical_directory(&output_directory)?;
        let (before, _) = scan_outputs(&output_root)?;
        self.output_root = Some(output_root);
        self.before = before;
        self.spawn(&command.executable, &command.arguments)
    }

    pub async fn start_write(
        &mut self,
        inspector: &WicDeviceInspector,
        request: WicWriteRequest,
    ) -> Result<(), WicAdapterError> {
        self.ensure_idle()?;
        let command = inspector.command_for(&request).await?;
        self.output_root = None;
        self.before.clear();
        self.spawn(&command.executable, &command.arguments)
    }

    fn ensure_idle(&self) -> Result<(), WicAdapterError> {
        if self.child.is_some()
            || self.output.is_some()
            || self.start_events_pending > 0
            || !self.terminal_pending.is_empty()
        {
            Err(WicAdapterError::Busy)
        } else {
            Ok(())
        }
    }

    fn spawn(&mut self, executable: &Path, arguments: &[OsString]) -> Result<(), WicAdapterError> {
        if !self.build_dir.is_dir() {
            return Err(WicAdapterError::Spawn(format!(
                "build directory does not exist: {}",
                self.build_dir.display()
            )));
        }
        let mut process = Command::new(executable);
        process
            .args(arguments)
            .current_dir(&self.build_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| WicAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| WicAdapterError::Spawn("stdout is unavailable".into()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| WicAdapterError::Spawn("stderr is unavailable".into()))?;
        let (sender, receiver) = tokio::sync::mpsc::channel(WIC_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_wic_output(
            stdout,
            WicRunnerOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_wic_output(
            stderr,
            WicRunnerOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.start_events_pending = 2;
        self.cancellation_requested = false;
        self.started_at = Some(Instant::now());
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<WicRunnerEvent, WicAdapterError> {
        if self.start_events_pending == 2 {
            self.start_events_pending = 1;
            return Ok(WicRunnerEvent::Starting);
        }
        if self.start_events_pending == 1 {
            self.start_events_pending = 0;
            return Ok(WicRunnerEvent::Started);
        }
        let remaining = self.remaining();
        if let Some(receiver) = self.output.as_mut() {
            match tokio::time::timeout(remaining, receiver.recv()).await {
                Err(_) => {
                    self.kill_and_clear().await;
                    return Ok(WicRunnerEvent::Failed {
                        message: "Wic operation timed out".into(),
                        exit_code: None,
                    });
                }
                Ok(Some(WicPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(WicRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Ok(Some(WicPipeEvent::Failed(message))) => {
                    self.kill_and_clear().await;
                    return Ok(WicRunnerEvent::Lost { message });
                }
                Ok(None) => {
                    self.output = None;
                }
            }
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        let remaining = self.remaining();
        let child = self.child.as_mut().ok_or(WicAdapterError::NotRunning)?;
        let status = match tokio::time::timeout(remaining, child.wait()).await {
            Ok(Ok(status)) => status,
            Ok(Err(error)) => {
                self.kill_and_clear().await;
                return Ok(WicRunnerEvent::Lost {
                    message: format!("Wic process wait failed: {error}"),
                });
            }
            Err(_) => {
                self.kill_and_clear().await;
                return Ok(WicRunnerEvent::Failed {
                    message: "Wic operation timed out".into(),
                    exit_code: None,
                });
            }
        };
        self.child = None;
        self.clear_process_state();
        if !status.success() {
            return Ok(WicRunnerEvent::Failed {
                message: "Wic operation exited unsuccessfully".into(),
                exit_code: status.code(),
            });
        }
        let (outputs, limitations) = if let Some(root) = self.output_root.take() {
            let (after, limitations) = scan_outputs(&root)?;
            let outputs = after
                .into_iter()
                .filter(|(path, identity)| self.before.get(path) != Some(identity))
                .map(|(path, (size_bytes, modified_nanoseconds))| WicOutput {
                    kind: classify_output(&path),
                    identity: WicOutputIdentity {
                        path,
                        size_bytes,
                        modified_unix_seconds: (modified_nanoseconds / 1_000_000_000) as u64,
                    },
                })
                .collect();
            (outputs, limitations)
        } else {
            (Vec::new(), Vec::new())
        };
        self.before.clear();
        Ok(WicRunnerEvent::Completed {
            exit_code: status.code().unwrap_or(0),
            outputs,
            limitations,
        })
    }

    pub async fn cancel(&mut self) -> Result<bool, WicAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(WicRunnerEvent::CancellationRejected {
                    message: "no cancellable Wic process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let child = self.child.as_mut().expect("checked above");
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(group) = self.process_group {
            if unsafe { libc::kill(-group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => {
                    result.map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
                }
                Err(_) => {
                    let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
                    forced = true;
                    child
                        .wait()
                        .await
                        .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?;
            child
                .wait()
                .await
                .map_err(|error| WicAdapterError::ProcessControl(error.to_string()))?
        };
        self.child = None;
        self.clear_process_state();
        self.terminal_pending.push_back(WicRunnerEvent::Cancelled {
            forced,
            exit_code: status.code(),
        });
        Ok(true)
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.child = None;
        self.output = None;
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.cancellation_requested = false;
        self.started_at = None;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    fn remaining(&self) -> Duration {
        self.started_at
            .map(|started| self.execution_timeout.saturating_sub(started.elapsed()))
            .unwrap_or(self.execution_timeout)
    }
}

impl Drop for WicJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(group) = self.process_group {
            let _ = unsafe { libc::kill(-group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

fn scan_outputs(root: &Path) -> Result<WicOutputScan, WicAdapterError> {
    let root = canonical_directory(root)?;
    let mut files = BTreeMap::new();
    let mut limitations = Vec::new();
    for (index, entry) in fs::read_dir(&root)
        .map_err(|error| WicAdapterError::OutputScan(error.to_string()))?
        .enumerate()
    {
        if index >= MAX_WIC_OUTPUT_ENTRIES {
            limitations.push(format!(
                "Wic output scan was limited to {MAX_WIC_OUTPUT_ENTRIES} entries"
            ));
            break;
        }
        let Ok(entry) = entry else {
            limitations.push("one Wic output entry was unreadable".into());
            continue;
        };
        let path = entry.path();
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            limitations.push(format!("metadata unavailable for {}", path.display()));
            continue;
        };
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let Ok(canonical) = fs::canonicalize(&path) else {
            limitations.push(format!("could not canonicalize {}", path.display()));
            continue;
        };
        if canonical != path || !canonical.starts_with(&root) {
            limitations.push(format!("unsafe Wic output ignored: {}", path.display()));
            continue;
        }
        let modified = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |duration| duration.as_nanos());
        files.insert(canonical, (metadata.len(), modified));
    }
    Ok((files, limitations))
}

fn classify_output(path: &Path) -> WicOutputKind {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.ends_with(".wic") {
        WicOutputKind::Wic
    } else if name.ends_with(".direct") {
        WicOutputKind::Direct
    } else if name.ends_with(".bmap") {
        WicOutputKind::Bmap
    } else if name.ends_with(".gz") || name.ends_with(".bz2") || name.ends_with(".xz") {
        WicOutputKind::Compressed
    } else {
        WicOutputKind::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use yoctui_model::{WicCompression, WicCreateDraft};

    static FIXTURE_ID: AtomicU64 = AtomicU64::new(1);

    fn fixture(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "yoctui-wic-capability-{}-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            FIXTURE_ID.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        fs::canonicalize(path).unwrap()
    }

    #[cfg(unix)]
    fn executable(path: &Path, body: &str) {
        crate::test_support::write_executable(path, &format!("#!/bin/sh\n{body}\n"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_async_spawn_retries_only_transient_text_file_busy() {
        let directory = fixture("spawn-retry");
        let program = directory.join("wic");
        executable(&program, "exit 0");
        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let release = tokio::spawn(async move {
            tokio::time::sleep(WIC_SPAWN_RETRY_DELAY + WIC_SPAWN_RETRY_DELAY).await;
            drop(writer);
        });
        let mut command = Command::new(&program);
        let mut child = spawn_async_command(&mut command).await.unwrap();
        assert!(child.wait().await.unwrap().success());
        release.await.unwrap();

        let writer = fs::OpenOptions::new().write(true).open(&program).unwrap();
        let mut command = Command::new(&program);
        let error = match spawn_async_command(&mut command).await {
            Ok(_) => panic!("write-held executable unexpectedly spawned"),
            Err(error) => error,
        };
        assert!(is_transient_spawn_error(&error));
        drop(writer);

        assert!(!is_transient_spawn_error(
            &std::io::Error::from_raw_os_error(libc::EACCES)
        ));
        fs::remove_dir_all(directory).unwrap();
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

    #[cfg(unix)]
    async fn runner_fixture(name: &str, body: &str) -> (PathBuf, PathBuf, WicCreateCommandSpec) {
        let directory = fixture(name);
        let program = directory.join("wic");
        executable(&program, body);
        let kickstart_path = directory.join("directdisk.wks");
        fs::write(&kickstart_path, "part / --source=rootfs\n").unwrap();
        let kickstart_path = fs::canonicalize(kickstart_path).unwrap();
        let output = directory.join("output");
        fs::create_dir(&output).unwrap();
        let output = fs::canonicalize(output).unwrap();
        let capability = WicCapability::Available {
            executable: fs::canonicalize(program).unwrap(),
            kickstarts: vec![read_kickstart(&kickstart_path, Some("directdisk".into())).unwrap()],
            image_targets: vec!["core-image-minimal".into()],
        };
        let preview = WicCreateDraft {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            kickstart: WicKickstartIdentity {
                name: "directdisk".into(),
                path: Some(kickstart_path),
            },
            output_directory: output.display().to_string(),
            generate_bmap: false,
            compression: WicCompression::None,
        }
        .preview(&capability)
        .unwrap();
        let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
        (directory, output, command)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_reports_only_new_outputs_and_nonzero_failure() {
        let (directory, output, command) = runner_fixture(
            "runner-success",
            "printf 'before\\n'; printf 'warning\\n' >&2; printf image > \"$6/new.wic\"; exit 0",
        )
        .await;
        fs::write(output.join("existing.wic"), "old").unwrap();
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start(command, output.clone()).await.unwrap();
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Starting);
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Started);
        let terminal = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Completed { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        let WicRunnerEvent::Completed { outputs, .. } = terminal else {
            unreachable!()
        };
        assert_eq!(outputs.len(), 1);
        assert!(outputs[0].identity.path.ends_with("new.wic"));
        fs::remove_dir_all(directory).unwrap();

        let (directory, output, command) =
            runner_fixture("runner-failure", "printf failed >&2; exit 9").await;
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start(command, output).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let terminal = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Failed { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            terminal,
            WicRunnerEvent::Failed {
                exit_code: Some(9),
                ..
            }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_rejects_duplicate_and_forces_cancellation() {
        let (directory, output, command) = runner_fixture(
            "runner-cancel",
            "trap '' TERM; printf 'ready\\n'; while :; do :; done",
        )
        .await;
        let mut runner = WicJobRunner::new(directory.clone())
            .with_cancellation_timeout(Duration::from_millis(50));
        runner.start(command.clone(), output.clone()).await.unwrap();
        assert_eq!(
            runner.start(command, output).await.unwrap_err(),
            WicAdapterError::Busy
        );
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Starting
        ));
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Started
        ));
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                WicRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        let cancelled = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let event = runner.next_event().await.unwrap();
                if matches!(event, WicRunnerEvent::Cancelled { .. }) {
                    break event;
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            cancelled,
            WicRunnerEvent::Cancelled { forced: true, .. }
        ));
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::CancellationRejected { .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_adapter_runner_times_out_without_blocking_forever() {
        let (directory, output, command) = runner_fixture("runner-timeout", "sleep 30").await;
        let mut runner =
            WicJobRunner::new(directory.clone()).with_execution_timeout(Duration::from_millis(20));
        runner.start(command, output).await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Failed {
                ref message,
                exit_code: None
            } if message.contains("timed out")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    fn lsblk_node(
        path: &str,
        kind: &str,
        major_minor: &str,
        size: u64,
        access: (bool, bool),
        mounts: Vec<&str>,
        children: Vec<serde_json::Value>,
    ) -> serde_json::Value {
        let (removable, read_only) = access;
        serde_json::json!({
            "path": path,
            "type": kind,
            "maj:min": major_minor,
            "size": size,
            "model": if kind == "disk" { serde_json::Value::String("fixture".into()) } else { serde_json::Value::Null },
            "serial": if kind == "disk" { serde_json::Value::String(format!("serial-{major_minor}")) } else { serde_json::Value::Null },
            "tran": if removable { serde_json::Value::String("usb".into()) } else { serde_json::Value::Null },
            "rm": removable,
            "ro": read_only,
            "mountpoints": mounts,
            "children": children,
        })
    }

    fn device_inventory_json(extra: Vec<serde_json::Value>) -> String {
        let root_partition = lsblk_node(
            "/dev/sda1",
            "part",
            "8:1",
            4_096,
            (false, false),
            vec!["/"],
            Vec::new(),
        );
        let mut devices = vec![lsblk_node(
            "/dev/sda",
            "disk",
            "8:0",
            8_192,
            (false, false),
            Vec::new(),
            vec![root_partition],
        )];
        devices.extend(extra);
        serde_json::json!({ "blockdevices": devices }).to_string()
    }

    #[cfg(unix)]
    fn device_write_fixture(
        name: &str,
        inventory: &str,
        wic_body: &str,
    ) -> (
        PathBuf,
        PathBuf,
        WicDeviceInspector,
        WicDeviceInventoryRequest,
        PathBuf,
    ) {
        let directory = fixture(name);
        let image = directory.join("image.wic");
        fs::write(&image, b"image").unwrap();
        let image = fs::canonicalize(image).unwrap();
        let lsblk = directory.join("lsblk");
        executable(
            &lsblk,
            &format!(
                "test \"$#\" -eq 5 && test \"$1\" = --json && test \"$2\" = --bytes && test \"$3\" = --paths && test \"$4\" = --output && test \"$5\" = 'PATH,TYPE,MAJ:MIN,SIZE,MODEL,SERIAL,TRAN,RM,RO,MOUNTPOINTS' || exit 64\nprintf '%s' '{}'",
                inventory.replace('\'', "'\\''")
            ),
        );
        let wic = directory.join("wic");
        executable(&wic, wic_body);
        let request = WicDeviceInventoryRequest {
            generation: 1,
            image: WicOutputIdentity {
                path: image,
                size_bytes: 5,
                modified_unix_seconds: 1,
            },
        };
        (
            directory,
            fs::canonicalize(wic).unwrap(),
            WicDeviceInspector::with_program(fs::canonicalize(&lsblk).unwrap())
                .without_device_node_validation_for_tests(),
            request,
            lsblk,
        )
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_device_write_discovers_safe_whole_device_and_builds_exact_argv() {
        let inventory = device_inventory_json(vec![
            lsblk_node(
                "/dev/sdz",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/loop0",
                "loop",
                "7:0",
                16_384,
                (false, false),
                Vec::new(),
                Vec::new(),
            ),
        ]);
        let (directory, wic, inspector, request, _) =
            device_write_fixture("device-exact", &inventory, "printf '%s\\n' \"$@\"; exit 0");
        let response = inspector.discover(request.clone()).await.unwrap();
        assert_eq!(response.request, request);
        assert_eq!(response.devices.len(), 1);
        let device = &response.devices[0];
        assert_eq!(device.identity.path, Path::new("/dev/sdz"));
        assert_eq!(device.identity.major_minor, "8:240");
        assert_eq!(device.identity.serial.as_deref(), Some("serial-8:240"));
        assert!(
            response
                .limitations
                .iter()
                .any(|limitation| limitation.contains("/dev/sda")
                    && limitation.contains("root filesystem"))
        );
        assert!(
            response
                .limitations
                .iter()
                .any(|limitation| limitation.contains("device type loop"))
        );
        let write_request = WicWriteRequest {
            executable: wic,
            image: request.image,
            device: device.identity.clone(),
        };
        let command = inspector.command_for(&write_request).await.unwrap();
        assert_eq!(command.executable(), write_request.executable);
        assert_eq!(
            command.arguments(),
            &[
                OsString::from("write"),
                write_request.image.path.as_os_str().to_owned(),
                OsString::from("/dev/sdz"),
            ]
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_device_write_fails_closed_for_exclusions_duplicates_and_stale_identity() {
        let candidates = vec![
            lsblk_node(
                "/dev/sdb",
                "disk",
                "8:16",
                16_384,
                (false, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdc",
                "disk",
                "8:32",
                16_384,
                (true, true),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdd",
                "disk",
                "8:48",
                16_384,
                (true, false),
                vec!["/media/card"],
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sde",
                "disk",
                "8:64",
                1,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdf",
                "disk",
                "8:80",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdf1",
                "part",
                "8:81",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sr0",
                "rom",
                "11:0",
                16_384,
                (true, true),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/dm-0",
                "lvm",
                "253:0",
                16_384,
                (false, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdz",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
        ];
        let inventory = device_inventory_json(candidates);
        let (directory, wic, inspector, request, lsblk) =
            device_write_fixture("device-rejections", &inventory, "exit 0");
        let inspector = inspector.with_unwritable_device("/dev/sdf".into());
        let response = inspector.discover(request.clone()).await.unwrap();
        assert_eq!(response.devices.len(), 1);
        for expected in [
            "not removable",
            "read-only",
            "mounted descendants",
            "smaller",
            "cannot be opened",
            "device type part",
            "device type rom",
            "device type lvm",
        ] {
            assert!(
                response
                    .limitations
                    .iter()
                    .any(|limitation| limitation.contains(expected)),
                "{expected}: {:?}",
                response.limitations
            );
        }
        let write_request = WicWriteRequest {
            executable: wic,
            image: request.image.clone(),
            device: response.devices[0].identity.clone(),
        };
        let mut changed_identity_records = Vec::new();
        for (field, value) in [
            ("maj:min", serde_json::json!("8:241")),
            ("size", serde_json::json!(32_768)),
            ("model", serde_json::json!("replacement")),
            ("serial", serde_json::json!("replacement")),
            ("tran", serde_json::json!("mmc")),
            ("rm", serde_json::json!(false)),
            ("ro", serde_json::json!(true)),
            ("mountpoints", serde_json::json!(["/media/stale"])),
        ] {
            let mut node = lsblk_node(
                "/dev/sdz",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            );
            node[field] = value;
            changed_identity_records.push(device_inventory_json(vec![node]));
        }
        for changed in changed_identity_records {
            executable(
                &lsblk,
                &format!("printf '%s' '{}'", changed.replace('\'', "'\\''")),
            );
            assert_eq!(
                inspector.command_for(&write_request).await.unwrap_err(),
                WicAdapterError::StaleDevice
            );
        }

        let duplicate = device_inventory_json(vec![
            lsblk_node(
                "/dev/sdz",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdz",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
        ]);
        assert!(
            parse_lsblk_devices(
                duplicate.as_bytes(),
                &request.image,
                false,
                &BTreeSet::new()
            )
            .is_err()
        );
        assert!(
            parse_lsblk_devices(
                br#"{"blockdevices":[{"path":"/dev/sda","type":"disk"}]}"#,
                &request.image,
                false,
                &BTreeSet::new()
            )
            .is_err()
        );
        let mut malformed_non_disk = lsblk_node(
            "/dev/loop0",
            "loop",
            "7:0",
            16_384,
            (false, false),
            Vec::new(),
            Vec::new(),
        );
        malformed_non_disk["rm"] = serde_json::json!("maybe");
        assert!(
            parse_lsblk_devices(
                device_inventory_json(vec![malformed_non_disk]).as_bytes(),
                &request.image,
                false,
                &BTreeSet::new()
            )
            .is_err()
        );
        let duplicate_major_minor = device_inventory_json(vec![
            lsblk_node(
                "/dev/sdz",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
            lsblk_node(
                "/dev/sdy",
                "disk",
                "8:240",
                16_384,
                (true, false),
                Vec::new(),
                Vec::new(),
            ),
        ]);
        assert!(
            parse_lsblk_devices(
                duplicate_major_minor.as_bytes(),
                &request.image,
                false,
                &BTreeSet::new()
            )
            .is_err()
        );
        let no_root = serde_json::json!({
            "blockdevices": [lsblk_node(
                "/dev/sdz", "disk", "8:240", 16_384, (true, false), Vec::new(), Vec::new()
            )]
        })
        .to_string();
        assert!(
            parse_lsblk_devices(no_root.as_bytes(), &request.image, false, &BTreeSet::new())
                .is_err()
        );
        let ambiguous_root = serde_json::json!({
            "blockdevices": [
                lsblk_node(
                    "/dev/sda", "disk", "8:0", 8_192, (false, false), vec!["/"], Vec::new()
                ),
                lsblk_node(
                    "/dev/sdb", "disk", "8:16", 8_192, (false, false), vec!["/"], Vec::new()
                )
            ]
        })
        .to_string();
        assert!(
            parse_lsblk_devices(
                ambiguous_root.as_bytes(),
                &request.image,
                false,
                &BTreeSet::new()
            )
            .is_err()
        );
        assert!(
            parse_lsblk_devices(
                &vec![b' '; MAX_WIC_DEVICE_JSON_BYTES as usize + 1],
                &request.image,
                false,
                &BTreeSet::new()
            )
            .is_err()
        );
        assert!(matches!(
            WicDeviceInspector::with_program(directory.join("missing-lsblk"))
                .without_device_node_validation_for_tests()
                .discover(request.clone())
                .await,
            Err(WicAdapterError::MissingDeviceTool(_))
        ));
        let image_link = directory.join("linked-image.wic");
        std::os::unix::fs::symlink(&request.image.path, &image_link).unwrap();
        let mut linked_request = request.clone();
        linked_request.image.path = image_link;
        assert!(matches!(
            inspector.discover(linked_request).await,
            Err(WicAdapterError::UnsafeImage(_))
        ));
        assert!(!validate_device_node(&directory.join("linked-image.wic")));
        executable(
            &lsblk,
            &format!("printf '%s' '{}'", inventory.replace('\'', "'\\''")),
        );
        fs::remove_file(&write_request.executable).unwrap();
        assert!(matches!(
            inspector.command_for(&write_request).await,
            Err(WicAdapterError::UnsafeExecutable(_))
        ));
        fs::write(&request.image.path, b"changed-size").unwrap();
        assert!(matches!(
            inspector.discover(request).await,
            Err(WicAdapterError::UnsafeImage(_))
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_device_write_discovery_bounds_process_failures_and_timeouts() {
        let inventory = device_inventory_json(vec![lsblk_node(
            "/dev/sdz",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        )]);
        let (directory, _, inspector, request, lsblk) =
            device_write_fixture("device-discovery-failure", &inventory, "exit 0");
        executable(&lsblk, "printf 'permission denied' >&2; exit 7");
        assert!(matches!(
            inspector.discover(request.clone()).await,
            Err(WicAdapterError::DeviceDiscovery(message))
                if message == "permission denied"
        ));
        executable(&lsblk, "exec sleep 30");
        assert!(matches!(
            inspector
                .with_inspection_timeout(Duration::from_millis(20))
                .discover(request)
                .await,
            Err(WicAdapterError::DeviceDiscovery(message))
                if message == "lsblk timed out"
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_device_write_runner_streams_bounds_fails_and_cancels() {
        let inventory = device_inventory_json(vec![lsblk_node(
            "/dev/sdz",
            "disk",
            "8:240",
            16_384,
            (true, false),
            Vec::new(),
            Vec::new(),
        )]);
        let (directory, wic, inspector, request, _) =
            device_write_fixture("device-runner", &inventory, "printf 'writing\\n'; exit 0");
        let response = inspector.discover(request.clone()).await.unwrap();
        let write_request = WicWriteRequest {
            executable: wic,
            image: request.image,
            device: response.devices[0].identity.clone(),
        };
        let mut runner = WicJobRunner::new(directory.clone());
        runner.start_write(&inspector, write_request).await.unwrap();
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Starting);
        assert_eq!(runner.next_event().await.unwrap(), WicRunnerEvent::Started);
        let mut saw_output = false;
        loop {
            match runner.next_event().await.unwrap() {
                WicRunnerEvent::Output { line, .. } => saw_output |= line == "writing",
                WicRunnerEvent::Completed { outputs, .. } => {
                    assert!(outputs.is_empty());
                    break;
                }
                _ => {}
            }
        }
        assert!(saw_output);
        fs::remove_dir_all(directory).unwrap();

        let (directory, wic, inspector, request, _) = device_write_fixture(
            "device-runner-failure",
            &inventory,
            "dd if=/dev/zero bs=70000 count=1 2>/dev/null | tr '\\000' x; printf '\\nfailed\\n' >&2; exit 9",
        );
        let response = inspector.discover(request.clone()).await.unwrap();
        let mut runner = WicJobRunner::new(directory.clone());
        runner
            .start_write(
                &inspector,
                WicWriteRequest {
                    executable: wic,
                    image: request.image,
                    device: response.devices[0].identity.clone(),
                },
            )
            .await
            .unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        let mut saw_truncated = false;
        let terminal = loop {
            let event = runner.next_event().await.unwrap();
            match event {
                WicRunnerEvent::Output { truncated, .. } => saw_truncated |= truncated,
                WicRunnerEvent::Failed { .. } => break event,
                _ => {}
            }
        };
        assert!(saw_truncated);
        assert!(matches!(
            terminal,
            WicRunnerEvent::Failed {
                exit_code: Some(9),
                ..
            }
        ));
        fs::remove_dir_all(directory).unwrap();

        let (directory, wic, inspector, request, _) = device_write_fixture(
            "device-runner-cancel",
            &inventory,
            "trap '' TERM; printf 'ready\\n'; while :; do sleep 1; done",
        );
        let response = inspector.discover(request.clone()).await.unwrap();
        let mut runner = WicJobRunner::new(directory.clone())
            .with_cancellation_timeout(Duration::from_millis(50));
        runner
            .start_write(
                &inspector,
                WicWriteRequest {
                    executable: wic,
                    image: request.image,
                    device: response.devices[0].identity.clone(),
                },
            )
            .await
            .unwrap();
        let _ = runner.next_event().await.unwrap();
        let _ = runner.next_event().await.unwrap();
        loop {
            if matches!(
                runner.next_event().await.unwrap(),
                WicRunnerEvent::Output { ref line, .. } if line == "ready"
            ) {
                break;
            }
        }
        assert!(runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            WicRunnerEvent::Cancelled { forced: true, .. }
        ));
        fs::remove_dir_all(directory).unwrap();
    }
}
