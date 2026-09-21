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
const SDK_TOOL_SPAWN_ATTEMPTS: usize = 4;
const SDK_TOOL_SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[cfg(unix)]
fn is_transient_sdk_tool_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
fn is_transient_sdk_tool_spawn_error(_error: &io::Error) -> bool {
    false
}

async fn spawn_sdk_tool_process(process: &mut Command) -> io::Result<Child> {
    for attempt in 1..=SDK_TOOL_SPAWN_ATTEMPTS {
        match process.spawn() {
            Ok(child) => return Ok(child),
            Err(error)
                if attempt < SDK_TOOL_SPAWN_ATTEMPTS
                    && is_transient_sdk_tool_spawn_error(&error) =>
            {
                tokio::time::sleep(SDK_TOOL_SPAWN_RETRY_DELAY).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the bounded SDK tool spawn loop always returns")
}

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
