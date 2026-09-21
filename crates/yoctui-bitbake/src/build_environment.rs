use std::{
    collections::BTreeMap,
    path::PathBuf,
    process::{Output, Stdio},
    time::Duration,
};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::{
    process::Command,
    time::{error::Elapsed, timeout},
};
use yoctui_model::{BuildEnvironmentCloneRequest, BuildEnvironmentProfile};
use yoctui_utils::{is_transient_spawn_error, path_entry_exists};

const MAX_OUTPUT: usize = 64 * 1024;
const SPAWN_ATTEMPTS: usize = 4;
const SPAWN_RETRY_DELAY: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildEnvironmentResponse {
    pub profile: BuildEnvironmentProfile,
    pub environment: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildEnvironmentClonePreview {
    pub clone_argv: Vec<String>,
    pub checkout_argv: Option<Vec<String>>,
    pub destination: PathBuf,
}

#[derive(Debug, Error)]
pub enum BuildEnvironmentAdapterError {
    #[error("unsafe build environment path: {0}")]
    UnsafePath(PathBuf),
    #[error("missing build environment path: {0}")]
    MissingPath(PathBuf),
    #[error("environment script is not executable: {0}")]
    NotExecutable(PathBuf),
    #[error("environment setup requires interactive input")]
    InteractiveRequired,
    #[error("environment setup timed out")]
    Timeout,
    #[error("environment setup failed: {0}")]
    Failed(String),
    #[error("environment output exceeded the adapter bound")]
    OutputTooLarge,
    #[error("environment output was not valid UTF-8")]
    InvalidOutput,
    #[error("clone destination is not empty: {0}")]
    DestinationNotEmpty(PathBuf),
    #[error("clone destination parent is unavailable: {0}")]
    DestinationParent(PathBuf),
}

#[derive(Debug, Clone)]
pub struct BuildEnvironmentAdapter {
    timeout: Duration,
    git_program: PathBuf,
}

impl Default for BuildEnvironmentAdapter {
    fn default() -> Self {
        Self::new(Duration::from_secs(30))
    }
}

impl BuildEnvironmentAdapter {
    pub fn new(timeout: Duration) -> Self {
        Self {
            timeout,
            git_program: PathBuf::from("git"),
        }
    }

    pub fn with_git_program(mut self, program: impl Into<PathBuf>) -> Self {
        self.git_program = program.into();
        self
    }

    pub fn preview_clone(
        &self,
        request: &BuildEnvironmentCloneRequest,
    ) -> Result<BuildEnvironmentClonePreview, BuildEnvironmentAdapterError> {
        request
            .validate()
            .map_err(|_| BuildEnvironmentAdapterError::UnsafePath(request.destination.clone()))?;
        validate_clone_ancestors(&request.destination)?;
        if path_entry_exists(&request.destination)
            .map_err(|_| BuildEnvironmentAdapterError::UnsafePath(request.destination.clone()))?
        {
            if request.destination.is_symlink() {
                return Err(BuildEnvironmentAdapterError::UnsafePath(
                    request.destination.clone(),
                ));
            }
            if request
                .destination
                .read_dir()
                .map_err(|_| {
                    BuildEnvironmentAdapterError::DestinationNotEmpty(request.destination.clone())
                })?
                .next()
                .is_some()
            {
                return Err(BuildEnvironmentAdapterError::DestinationNotEmpty(
                    request.destination.clone(),
                ));
            }
        }
        let mut clone_argv = vec![
            "clone".into(),
            request.repository.clone(),
            request.destination.display().to_string(),
        ];
        if request.revision.is_some() {
            clone_argv.insert(1, "--no-checkout".into());
        }
        let checkout_argv = request.revision.as_ref().map(|revision| {
            vec![
                "-C".into(),
                request.destination.display().to_string(),
                "checkout".into(),
                revision.clone(),
            ]
        });
        Ok(BuildEnvironmentClonePreview {
            clone_argv,
            checkout_argv,
            destination: request.destination.clone(),
        })
    }

    pub async fn clone_poky(
        &self,
        request: BuildEnvironmentCloneRequest,
    ) -> Result<BuildEnvironmentClonePreview, BuildEnvironmentAdapterError> {
        let preview = self.preview_clone(&request)?;
        if let Some(parent) = request.destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|_| BuildEnvironmentAdapterError::DestinationParent(parent.to_owned()))?;
        }
        // Recheck after creating the reviewed parent chain, before executing Git.
        self.preview_clone(&request)?;
        let output = self.run_git(&preview.clone_argv).await?;
        if output.stdout.len() > MAX_OUTPUT || output.stderr.len() > MAX_OUTPUT {
            return Err(BuildEnvironmentAdapterError::OutputTooLarge);
        }
        if !output.status.success() {
            return Err(BuildEnvironmentAdapterError::Failed(
                String::from_utf8_lossy(&output.stderr).trim().to_owned(),
            ));
        }
        if let Some(checkout) = &preview.checkout_argv {
            let output = self.run_git(checkout).await?;
            if !output.status.success() {
                return Err(BuildEnvironmentAdapterError::Failed(
                    String::from_utf8_lossy(&output.stderr).trim().to_owned(),
                ));
            }
        }
        Ok(preview)
    }

    async fn run_git(&self, argv: &[String]) -> Result<Output, BuildEnvironmentAdapterError> {
        for attempt in 1..=SPAWN_ATTEMPTS {
            let mut command = Command::new(&self.git_program);
            command
                .args(argv)
                .env("GIT_TERMINAL_PROMPT", "0")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            #[cfg(unix)]
            command.process_group(0);
            let mut child = match command.spawn() {
                Ok(child) => child,
                Err(error) if attempt < SPAWN_ATTEMPTS && is_transient_spawn_error(&error) => {
                    tokio::time::sleep(SPAWN_RETRY_DELAY).await;
                    continue;
                }
                Err(error) => return Err(BuildEnvironmentAdapterError::Failed(error.to_string())),
            };
            let mut group = child.id().map(yoctui_utils::ProcessGroupGuard::new);
            let stdout = child.stdout.take().expect("piped stdout");
            let stderr = child.stderr.take().expect("piped stderr");
            let collect = async {
                let (status, stdout, stderr) = tokio::join!(
                    child.wait(),
                    bounded_git_output(stdout),
                    bounded_git_output(stderr)
                );
                Ok::<_, std::io::Error>(Output {
                    status: status?,
                    stdout: stdout?,
                    stderr: stderr?,
                })
            };
            let result = timeout(self.timeout, collect).await;
            match result {
                Ok(Ok(output)) => {
                    if let Some(group) = &mut group {
                        group.disarm();
                    }
                    return Ok(output);
                }
                result => {
                    drop(group);
                    let _ = child.kill().await;
                    let _ = child.wait().await;
                    return Err(match result {
                        Err(_) => BuildEnvironmentAdapterError::Timeout,
                        Ok(Err(error)) => BuildEnvironmentAdapterError::Failed(error.to_string()),
                        _ => unreachable!(),
                    });
                }
            }
        }
        unreachable!("bounded retry loop returns")
    }

    pub fn validate(
        &self,
        profile: &BuildEnvironmentProfile,
    ) -> Result<BuildEnvironmentProfile, BuildEnvironmentAdapterError> {
        profile
            .validate()
            .map_err(|_| BuildEnvironmentAdapterError::UnsafePath(profile.build_dir.clone()))?;
        for (path, may_create) in [(&profile.source_dir, false), (&profile.build_dir, true)] {
            if may_create
                && !path_entry_exists(path)
                    .map_err(|_| BuildEnvironmentAdapterError::UnsafePath(path.clone()))?
            {
                // oe-init-build-env creates the reviewed build directory. Validation
                // must not require it to exist or create it ahead of the script.
                if path.parent().is_some_and(|parent| parent.is_dir()) {
                    continue;
                }
                return Err(BuildEnvironmentAdapterError::MissingPath(path.clone()));
            }
            if !path.is_dir() {
                return Err(BuildEnvironmentAdapterError::MissingPath(path.clone()));
            }
            if path.is_symlink() {
                return Err(BuildEnvironmentAdapterError::UnsafePath(path.clone()));
            }
        }
        let script = &profile.init_script;
        if !script.is_file() {
            return Err(BuildEnvironmentAdapterError::MissingPath(script.clone()));
        }
        if script.is_symlink() {
            return Err(BuildEnvironmentAdapterError::UnsafePath(script.clone()));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if std::fs::metadata(script)
                .map(|metadata| metadata.permissions().mode() & 0o111 == 0)
                .unwrap_or(true)
            {
                return Err(BuildEnvironmentAdapterError::NotExecutable(script.clone()));
            }
        }
        Ok(profile.clone())
    }

    pub async fn initialize(
        &self,
        profile: BuildEnvironmentProfile,
    ) -> Result<BuildEnvironmentResponse, BuildEnvironmentAdapterError> {
        let profile = self.validate(&profile)?;
        let script = r#"
set -e
source "$1" "$2"
env -0
"#;
        let mut command = Command::new("bash");
        command
            .arg("-c")
            .arg(script)
            .arg("yoctui-init")
            .arg(&profile.init_script)
            .arg(&profile.build_dir)
            .current_dir(&profile.source_dir)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        #[cfg(unix)]
        command.process_group(0);
        let child = command
            .spawn()
            .map_err(|error| BuildEnvironmentAdapterError::Failed(error.to_string()))?;
        let mut group = child.id().map(yoctui_utils::ProcessGroupGuard::new);
        let output = timeout(self.timeout, child.wait_with_output())
            .await
            .map_err(|_: Elapsed| BuildEnvironmentAdapterError::Timeout)?
            .map_err(|error| BuildEnvironmentAdapterError::Failed(error.to_string()))?;
        if let Some(group) = &mut group {
            group.disarm();
        }
        if output.stdout.len() > MAX_OUTPUT || output.stderr.len() > MAX_OUTPUT {
            return Err(BuildEnvironmentAdapterError::OutputTooLarge);
        }
        if !output.status.success() {
            let diagnostic = String::from_utf8(output.stderr)
                .map_err(|_| BuildEnvironmentAdapterError::InvalidOutput)?;
            if looks_interactive(&diagnostic) {
                return Err(BuildEnvironmentAdapterError::InteractiveRequired);
            }
            return Err(BuildEnvironmentAdapterError::Failed(
                diagnostic.trim().to_owned(),
            ));
        }
        let bytes = output.stdout;
        let mut environment = BTreeMap::new();
        for record in bytes
            .split(|byte| *byte == 0)
            .filter(|record| !record.is_empty())
        {
            let record = std::str::from_utf8(record)
                .map_err(|_| BuildEnvironmentAdapterError::InvalidOutput)?;
            let Some((key, value)) = record.split_once('=') else {
                continue;
            };
            if valid_environment_key(key) {
                environment.insert(key.to_owned(), value.to_owned());
            }
        }
        if !environment.contains_key("BUILDDIR") {
            environment.insert("BUILDDIR".into(), profile.build_dir.display().to_string());
        }
        Ok(BuildEnvironmentResponse {
            profile,
            environment,
        })
    }
}

