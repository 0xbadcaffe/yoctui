use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    ffi::{OsStr, OsString},
    fs, io,
    path::{Component, Path, PathBuf},
    process::Stdio,
    time::{Duration, UNIX_EPOCH},
};

use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
    time::Instant,
};
use yoctui_model::{
    SdkArtifactIdentity, SdkNativeMode, SdkNativePreview, SdkNativeRequest, SdkOperation,
    SdkOutputStream, SdkPublishPreview, SdkToolCapability,
};

use crate::output_text;

const SDK_TOOL_NAMES: [&str; 3] = ["oe-publish-sdk", "oe-find-native-sysroot", "oe-run-native"];
const MAX_SDK_TOOL_ROOTS: usize = 32;
const MAX_SDK_ENVIRONMENT_BYTES: u64 = 256 * 1024;
const MAX_SDK_ENVIRONMENT_VARIABLES: usize = 256;
const MAX_SDK_ENVIRONMENT_VALUE_BYTES: usize = 8 * 1024;
const MAX_SDK_TOOL_LINE_BYTES: usize = 64 * 1024;
const SDK_TOOL_EVENT_CHANNEL_CAPACITY: usize = 256;
const SDK_TOOL_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SdkToolAdapterError {
    #[error("SDK tool workspace root is unsafe: {0}")]
    UnsafeWorkspaceRoot(PathBuf),
    #[error("SDK tool executable is unsafe or unavailable: {0}")]
    UnsafeTool(PathBuf),
    #[error("SDK tool request is invalid: {0}")]
    InvalidRequest(String),
    #[error("SDK tool preview does not match its independently reconstructed command")]
    PreviewMismatch,
    #[error("SDK installer identity is stale or unsafe: {0}")]
    UnsafeInstaller(PathBuf),
    #[error("SDK publication destination is unsafe or not empty: {0}")]
    UnsafeDestination(PathBuf),
    #[error("SDK build directory is unsafe: {0}")]
    UnsafeBuildDirectory(PathBuf),
    #[error("extracted SDK root is unsafe: {0}")]
    UnsafeExtractedRoot(PathBuf),
    #[error("extracted SDK environment setup is invalid: {0}")]
    InvalidEnvironment(String),
    #[error("an SDK tool process or unconsumed event is already active")]
    Busy,
    #[error("could not start SDK tool: {0}")]
    Spawn(String),
    #[error("SDK tool process stream is unavailable: {0:?}")]
    StreamUnavailable(SdkOutputStream),
    #[error("SDK tool runner is not active")]
    NotRunning,
    #[error("SDK tool process control failed: {0}")]
    ProcessControl(String),
}

#[derive(Debug, Clone)]
pub struct SdkToolCapabilityInspector {
    workspace_roots: Vec<PathBuf>,
}

impl SdkToolCapabilityInspector {
    pub fn new(workspace_roots: Vec<PathBuf>) -> Self {
        Self { workspace_roots }
    }