fn validate_clone_ancestors(
    destination: &std::path::Path,
) -> Result<(), BuildEnvironmentAdapterError> {
    for ancestor in destination.ancestors().skip(1) {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if metadata.is_symlink() || !metadata.is_dir() => {
                return Err(BuildEnvironmentAdapterError::UnsafePath(
                    ancestor.to_owned(),
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => {
                return Err(BuildEnvironmentAdapterError::DestinationParent(
                    ancestor.to_owned(),
                ));
            }
        }
    }
    Ok(())
}

async fn bounded_git_output(
    mut stream: impl tokio::io::AsyncRead + Unpin,
) -> std::io::Result<Vec<u8>> {
    let mut output = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let count = stream.read(&mut buffer).await?;
        if count == 0 {
            return Ok(output);
        }
        let keep = count.min(MAX_OUTPUT.saturating_sub(output.len()));
        output.extend_from_slice(&buffer[..keep]);
    }
}

fn valid_environment_key(key: &str) -> bool {
    !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_')
}

fn looks_interactive(stderr: &str) -> bool {
    let lower = stderr.to_ascii_lowercase();
    [
        "(y/n)",
        "[y/n]",
        "password:",
        "select ",
        "press enter",
        "choice:",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

#[cfg(test)]
#[path = "tests/build_environment/mod.rs"]
mod tests;