    pub fn inspect(&self) -> SdkToolCapability {
        let roots = match validate_workspace_roots(&self.workspace_roots) {
            Ok(roots) => roots,
            Err(error) => {
                return SdkToolCapability::Failed {
                    message: error.to_string(),
                };
            }
        };
        let mut tools = BTreeMap::new();
        for name in SDK_TOOL_NAMES {
            match discover_tool(&roots, name) {
                Ok(tool) => {
                    tools.insert(name, tool);
                }
                Err(error) => {
                    return SdkToolCapability::Failed {
                        message: error.to_string(),
                    };
                }
            }
        }
        SdkToolCapability::Available {
            publish: tools.remove("oe-publish-sdk").flatten(),
            find_sysroot: tools.remove("oe-find-native-sysroot").flatten(),
            run_native: tools.remove("oe-run-native").flatten(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SdkToolAdapter {
    build_directory: PathBuf,
    sdk_deploy_root: PathBuf,
    workspace_roots: Vec<PathBuf>,
}

impl SdkToolAdapter {
    pub fn new(
        build_directory: PathBuf,
        sdk_deploy_root: PathBuf,
        workspace_roots: Vec<PathBuf>,
    ) -> Self {
        Self {
            build_directory,
            sdk_deploy_root,
            workspace_roots,
        }
    }

    pub fn capability(&self) -> SdkToolCapability {
        SdkToolCapabilityInspector::new(self.workspace_roots.clone()).inspect()
    }

    pub fn publication_command(
        &self,
        preview: &SdkPublishPreview,
    ) -> Result<SdkToolCommandSpec, SdkToolAdapterError> {
        let roots = validate_workspace_roots(&self.workspace_roots)?;
        let deploy_root = validate_exact_directory(
            &self.sdk_deploy_root,
            SdkToolAdapterError::UnsafeInstaller(self.sdk_deploy_root.clone()),
        )?;
        validate_named_tool(&preview.request.executable, "oe-publish-sdk", &roots)?;
        validate_installer(&preview.request.artifact, &deploy_root)?;
        validate_empty_destination(&preview.request.destination)?;
        let expected = SdkPublishPreview::new(
            preview.request.executable.clone(),
            preview.request.artifact.clone(),
            preview.request.destination.clone(),
        )
        .map_err(|message| SdkToolAdapterError::InvalidRequest(message.into()))?;
        if expected != *preview {
            return Err(SdkToolAdapterError::PreviewMismatch);
        }
        Ok(SdkToolCommandSpec {
            operation: SdkOperation::Publish(preview.request.clone()),
            executable: preview.request.executable.clone(),
            arguments: vec![
                preview.request.artifact.path.as_os_str().to_owned(),
                preview.request.destination.as_os_str().to_owned(),
            ],
            current_directory: preview.request.destination.clone(),
            environment: BTreeMap::new(),
            clear_environment: false,
            environment_setup: None,
            allowed_roots: roots,
            sdk_deploy_root: Some(deploy_root),
        })
    }

    pub fn native_command(
        &self,
        preview: &SdkNativePreview,
    ) -> Result<SdkToolCommandSpec, SdkToolAdapterError> {
        let roots = validate_workspace_roots(&self.workspace_roots)?;
        let expected = SdkNativePreview::new(preview.request.clone())
            .map_err(|message| SdkToolAdapterError::InvalidRequest(message.into()))?;
        if expected != *preview {
            return Err(SdkToolAdapterError::PreviewMismatch);
        }
        let tool_name = match preview.request.mode {
            SdkNativeMode::FindSysroot => "oe-find-native-sysroot",
            SdkNativeMode::RunNative => "oe-run-native",
        };
        validate_named_tool(&preview.request.executable, tool_name, &roots)?;
        let (current_directory, environment, clear_environment, environment_setup) =
            if let Some(extracted_root) = &preview.request.extracted_root {
                let root = validate_exact_directory(
                    extracted_root,
                    SdkToolAdapterError::UnsafeExtractedRoot(extracted_root.clone()),
                )?;
                let setup = find_environment_setup(&root)?;
                let environment = parse_environment_setup(&setup.path)?;
                (root, environment, true, Some(setup))
            } else {
                let build = validate_exact_directory(
                    &self.build_directory,
                    SdkToolAdapterError::UnsafeBuildDirectory(self.build_directory.clone()),
                )?;
                (build, BTreeMap::new(), false, None)
            };
        let mut arguments = vec![OsString::from(&preview.request.recipe)];
        if let Some(tool) = &preview.request.tool {
            arguments.push(OsString::from(tool));
        }
        arguments.extend(preview.request.arguments.iter().map(OsString::from));
        Ok(SdkToolCommandSpec {
            operation: SdkOperation::Native(preview.request.clone()),
            executable: preview.request.executable.clone(),
            arguments,
            current_directory,
            environment,
            clear_environment,
            environment_setup,
            allowed_roots: roots,
            sdk_deploy_root: None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SdkEnvironmentSetupIdentity {
    path: PathBuf,
    size_bytes: u64,
    modified_unix_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdkToolCommandSpec {
    operation: SdkOperation,
    executable: PathBuf,
    arguments: Vec<OsString>,
    current_directory: PathBuf,
    environment: BTreeMap<OsString, OsString>,
    clear_environment: bool,
    environment_setup: Option<SdkEnvironmentSetupIdentity>,
    allowed_roots: Vec<PathBuf>,
    sdk_deploy_root: Option<PathBuf>,
}

impl SdkToolCommandSpec {
    pub fn operation(&self) -> &SdkOperation {
        &self.operation
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }

    pub fn arguments(&self) -> &[OsString] {
        &self.arguments
    }

    pub fn current_directory(&self) -> &Path {
        &self.current_directory
    }

    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    pub fn clears_environment(&self) -> bool {
        self.clear_environment
    }

    fn revalidate(&self) -> Result<(), SdkToolAdapterError> {
        let expected_name = match &self.operation {
            SdkOperation::Publish(_) => "oe-publish-sdk",
            SdkOperation::Native(request) => match request.mode {
                SdkNativeMode::FindSysroot => "oe-find-native-sysroot",
                SdkNativeMode::RunNative => "oe-run-native",
            },
        };
        validate_named_tool(&self.executable, expected_name, &self.allowed_roots)?;
        match &self.operation {
            SdkOperation::Publish(request) => {
                let Some(root) = &self.sdk_deploy_root else {
                    return Err(SdkToolAdapterError::PreviewMismatch);
                };
                validate_installer(&request.artifact, root)?;
                validate_empty_destination(&request.destination)?;
                if self.current_directory != request.destination
                    || self.arguments
                        != [
                            request.artifact.path.as_os_str().to_owned(),
                            request.destination.as_os_str().to_owned(),
                        ]
                {
                    return Err(SdkToolAdapterError::PreviewMismatch);
                }
            }
            SdkOperation::Native(request) => {
                let expected = native_arguments(request);
                if self.arguments != expected {
                    return Err(SdkToolAdapterError::PreviewMismatch);
                }
                if let Some(root) = &request.extracted_root {
                    let canonical = validate_exact_directory(
                        root,
                        SdkToolAdapterError::UnsafeExtractedRoot(root.clone()),
                    )?;
                    if self.current_directory != canonical || !self.clear_environment {
                        return Err(SdkToolAdapterError::PreviewMismatch);
                    }
                    let setup = find_environment_setup(&canonical)?;
                    let environment = parse_environment_setup(&setup.path)?;
                    if self.environment_setup.as_ref() != Some(&setup)
                        || self.environment != environment
                    {
                        return Err(SdkToolAdapterError::InvalidEnvironment(
                            "environment setup changed after preview".into(),
                        ));
                    }
                } else {
                    validate_exact_directory(
                        &self.current_directory,
                        SdkToolAdapterError::UnsafeBuildDirectory(self.current_directory.clone()),
                    )?;
                    if self.clear_environment
                        || self.environment_setup.is_some()
                        || !self.environment.is_empty()
                    {
                        return Err(SdkToolAdapterError::PreviewMismatch);
                    }
                }
            }
        }
        Ok(())
    }
}

fn native_arguments(request: &SdkNativeRequest) -> Vec<OsString> {
    let mut arguments = vec![OsString::from(&request.recipe)];
    if let Some(tool) = &request.tool {
        arguments.push(OsString::from(tool));
    }
    arguments.extend(request.arguments.iter().map(OsString::from));
    arguments
}

fn validate_workspace_roots(
    workspace_roots: &[PathBuf],
) -> Result<Vec<PathBuf>, SdkToolAdapterError> {
    if workspace_roots.is_empty() || workspace_roots.len() > MAX_SDK_TOOL_ROOTS {
        return Err(SdkToolAdapterError::InvalidRequest(format!(
            "SDK tool roots must contain between 1 and {MAX_SDK_TOOL_ROOTS} entries"
        )));
    }
    let mut roots = Vec::new();
    for root in workspace_roots {
        roots.push(validate_exact_directory(
            root,
            SdkToolAdapterError::UnsafeWorkspaceRoot(root.clone()),
        )?);
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn discover_tool(roots: &[PathBuf], name: &str) -> Result<Option<PathBuf>, SdkToolAdapterError> {
    let mut found = BTreeSet::new();
    for root in roots {
        for candidate in [root.join("scripts").join(name), root.join(name)] {
            match fs::symlink_metadata(&candidate) {
                Ok(_) => {
                    found.insert(validate_executable(&candidate)?);
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(_) => return Err(SdkToolAdapterError::UnsafeTool(candidate)),
            }
        }
    }
    Ok(found.pop_first())
}

fn validate_named_tool(
    path: &Path,
    expected_name: &str,
    roots: &[PathBuf],
) -> Result<PathBuf, SdkToolAdapterError> {
    if path.file_name() != Some(OsStr::new(expected_name)) {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    let canonical = validate_executable(path)?;
    if !roots
        .iter()
        .any(|root| canonical.starts_with(root) && canonical != *root)
    {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    Ok(canonical)
}

fn validate_executable(path: &Path) -> Result<PathBuf, SdkToolAdapterError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| SdkToolAdapterError::UnsafeTool(path.into()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    let canonical =
        fs::canonicalize(path).map_err(|_| SdkToolAdapterError::UnsafeTool(path.into()))?;
    if canonical != path {
        return Err(SdkToolAdapterError::UnsafeTool(path.into()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(SdkToolAdapterError::UnsafeTool(path.into()));
        }
    }
    Ok(canonical)
}

fn validate_exact_directory(
    path: &Path,
    error: SdkToolAdapterError,
) -> Result<PathBuf, SdkToolAdapterError> {
    if !path.is_absolute()
        || path == Path::new("/")
        || path
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return Err(error);
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| error.clone())?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error);
    }
    let canonical = fs::canonicalize(path).map_err(|_| error.clone())?;
    if canonical != path {
        return Err(error);
    }
    Ok(canonical)
}

fn validate_installer(
    identity: &SdkArtifactIdentity,
    deploy_root: &Path,
) -> Result<(), SdkToolAdapterError> {
    identity
        .validate()
        .map_err(|_| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    if !identity.path.starts_with(deploy_root)
        || identity.path == deploy_root
        || identity.path.extension() != Some(OsStr::new("sh"))
    {
        return Err(SdkToolAdapterError::UnsafeInstaller(identity.path.clone()));
    }
    let metadata = fs::symlink_metadata(&identity.path)
        .map_err(|_| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SdkToolAdapterError::UnsafeInstaller(identity.path.clone()));
    }
    let canonical = fs::canonicalize(&identity.path)
        .map_err(|_| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    let modified = modified_seconds(&metadata)
        .ok_or_else(|| SdkToolAdapterError::UnsafeInstaller(identity.path.clone()))?;
    if canonical != identity.path
        || metadata.len() != identity.size_bytes
        || modified != identity.modified_unix_seconds
    {
        return Err(SdkToolAdapterError::UnsafeInstaller(identity.path.clone()));
    }
    Ok(())
}

fn validate_empty_destination(path: &Path) -> Result<PathBuf, SdkToolAdapterError> {
    let destination =
        validate_exact_directory(path, SdkToolAdapterError::UnsafeDestination(path.into()))?;
    let mut entries = fs::read_dir(&destination)
        .map_err(|_| SdkToolAdapterError::UnsafeDestination(path.into()))?;
    if entries.next().is_some() {
        return Err(SdkToolAdapterError::UnsafeDestination(path.into()));
    }
    Ok(destination)
}

fn find_environment_setup(root: &Path) -> Result<SdkEnvironmentSetupIdentity, SdkToolAdapterError> {
    let mut matches = Vec::new();
    let entries =
        fs::read_dir(root).map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
    for entry in entries {
        let entry = entry.map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !name.starts_with("environment-setup-") {
            continue;
        }
        let metadata = fs::symlink_metadata(&path)
            .map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(SdkToolAdapterError::UnsafeExtractedRoot(root.into()));
        }
        let canonical = fs::canonicalize(&path)
            .map_err(|_| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        if canonical != path || canonical.parent() != Some(root) {
            return Err(SdkToolAdapterError::UnsafeExtractedRoot(root.into()));
        }
        let modified_unix_seconds = modified_seconds(&metadata)
            .ok_or_else(|| SdkToolAdapterError::UnsafeExtractedRoot(root.into()))?;
        matches.push(SdkEnvironmentSetupIdentity {
            path: canonical,
            size_bytes: metadata.len(),
            modified_unix_seconds,
        });
    }
    matches.sort_by(|left, right| left.path.cmp(&right.path));
    if matches.len() != 1 {
        return Err(SdkToolAdapterError::InvalidEnvironment(format!(
            "expected exactly one environment-setup-* file, found {}",
            matches.len()
        )));
    }
    Ok(matches.remove(0))
}

fn modified_seconds(metadata: &fs::Metadata) -> Option<u64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn parse_environment_setup(
    path: &Path,
) -> Result<BTreeMap<OsString, OsString>, SdkToolAdapterError> {
    let metadata = fs::metadata(path)
        .map_err(|error| SdkToolAdapterError::InvalidEnvironment(error.to_string()))?;
    if metadata.len() > MAX_SDK_ENVIRONMENT_BYTES {
        return Err(SdkToolAdapterError::InvalidEnvironment(format!(
            "environment setup exceeds {MAX_SDK_ENVIRONMENT_BYTES} bytes"
        )));
    }
    let bytes = fs::read(path)
        .map_err(|error| SdkToolAdapterError::InvalidEnvironment(error.to_string()))?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        SdkToolAdapterError::InvalidEnvironment("environment setup is not UTF-8".into())
    })?;
    let mut values = BTreeMap::<String, String>::new();
    if let Ok(path) = std::env::var("PATH")
        && path.len() <= MAX_SDK_ENVIRONMENT_VALUE_BYTES
        && path.is_ascii()
        && !path.chars().any(char::is_control)
    {
        values.insert("PATH".into(), path);
    }
    let mut exported = 0_usize;
    for (index, raw_line) in text.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let assignment = line.strip_prefix("export ").ok_or_else(|| {
            SdkToolAdapterError::InvalidEnvironment(format!(
                "unsupported environment statement on line {}",
                index + 1
            ))
        })?;
        let (name, raw_value) = assignment.split_once('=').ok_or_else(|| {
            SdkToolAdapterError::InvalidEnvironment(format!(
                "malformed environment assignment on line {}",
                index + 1
            ))
        })?;
        if !valid_environment_name(name) || exported >= MAX_SDK_ENVIRONMENT_VARIABLES {
            return Err(SdkToolAdapterError::InvalidEnvironment(format!(
                "invalid or excessive environment variable on line {}",
                index + 1
            )));
        }
        let value = parse_environment_value(raw_value.trim(), &values).map_err(|message| {
            SdkToolAdapterError::InvalidEnvironment(format!("{message} on line {}", index + 1))
        })?;
        if value.len() > MAX_SDK_ENVIRONMENT_VALUE_BYTES || value.chars().any(char::is_control) {
            return Err(SdkToolAdapterError::InvalidEnvironment(format!(
                "environment value exceeds its bound on line {}",
                index + 1
            )));
        }
        values.insert(name.into(), value);
        exported = exported.saturating_add(1);
    }
    if exported == 0 {
        return Err(SdkToolAdapterError::InvalidEnvironment(
            "environment setup contained no supported exports".into(),
        ));
    }
    Ok(values
        .into_iter()
        .map(|(name, value)| (OsString::from(name), OsString::from(value)))
        .collect())
}

fn valid_environment_name(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && characters.all(|character| character == '_' || character.is_ascii_alphanumeric())
}

fn parse_environment_value(
    value: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, &'static str> {
    if !value.is_ascii() {
        return Err("non-ASCII environment values are unsupported");
    }
    if value.is_empty() {
        return Ok(String::new());
    }
    let (value, expand) = if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
        (&value[1..value.len() - 1], false)
    } else if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
        (&value[1..value.len() - 1], true)
    } else {
        if value.chars().any(|character| {
            character.is_whitespace()
                || matches!(character, ';' | '|' | '&' | '<' | '>' | '`' | '\\')
        }) {
            return Err("unsafe unquoted environment value");
        }
        (value, true)
    };
    if !expand {
        return Ok(value.into());
    }
    expand_environment_value(value, variables)
}

fn expand_environment_value(
    value: &str,
    variables: &BTreeMap<String, String>,
) -> Result<String, &'static str> {
    let characters = value.as_bytes();
    let mut expanded = String::new();
    let mut index = 0;
    while index < characters.len() {
        if characters[index] != b'$' {
            expanded.push(characters[index] as char);
            index += 1;
            continue;
        }
        index += 1;
        let (name, next) = if characters.get(index) == Some(&b'{') {
            let start = index + 1;
            let Some(end_offset) = characters[start..].iter().position(|byte| *byte == b'}') else {
                return Err("unterminated environment expansion");
            };
            let end = start + end_offset;
            (&value[start..end], end + 1)
        } else {
            let start = index;
            while index < characters.len()
                && (characters[index] == b'_'
                    || (characters[index] as char).is_ascii_alphanumeric())
            {
                index += 1;
            }
            if start == index {
                return Err("unsupported environment expansion");
            }
            (&value[start..index], index)
        };
        if !valid_environment_name(name) {
            return Err("invalid environment expansion");
        }
        let replacement = variables
            .get(name)
            .ok_or("environment expansion referenced an unavailable variable")?;
        expanded.push_str(replacement);
        index = next;
    }
    Ok(expanded)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SdkToolRunnerEvent {
    Started,
    Output {
        stream: SdkOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: Option<i32>,
    },
    Failed {
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    TimedOut {
        forced: bool,
        exit_code: Option<i32>,
    },
    Lost {
        message: String,
    },
}

#[derive(Debug)]
enum SdkToolPipeEvent {
    Output {
        stream: SdkOutputStream,
        line: String,
        truncated: bool,
    },
    Failed {
        stream: SdkOutputStream,
        message: String,
    },
}

async fn read_sdk_tool_output<R>(
    stream: R,
    kind: SdkOutputStream,
    sender: tokio::sync::mpsc::Sender<SdkToolPipeEvent>,
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
                let _ = sender
                    .send(SdkToolPipeEvent::Failed {
                        stream: kind,
                        message: error.to_string(),
                    })
                    .await;
                break;
            }
        };
        if buffer.is_empty() {
            if !bytes.is_empty() || truncated {
                let _ = sender
                    .send(SdkToolPipeEvent::Output {
                        stream: kind,
                        line: output_text(&bytes),
                        truncated,
                    })
                    .await;
            }
            break;
        }
        let newline = buffer.iter().position(|byte| *byte == b'\n');
        let take = newline.unwrap_or(buffer.len());
        if !truncated {
            let remaining = MAX_SDK_TOOL_LINE_BYTES.saturating_sub(bytes.len());
            bytes.extend_from_slice(&buffer[..take.min(remaining)]);
            truncated = take > remaining;
        }
        reader.consume(take + usize::from(newline.is_some()));
        if newline.is_some() {
            if sender
                .send(SdkToolPipeEvent::Output {
                    stream: kind,
                    line: output_text(&bytes),
                    truncated,
                })
                .await
                .is_err()
            {
                break;
            }
            bytes.clear();
            truncated = false;
        }
    }
}

pub struct SdkToolJobRunner {
    child: Option<Child>,
    output: Option<tokio::sync::mpsc::Receiver<SdkToolPipeEvent>>,
    streams_drained: bool,
    started_pending: bool,
    terminal_pending: VecDeque<SdkToolRunnerEvent>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    deadline: Option<Instant>,
    cancellation_requested: bool,
    #[cfg(unix)]
    process_group: Option<i32>,
}

impl Default for SdkToolJobRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl SdkToolJobRunner {
    pub fn new() -> Self {
        Self {
            child: None,
            output: None,
            streams_drained: true,
            started_pending: false,
            terminal_pending: VecDeque::new(),
            cancellation_timeout: Duration::from_secs(5),
            operation_timeout: SDK_TOOL_OPERATION_TIMEOUT,
            deadline: None,
            cancellation_requested: false,
            #[cfg(unix)]
            process_group: None,
        }
    }

    pub fn with_cancellation_timeout(mut self, timeout: Duration) -> Self {
        self.cancellation_timeout = timeout;
        self
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn is_active(&self) -> bool {
        self.child.is_some()
    }

    pub async fn start(&mut self, command: SdkToolCommandSpec) -> Result<(), SdkToolAdapterError> {
        if self.child.is_some()
            || self.started_pending
            || !self.terminal_pending.is_empty()
            || self.output.is_some()
        {
            return Err(SdkToolAdapterError::Busy);
        }
        command.revalidate()?;
        let mut process = Command::new(&command.executable);
        process
            .args(&command.arguments)
            .current_dir(&command.current_directory)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if command.clear_environment {
            process.env_clear();
        }
        process.envs(&command.environment);
        #[cfg(unix)]
        process.process_group(0);
        let mut child = process
            .spawn()
            .map_err(|error| SdkToolAdapterError::Spawn(error.to_string()))?;
        #[cfg(unix)]
        {
            self.process_group = child.id().map(|id| id as i32);
        }
        let Some(stdout) = child.stdout.take() else {
            let _ = child.kill().await;
            #[cfg(unix)]
            {
                self.process_group = None;
            }
            return Err(SdkToolAdapterError::StreamUnavailable(
                SdkOutputStream::Stdout,
            ));
        };
        let Some(stderr) = child.stderr.take() else {
            let _ = child.kill().await;
            #[cfg(unix)]
            {
                self.process_group = None;
            }
            return Err(SdkToolAdapterError::StreamUnavailable(
                SdkOutputStream::Stderr,
            ));
        };
        let (sender, receiver) = tokio::sync::mpsc::channel(SDK_TOOL_EVENT_CHANNEL_CAPACITY);
        tokio::spawn(read_sdk_tool_output(
            stdout,
            SdkOutputStream::Stdout,
            sender.clone(),
        ));
        tokio::spawn(read_sdk_tool_output(
            stderr,
            SdkOutputStream::Stderr,
            sender.clone(),
        ));
        drop(sender);
        self.child = Some(child);
        self.output = Some(receiver);
        self.streams_drained = false;
        self.started_pending = true;
        self.deadline = Some(Instant::now() + self.operation_timeout);
        self.cancellation_requested = false;
        Ok(())
    }

    pub async fn next_event(&mut self) -> Result<SdkToolRunnerEvent, SdkToolAdapterError> {
        if self.started_pending {
            self.started_pending = false;
            return Ok(SdkToolRunnerEvent::Started);
        }
        if let Some(event) = self.terminal_pending.pop_front() {
            return Ok(event);
        }
        if self.output.is_none() && !self.streams_drained && self.child.is_some() {
            self.kill_and_clear().await;
            return Ok(SdkToolRunnerEvent::Lost {
                message: "SDK tool output event channel was lost".into(),
            });
        }
        if self
            .deadline
            .is_some_and(|deadline| Instant::now() >= deadline)
        {
            return self.timeout_active().await;
        }
        if let Some(receiver) = self.output.as_mut() {
            let deadline = self.deadline.ok_or(SdkToolAdapterError::NotRunning)?;
            let event = tokio::select! {
                event = receiver.recv() => Some(event),
                _ = tokio::time::sleep_until(deadline) => None,
            };
            match event {
                Some(Some(SdkToolPipeEvent::Output {
                    stream,
                    line,
                    truncated,
                })) => {
                    return Ok(SdkToolRunnerEvent::Output {
                        stream,
                        line,
                        truncated,
                    });
                }
                Some(Some(SdkToolPipeEvent::Failed { stream, message })) => {
                    self.kill_and_clear().await;
                    return Ok(SdkToolRunnerEvent::Lost {
                        message: format!("{stream:?} stream failed: {message}"),
                    });
                }
                Some(None) => {
                    self.output = None;
                    self.streams_drained = true;
                }
                None => return self.timeout_active().await,
            }
        }
        let deadline = self.deadline.ok_or(SdkToolAdapterError::NotRunning)?;
        let status = {
            let child = self.child.as_mut().ok_or(SdkToolAdapterError::NotRunning)?;
            match tokio::time::timeout_at(deadline, child.wait()).await {
                Ok(Ok(status)) => status,
                Ok(Err(error)) => {
                    self.kill_and_clear().await;
                    return Ok(SdkToolRunnerEvent::Lost {
                        message: format!("SDK tool process wait failed: {error}"),
                    });
                }
                Err(_) => return self.timeout_active().await,
            }
        };
        self.clear_process_state();
        if status.success() {
            Ok(SdkToolRunnerEvent::Completed {
                exit_code: status.code(),
            })
        } else {
            Ok(SdkToolRunnerEvent::Failed {
                exit_code: status.code(),
            })
        }
    }

    pub async fn cancel(&mut self) -> Result<bool, SdkToolAdapterError> {
        if self.cancellation_requested || self.child.is_none() {
            self.terminal_pending
                .push_back(SdkToolRunnerEvent::CancellationRejected {
                    message: "no cancellable SDK tool process is active".into(),
                });
            return Ok(false);
        }
        self.cancellation_requested = true;
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        self.terminal_pending
            .push_back(SdkToolRunnerEvent::Cancelled {
                forced,
                exit_code: status.and_then(|status| status.code()),
            });
        Ok(true)
    }

    async fn timeout_active(&mut self) -> Result<SdkToolRunnerEvent, SdkToolAdapterError> {
        let (status, forced) = self.terminate_active().await?;
        self.clear_process_state();
        Ok(SdkToolRunnerEvent::TimedOut {
            forced,
            exit_code: status.and_then(|status| status.code()),
        })
    }

    async fn terminate_active(
        &mut self,
    ) -> Result<(Option<std::process::ExitStatus>, bool), SdkToolAdapterError> {
        let Some(child) = self.child.as_mut() else {
            return Ok((None, false));
        };
        let mut forced = false;
        #[cfg(unix)]
        let status = if let Some(process_group) = self.process_group {
            // SAFETY: the negative PID targets only the process group created for this child.
            if unsafe { libc::kill(-process_group, libc::SIGTERM) } != 0 {
                child
                    .start_kill()
                    .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?;
                forced = true;
            }
            match tokio::time::timeout(self.cancellation_timeout, child.wait()).await {
                Ok(result) => Some(
                    result
                        .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?,
                ),
                Err(_) => {
                    // SAFETY: same child-owned process group as the graceful signal.
                    let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
                    forced = true;
                    Some(
                        child.wait().await.map_err(|error| {
                            SdkToolAdapterError::ProcessControl(error.to_string())
                        })?,
                    )
                }
            }
        } else {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?,
            )
        };
        #[cfg(not(unix))]
        let status = {
            forced = true;
            child
                .kill()
                .await
                .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?;
            Some(
                child
                    .wait()
                    .await
                    .map_err(|error| SdkToolAdapterError::ProcessControl(error.to_string()))?,
            )
        };
        Ok((status, forced))
    }

    async fn kill_and_clear(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        self.clear_process_state();
    }

    fn clear_process_state(&mut self) {
        self.child = None;
        self.output = None;
        self.streams_drained = true;
        self.deadline = None;
        self.cancellation_requested = false;
        #[cfg(unix)]
        {
            self.process_group = None;
        }
    }

    #[cfg(test)]
    fn lose_output_channel(&mut self) {
        self.output = None;
    }
}

impl Drop for SdkToolJobRunner {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(process_group) = self.process_group {
            // SAFETY: this is the child-owned process group created by `start`.
            let _ = unsafe { libc::kill(-process_group, libc::SIGKILL) };
        }
        if let Some(child) = self.child.as_mut() {
            let _ = child.start_kill();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    #[cfg(unix)]
    use std::os::unix::fs::{PermissionsExt, symlink};

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(1);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "yoctui-sdk-tool-{name}-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
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

    fn executable(path: &Path, body: &str) {
        fs::write(path, body).unwrap();
        #[cfg(unix)]
        {
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(path, permissions).unwrap();
        }
    }

    fn fixture(name: &str) -> (TestDirectory, SdkToolAdapter) {
        let directory = TestDirectory::new(name);
        let workspace = directory.path().join("workspace");
        let scripts = workspace.join("scripts");
        let build = directory.path().join("build");
        let deploy = directory.path().join("deploy/sdk");
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&build).unwrap();
        fs::create_dir_all(&deploy).unwrap();
        for tool in SDK_TOOL_NAMES {
            executable(&scripts.join(tool), "#!/bin/sh\nprintf '%s\\n' \"$@\"\n");
        }
        let adapter = SdkToolAdapter::new(build, deploy, vec![workspace]);
        (directory, adapter)
    }

    fn artifact(path: &Path) -> SdkArtifactIdentity {
        let metadata = fs::metadata(path).unwrap();
        SdkArtifactIdentity {
            path: path.into(),
            size_bytes: metadata.len(),
            modified_unix_seconds: modified_seconds(&metadata).unwrap(),
        }
    }

    fn publish_preview(adapter: &SdkToolAdapter, directory: &TestDirectory) -> SdkPublishPreview {
        let installer = adapter.sdk_deploy_root.join("poky-toolchain.sh");
        fs::write(&installer, b"installer").unwrap();
        let destination = directory.path().join("published");
        fs::create_dir(&destination).unwrap();
        let executable = match adapter.capability() {
            SdkToolCapability::Available {
                publish: Some(path),
                ..
            } => path,
            capability => panic!("unexpected capability: {capability:?}"),
        };
        SdkPublishPreview::new(executable, artifact(&installer), destination).unwrap()
    }

    fn native_preview(
        adapter: &SdkToolAdapter,
        mode: SdkNativeMode,
        extracted_root: Option<PathBuf>,
    ) -> SdkNativePreview {
        let capability = adapter.capability();
        let executable = capability.executable_for(mode).unwrap();
        SdkNativePreview::new(SdkNativeRequest {
            executable,
            mode,
            extracted_root,
            recipe: "cmake-native".into(),
            tool: (mode == SdkNativeMode::RunNative).then(|| "cmake".into()),
            arguments: if mode == SdkNativeMode::RunNative {
                vec!["--version".into()]
            } else {
                Vec::new()
            },
        })
        .unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn sdk_tool_capability_is_partial_and_rejects_unsafe_candidates() {
        let directory = TestDirectory::new("capability");
        let workspace = directory.path().join("workspace");
        let scripts = workspace.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        executable(&scripts.join("oe-publish-sdk"), "#!/bin/sh\nexit 0\n");
        let inspector = SdkToolCapabilityInspector::new(vec![workspace.clone()]);
        assert!(matches!(
            inspector.inspect(),
            SdkToolCapability::Available {
                publish: Some(_),
                find_sysroot: None,
                run_native: None,
            }
        ));

        let outside = directory.path().join("outside");
        executable(&outside, "#!/bin/sh\nexit 0\n");
        symlink(&outside, scripts.join("oe-run-native")).unwrap();
        assert!(matches!(
            inspector.inspect(),
            SdkToolCapability::Failed { .. }
        ));
        assert!(matches!(
            SdkToolCapabilityInspector::new(vec![directory.path().join("missing")]).inspect(),
            SdkToolCapability::Failed { .. }
        ));
    }

    #[test]
    fn sdk_tool_commands_reconstruct_exact_publication_and_native_argv() {
        let (directory, adapter) = fixture("commands");
        let publish = publish_preview(&adapter, &directory);
        let publish_command = adapter.publication_command(&publish).unwrap();
        assert_eq!(
            publish_command.arguments(),
            [
                publish.request.artifact.path.as_os_str(),
                publish.request.destination.as_os_str(),
            ]
        );

        let native = native_preview(&adapter, SdkNativeMode::RunNative, None);
        let native_command = adapter.native_command(&native).unwrap();
        assert_eq!(
            native_command.arguments(),
            ["cmake-native", "cmake", "--version"]
        );
        assert_eq!(native_command.current_directory(), adapter.build_directory);
        assert!(!native_command.clears_environment());

        let mut tampered = native;
        tampered.argv.push("injected".into());
        assert_eq!(
            adapter.native_command(&tampered),
            Err(SdkToolAdapterError::PreviewMismatch)
        );
    }

    #[cfg(unix)]
    #[test]
    fn sdk_tool_commands_reject_unsafe_paths_and_stale_installer() {
        let (directory, adapter) = fixture("unsafe");
        let mut preview = publish_preview(&adapter, &directory);
        preview.argv.push("injected".into());
        assert_eq!(
            adapter.publication_command(&preview),
            Err(SdkToolAdapterError::PreviewMismatch)
        );
        preview.argv.pop();
        fs::write(&preview.request.artifact.path, b"changed installer").unwrap();
        assert!(matches!(
            adapter.publication_command(&preview),
            Err(SdkToolAdapterError::UnsafeInstaller(_))
        ));

        let other = directory.path().join("other-tool");
        executable(&other, "#!/bin/sh\nexit 0\n");
        let mut native = native_preview(&adapter, SdkNativeMode::FindSysroot, None);
        native.request.executable = other.clone();
        native.argv[0] = other;
        assert!(matches!(
            adapter.native_command(&native),
            Err(SdkToolAdapterError::UnsafeTool(_))
        ));

        let destination = directory.path().join("nonempty");
        fs::create_dir(&destination).unwrap();
        fs::write(destination.join("existing"), b"data").unwrap();
        let installer = adapter.sdk_deploy_root.join("fresh.sh");
        fs::write(&installer, b"installer").unwrap();
        let executable = adapter.capability().publish_executable().unwrap();
        let nonempty =
            SdkPublishPreview::new(executable, artifact(&installer), destination).unwrap();
        assert!(matches!(
            adapter.publication_command(&nonempty),
            Err(SdkToolAdapterError::UnsafeDestination(_))
        ));

        let extracted = directory.path().join("real-extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-core2"),
            "export SDK_ROOT='/opt/sdk'\n",
        )
        .unwrap();
        let linked = directory.path().join("linked-extracted");
        symlink(&extracted, &linked).unwrap();
        let linked_preview = native_preview(&adapter, SdkNativeMode::FindSysroot, Some(linked));
        assert!(matches!(
            adapter.native_command(&linked_preview),
            Err(SdkToolAdapterError::UnsafeExtractedRoot(_))
        ));
    }

    #[test]
    fn sdk_tool_extracted_environment_is_validated_and_child_only() {
        let (directory, adapter) = fixture("environment");
        let extracted = directory.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-core2-64-poky-linux"),
            "export SDK_ROOT='/opt/sdk'\nexport SDK_BIN=\"$SDK_ROOT/bin\"\n",
        )
        .unwrap();
        let preview = native_preview(&adapter, SdkNativeMode::RunNative, Some(extracted.clone()));
        let command = adapter.native_command(&preview).unwrap();
        assert!(command.clears_environment());
        assert_eq!(
            command.environment().get(OsStr::new("SDK_BIN")),
            Some(&OsString::from("/opt/sdk/bin"))
        );
        assert!(!command.environment().contains_key(OsStr::new("HOME")));

        fs::write(
            extracted.join("environment-setup-second"),
            "export SECOND='value'\n",
        )
        .unwrap();
        assert!(matches!(
            adapter.native_command(&preview),
            Err(SdkToolAdapterError::InvalidEnvironment(_))
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_confines_extracted_environment_to_the_child() {
        let (directory, adapter) = fixture("child-environment");
        let extracted = directory.path().join("extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-core2-64-poky-linux"),
            "export SDK_ROOT='/opt/sdk'\nexport SDK_BIN=\"$SDK_ROOT/bin\"\n",
        )
        .unwrap();
        let preview = native_preview(&adapter, SdkNativeMode::RunNative, Some(extracted.clone()));
        executable(
            &preview.request.executable,
            "#!/bin/sh\nprintf 'SDK_BIN=%s HOME=%s\\n' \"$SDK_BIN\" \"${HOME-unset}\"\n",
        );
        let preview = SdkNativePreview::new(preview.request).unwrap();
        let command = adapter.native_command(&preview).unwrap();
        let mut runner = SdkToolJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        let output = match runner.next_event().await.unwrap() {
            SdkToolRunnerEvent::Output { line, .. } => line,
            event => panic!("unexpected runner event: {event:?}"),
        };
        assert_eq!(output, "SDK_BIN=/opt/sdk/bin HOME=unset");
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Completed { exit_code: Some(0) }
        ));
        assert!(std::env::var_os("SDK_BIN").is_none());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_streams_bounded_output_and_terminal_status() {
        let (directory, adapter) = fixture("runner-output");
        let publish = publish_preview(&adapter, &directory);
        let tool = publish.request.executable.clone();
        executable(
            &tool,
            &format!(
                "#!/bin/sh\nprintf 'stdout\\n'\nprintf 'stderr\\n' >&2\nprintf '{}\\n'\nexit 0\n",
                "x".repeat(MAX_SDK_TOOL_LINE_BYTES + 8)
            ),
        );
        let publish =
            SdkPublishPreview::new(tool, publish.request.artifact, publish.request.destination)
                .unwrap();
        let command = adapter.publication_command(&publish).unwrap();
        let mut runner = SdkToolJobRunner::new();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        let mut saw_stdout = false;
        let mut saw_stderr = false;
        let mut saw_truncated = false;
        loop {
            match runner.next_event().await.unwrap() {
                SdkToolRunnerEvent::Output {
                    stream, truncated, ..
                } => {
                    saw_stdout |= stream == SdkOutputStream::Stdout;
                    saw_stderr |= stream == SdkOutputStream::Stderr;
                    saw_truncated |= truncated;
                }
                SdkToolRunnerEvent::Completed { exit_code } => {
                    assert_eq!(exit_code, Some(0));
                    break;
                }
                event => panic!("unexpected runner event: {event:?}"),
            }
        }
        assert!(saw_stdout && saw_stderr && saw_truncated);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_rejects_duplicate_and_reports_nonzero_and_loss() {
        let (directory, adapter) = fixture("runner-outcomes");
        let preview = publish_preview(&adapter, &directory);
        executable(&preview.request.executable, "#!/bin/sh\nexit 7\n");
        let preview = SdkPublishPreview::new(
            preview.request.executable,
            preview.request.artifact,
            preview.request.destination,
        )
        .unwrap();
        let command = adapter.publication_command(&preview).unwrap();
        let mut runner = SdkToolJobRunner::new();
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.start(command.clone()).await,
            Err(SdkToolAdapterError::Busy)
        );
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Failed { exit_code: Some(7) }
        );

        executable(&preview.request.executable, "#!/bin/sh\nsleep 2\n");
        let command = adapter.publication_command(&preview).unwrap();
        runner.start(command).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        runner.lose_output_channel();
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Lost { .. }
        ));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_tool_runner_times_out_and_cancels_gracefully_or_forcibly() {
        let (directory, adapter) = fixture("runner-control");
        let preview = publish_preview(&adapter, &directory);
        executable(
            &preview.request.executable,
            "#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n",
        );
        let preview = SdkPublishPreview::new(
            preview.request.executable,
            preview.request.artifact,
            preview.request.destination,
        )
        .unwrap();
        let command = adapter.publication_command(&preview).unwrap();
        let mut runner = SdkToolJobRunner::new().with_cancellation_timeout(Duration::from_secs(1));
        runner.start(command.clone()).await.unwrap();
        assert_eq!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert!(runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::Cancelled { forced: false, .. }
        ));
        assert!(!runner.cancel().await.unwrap());
        assert!(matches!(
            runner.next_event().await.unwrap(),
            SdkToolRunnerEvent::CancellationRejected { .. }
        ));

        executable(
            &preview.request.executable,
            "#!/bin/sh\ntrap '' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let command = adapter.publication_command(&preview).unwrap();
        let mut forced_cancel =
            SdkToolJobRunner::new().with_cancellation_timeout(Duration::from_millis(20));
        forced_cancel.start(command.clone()).await.unwrap();
        assert_eq!(
            forced_cancel.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert!(matches!(
            forced_cancel.next_event().await.unwrap(),
            SdkToolRunnerEvent::Output { .. }
        ));
        assert!(forced_cancel.cancel().await.unwrap());
        assert!(matches!(
            forced_cancel.next_event().await.unwrap(),
            SdkToolRunnerEvent::Cancelled { forced: true, .. }
        ));

        let mut timed_out = SdkToolJobRunner::new()
            .with_cancellation_timeout(Duration::from_millis(20))
            .with_operation_timeout(Duration::from_millis(20));
        timed_out.start(command).await.unwrap();
        assert_eq!(
            timed_out.next_event().await.unwrap(),
            SdkToolRunnerEvent::Started
        );
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            SdkToolRunnerEvent::Output { .. }
        ));
        assert!(matches!(
            timed_out.next_event().await.unwrap(),
            SdkToolRunnerEvent::TimedOut { forced: true, .. }
        ));
    }
}
