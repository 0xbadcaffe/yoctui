use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::Terminal;
use serde::{Deserialize, Serialize};
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::{Duration, Instant},
};
#[cfg(unix)]
use std::{ffi::CString, os::unix::ffi::OsStrExt};
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use yoctui_app::{Input, key_action};
use yoctui_bitbake::{BackendEvent, BitBakeBackend, BridgeBackend, ProcessBackend};
use yoctui_model::{
    Action, AnimationSpeed, App, AppError, BuildRequest, BuildStatus, Effect, HostTelemetry,
    LayerBrowserEntry, LayerRelationship, LayerRelationships, RecipeDependencies, Screen, Severity,
    TaskId, TaskInfo, Theme, update,
};
use yoctui_ui::render;
#[derive(Parser, Debug)]
#[command(about = "A Ratatui frontend and control client for BitBake")]
struct Cli {
    #[arg(long, value_enum)]
    backend: Option<Backend>,
    #[arg(long)]
    build_dir: Option<PathBuf>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long)]
    log_level: Option<String>,
    #[arg(long)]
    no_color: bool,
    #[arg(long)]
    headless: bool,
    #[command(subcommand)]
    command: Option<Command>,
    targets: Vec<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum Backend {
    Bridge,
    Process,
}

impl std::fmt::Display for Backend {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Bridge => "bridge",
            Self::Process => "process",
        })
    }
}

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    backend: Option<Backend>,
    build_dir: Option<PathBuf>,
    log_retention_entries: Option<usize>,
    log_retention_bytes: Option<usize>,
    refresh_ms: Option<u64>,
    cancellation_timeout_ms: Option<u64>,
    default_target: Option<String>,
    editor: Option<String>,
    color: Option<bool>,
    theme: Option<Theme>,
    animation_speed: Option<AnimationSpeed>,
    reduced_motion: Option<bool>,
}

#[derive(Debug)]
struct Config {
    backend: Backend,
    build_dir: PathBuf,
    log_entries: usize,
    log_bytes: usize,
    refresh: Duration,
    cancellation_timeout: Duration,
    default_target: Option<String>,
    editor: Option<String>,
    log_level: String,
    color: bool,
    theme: Theme,
    animation_speed: AnimationSpeed,
    reduced_motion: bool,
    session_path: Option<PathBuf>,
}
#[derive(Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct Session {
    #[serde(default)]
    last_target: Option<String>,
    #[serde(default)]
    last_screen: Option<Screen>,
    #[serde(default)]
    log_filter: Option<Severity>,
    #[serde(default)]
    log_recipe_filter: Option<String>,
    #[serde(default)]
    log_task_filter: Option<String>,
    #[serde(default)]
    log_wrap: bool,
    #[serde(default)]
    last_backend: Option<Backend>,
    #[serde(default)]
    recent_build_dirs: Vec<PathBuf>,
}
#[derive(Subcommand, Debug)]
enum Command {
    Inspect,
    Build { targets: Vec<String> },
    Recipes,
    Layers,
    Config { name: String },
    Doctor,
}
struct TerminalGuard;
impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            Hide
        )?;
        Ok(Self)
    }
    fn suspend(&self) -> Result<()> {
        disable_raw_mode()?;
        execute!(
            io::stdout(),
            Show,
            DisableBracketedPaste,
            DisableMouseCapture,
            LeaveAlternateScreen
        )?;
        Ok(())
    }
    fn resume(&self) -> Result<()> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableMouseCapture,
            EnableBracketedPaste,
            Hide
        )?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CpuCounters {
    total: u64,
    idle: u64,
}

#[derive(Debug, Default)]
struct HostTelemetrySampler {
    previous_cpu: Option<CpuCounters>,
}

impl HostTelemetrySampler {
    fn sample(&mut self, build_dir: &Path) -> HostTelemetry {
        let current_cpu = read_cpu_counters();
        let cpu_utilization_percent = current_cpu.and_then(|current| {
            let previous = self.previous_cpu.replace(current)?;
            let total = current.total.saturating_sub(previous.total);
            let idle = current.idle.saturating_sub(previous.idle);
            (total > 0).then(|| {
                ((total.saturating_sub(idle) * 100) / total)
                    .min(100)
                    .try_into()
                    .unwrap_or(100)
            })
        });
        HostTelemetry {
            cpu_utilization_percent,
            disk_available_bytes: disk_available_bytes(build_dir),
        }
    }
}

fn read_cpu_counters() -> Option<CpuCounters> {
    let line = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .next()?
        .to_owned();
    parse_cpu_counters(&line)
}

fn parse_cpu_counters(line: &str) -> Option<CpuCounters> {
    let mut fields = line.split_whitespace();
    (fields.next()? == "cpu").then_some(())?;
    let values = fields
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    let total = values.iter().copied().sum();
    let idle = values.get(3).copied()? + values.get(4).copied().unwrap_or_default();
    Some(CpuCounters { total, idle })
}

#[cfg(unix)]
fn disk_available_bytes(path: &Path) -> Option<u64> {
    let path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat = std::mem::MaybeUninit::<libc::statvfs>::uninit();
    // SAFETY: `path` is a NUL-terminated C string and `stat` is valid writable storage.
    if unsafe { libc::statvfs(path.as_ptr(), stat.as_mut_ptr()) } != 0 {
        return None;
    }
    // SAFETY: a successful `statvfs` call initializes `stat`.
    let stat = unsafe { stat.assume_init() };
    Some(stat.f_bavail.saturating_mul(stat.f_frsize))
}

#[cfg(not(unix))]
fn disk_available_bytes(_path: &Path) -> Option<u64> {
    None
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore_terminal();
    }
}

fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(
        io::stdout(),
        Show,
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen
    );
}

fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        previous(info);
    }));
}
fn config_path(cli: &Cli) -> Option<PathBuf> {
    cli.config.clone().or_else(|| {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| env::var_os("HOME").map(|p| PathBuf::from(p).join(".config")))
            .map(|p| p.join("yoctui/config.toml"))
    })
}

fn read_file_config(path: Option<&Path>) -> Result<FileConfig> {
    let Some(path) = path else {
        return Ok(FileConfig::default());
    };
    if !path.exists() {
        return Ok(FileConfig::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("could not read configuration file {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("invalid configuration file {}", path.display()))
}

fn session_path(config: Option<&Path>) -> Option<PathBuf> {
    config
        .and_then(Path::parent)
        .map(|directory| directory.join("session.toml"))
}

fn read_session(path: Option<&Path>) -> Result<Session> {
    let Some(path) = path else {
        return Ok(Session::default());
    };
    if !path.exists() {
        return Ok(Session::default());
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("could not read session file {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("invalid session file {}", path.display()))
}

fn write_session(path: Option<&Path>, session: &Session) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory).with_context(|| {
            format!("could not create session directory {}", directory.display())
        })?;
    }
    fs::write(path, toml::to_string(session)?)
        .with_context(|| format!("could not write session file {}", path.display()))
}

fn env_usize(name: &str) -> Result<Option<usize>> {
    env::var(name)
        .ok()
        .map(|value| {
            value
                .parse()
                .with_context(|| format!("{name} must be a positive integer"))
        })
        .transpose()
}

fn resolve_config(cli: &Cli, session: &Session) -> Result<Config> {
    let configured_path = config_path(cli);
    let file = read_file_config(configured_path.as_deref())?;
    let environment_backend = env::var("YOCTUI_BACKEND")
        .ok()
        .map(|value| {
            Backend::from_str(&value, true)
                .map_err(|_| anyhow::anyhow!("YOCTUI_BACKEND must be bridge or process"))
        })
        .transpose()?;
    let backend = cli
        .backend
        .clone()
        .or(environment_backend)
        .or(file.backend)
        .or(session.last_backend.clone())
        .unwrap_or(Backend::Bridge);
    let build_dir = cli
        .build_dir
        .clone()
        .or_else(|| env::var_os("YOCTUI_BUILD_DIR").map(PathBuf::from))
        .or(file.build_dir)
        .or_else(|| {
            session
                .recent_build_dirs
                .iter()
                .find(|directory| directory.is_dir())
                .cloned()
        })
        .unwrap_or(env::current_dir()?);
    let log_entries = env_usize("YOCTUI_LOG_RETENTION_ENTRIES")?
        .or(file.log_retention_entries)
        .unwrap_or(10_000);
    let log_bytes = env_usize("YOCTUI_LOG_RETENTION_BYTES")?
        .or(file.log_retention_bytes)
        .unwrap_or(16 * 1024 * 1024);
    if log_entries == 0 || log_bytes == 0 {
        anyhow::bail!("log retention limits must be greater than zero");
    }
    let cancellation_timeout_ms = env_usize("YOCTUI_CANCELLATION_TIMEOUT_MS")?
        .map(u64::try_from)
        .transpose()?
        .or(file.cancellation_timeout_ms)
        .unwrap_or(5_000);
    if cancellation_timeout_ms == 0 {
        anyhow::bail!("cancellation timeout must be greater than zero");
    }
    Ok(Config {
        backend,
        build_dir,
        log_entries,
        log_bytes,
        refresh: Duration::from_millis(file.refresh_ms.unwrap_or(100).max(16)),
        cancellation_timeout: Duration::from_millis(cancellation_timeout_ms),
        default_target: env::var("YOCTUI_DEFAULT_TARGET")
            .ok()
            .or(file.default_target),
        editor: env::var("YOCTUI_EDITOR").ok().or(file.editor),
        log_level: cli
            .log_level
            .clone()
            .or_else(|| env::var("YOCTUI_LOG_LEVEL").ok())
            .unwrap_or_else(|| "info".into()),
        color: !cli.no_color && file.color.unwrap_or(true),
        theme: file.theme.unwrap_or_default(),
        animation_speed: file.animation_speed.unwrap_or_default(),
        reduced_motion: file.reduced_motion.unwrap_or(false),
        session_path: session_path(configured_path.as_deref()),
    })
}
#[tokio::main]
async fn main() -> Result<()> {
    install_panic_hook();
    let cli = Cli::parse();
    let session = read_session(session_path(config_path(&cli).as_deref()).as_deref())?;
    let config = resolve_config(&cli, &session)?;
    tracing_subscriber::fmt()
        .with_env_filter(config.log_level.clone())
        .with_writer(std::io::stderr)
        .init();
    let build_dir = config.build_dir.clone();
    if matches!(cli.command, Some(Command::Doctor)) {
        return doctor(&build_dir).await;
    }
    match &cli.command {
        Some(Command::Inspect) => {
            return inspect_workspace(config.backend.clone(), build_dir).await;
        }
        Some(Command::Recipes) => return print_recipes(config.backend.clone(), build_dir).await,
        Some(Command::Layers) => return print_layers(config.backend.clone(), build_dir).await,
        Some(Command::Config { name }) => {
            return print_variable(config.backend.clone(), build_dir, name).await;
        }
        Some(Command::Doctor) | Some(Command::Build { .. }) | None => {}
    }
    let targets = match &cli.command {
        Some(Command::Build { targets }) => targets.clone(),
        _ if !cli.targets.is_empty() => cli.targets.clone(),
        _ => config
            .default_target
            .clone()
            .or(session.last_target.clone())
            .into_iter()
            .collect(),
    };
    if cli.headless {
        return headless(
            config.backend,
            build_dir,
            targets,
            config.log_entries,
            config.log_bytes,
        )
        .await;
    }
    tui(config, targets, session).await
}

async fn load_workspace(backend: Backend, build_dir: PathBuf) -> Result<yoctui_model::Workspace> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.inspect_workspace().await;
    let shutdown = backend.shutdown().await;
    let workspace = result?;
    shutdown?;
    Ok(workspace)
}

async fn inspect_workspace(backend: Backend, build_dir: PathBuf) -> Result<()> {
    let workspace = load_workspace(backend, build_dir).await?;
    println!(
        "build directory: {}",
        workspace
            .build_dir
            .as_deref()
            .map_or_else(|| "unknown".into(), |path| path.display().to_string())
    );
    println!(
        "BitBake version: {}",
        workspace.bitbake_version.as_deref().unwrap_or("unknown")
    );
    println!(
        "Yocto/OpenEmbedded release: {}",
        workspace.release.as_deref().unwrap_or("unknown")
    );
    for (name, value) in workspace.variables {
        println!("{name}={value}");
    }
    Ok(())
}

async fn print_recipes(backend: Backend, build_dir: PathBuf) -> Result<()> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.list_recipes(None).await;
    let shutdown = backend.shutdown().await;
    let recipes = result?;
    shutdown?;
    for recipe in recipes {
        println!("{} {}", recipe.name, recipe.version.unwrap_or_default());
    }
    Ok(())
}

async fn print_layers(backend: Backend, build_dir: PathBuf) -> Result<()> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.list_layers().await;
    let shutdown = backend.shutdown().await;
    let layers = result?;
    shutdown?;
    for layer in layers {
        println!("{} {}", layer.name, layer.path.display());
    }
    Ok(())
}

async fn print_variable(backend: Backend, build_dir: PathBuf, name: &str) -> Result<()> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.get_variable(name.into(), None).await;
    let shutdown = backend.shutdown().await;
    let variable = result?;
    let value = variable
        .value
        .as_deref()
        .with_context(|| format!("{name} is not available from the selected backend"))?;
    shutdown?;
    println!("{name}={value}");
    if let Some(provenance) = variable.provenance {
        println!("provenance: {provenance}");
    }
    Ok(())
}
async fn doctor(build_dir: &Path) -> Result<()> {
    let initialized = std::env::var_os("BUILDDIR").is_some();
    let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
    let bitbake = tokio::process::Command::new("bitbake")
        .arg("--version")
        .output()
        .await;
    println!(
        "environment initialized: {}",
        if initialized {
            "yes"
        } else {
            "no — source oe-init-build-env"
        }
    );
    println!(
        "build directory: {} ({})",
        build_dir.display(),
        if build_dir.is_dir() {
            "usable"
        } else {
            "missing"
        }
    );
    match bitbake {
        Ok(o) => println!(
            "bitbake: {}",
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("available")
        ),
        Err(_) => {
            println!("bitbake: unavailable — source oe-init-build-env or add bitbake to PATH")
        }
    };
    match tokio::process::Command::new(&python)
        .args([
            "-c",
            "import bb; print(getattr(bb, '__version__', 'available'))",
        ])
        .output()
        .await
    {
        Ok(output) if output.status.success() => println!(
            "BitBake Python module: {}",
            String::from_utf8_lossy(&output.stdout).trim()
        ),
        Ok(_) | Err(_) => println!(
            "BitBake Python module: unavailable — source oe-init-build-env before starting Yoctui"
        ),
    }
    for f in ["conf/local.conf", "conf/bblayers.conf"] {
        println!(
            "{}: {}",
            f,
            if build_dir.join(f).is_file() {
                "present"
            } else {
                "not found (may be normal outside a build dir)"
            }
        )
    }
    match select_backend(Backend::Bridge, build_dir.to_path_buf()).await {
        Ok(mut bridge) => {
            let inspection = bridge.inspect_workspace().await;
            let shutdown = bridge.shutdown().await;
            match inspection {
                Ok(workspace) => println!(
                    "bridge protocol: ok (workspace: {})",
                    workspace
                        .build_dir
                        .as_deref()
                        .unwrap_or(build_dir)
                        .display()
                ),
                Err(error) => println!(
                    "bridge protocol: failed ({error}) — check the active Python/BitBake environment"
                ),
            }
            if let Err(error) = shutdown {
                println!("bridge shutdown: failed ({error})");
            }
        }
        Err(error) => {
            println!("bridge startup: failed ({error}) — check YOCTUI_BRIDGE_PATH and PYTHON")
        }
    }
    Ok(())
}
async fn headless(
    backend_kind: Backend,
    build_dir: PathBuf,
    targets: Vec<String>,
    log_entries: usize,
    log_bytes: usize,
) -> Result<()> {
    let mut backend = select_backend(backend_kind, build_dir.clone()).await?;
    let result = async {
        let mut app = App::new(log_entries, log_bytes);
        let workspace = backend.inspect_workspace().await?;
        let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
        if targets.is_empty() {
            println!("headless inspection completed");
            return Ok(());
        }
        backend
            .start_build(BuildRequest {
                targets,
                task: None,
            })
            .await
            .context("could not start bitbake")?;
        loop {
            match backend.next_event().await? {
                BackendEvent::Log(l) => {
                    println!("{}", l.message);
                    let _ = update(&mut app, Action::Log(l));
                }
                BackendEvent::BuildCompleted { success, exit_code } => {
                    let _ = update(&mut app, Action::BuildCompleted { success, exit_code });
                    println!(
                        "build {}{}",
                        if success { "completed" } else { "failed" },
                        exit_code.map_or_else(String::new, |code| format!(" (exit code {code})"))
                    );
                    if success {
                        return Ok(());
                    }
                    return Err(anyhow::anyhow!("BitBake build failed"));
                }
                event => {
                    if let Some(action) = action_from_event(event) {
                        let _ = update(&mut app, action);
                    }
                }
            }
        }
    }
    .await;
    let shutdown = backend.shutdown().await;
    result?;
    shutdown?;
    Ok(())
}

fn action_from_event(event: BackendEvent) -> Option<Action> {
    match event {
        BackendEvent::Workspace(workspace) => Some(Action::WorkspaceLoaded(workspace)),
        BackendEvent::BuildStarted => Some(Action::BuildStarted),
        BackendEvent::ParseProgress { current, total } => {
            Some(Action::ParseProgress { current, total })
        }
        BackendEvent::TaskStarted { recipe, task } => {
            let id = TaskId(format!("{recipe}:{task}"));
            Some(Action::TaskStarted(TaskInfo {
                id,
                recipe,
                task,
                progress: None,
            }))
        }
        BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => Some(Action::TaskProgress {
            id: TaskId(format!("{recipe}:{task}")),
            progress,
        }),
        BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        } => Some(Action::TaskCompleted {
            id: TaskId(format!("{recipe}:{task}")),
            success,
        }),
        BackendEvent::CommandFailed { code, message } => Some(Action::Failure(AppError::new(
            "BitBake",
            format!("{code}: {message}"),
            "inspect the bridge or BitBake diagnostics",
        ))),
        BackendEvent::Disconnected => Some(Action::Failure(AppError::new(
            "Bridge",
            "backend disconnected",
            "restart Yoctui and inspect the backend diagnostics",
        ))),
        BackendEvent::Recipes(_)
        | BackendEvent::Layers(_)
        | BackendEvent::Variable { .. }
        | BackendEvent::Dependencies { .. }
        | BackendEvent::RecipeSources { .. }
        | BackendEvent::LayerRelationships(_)
        | BackendEvent::Log(_)
        | BackendEvent::BuildCompleted { .. } => None,
    }
}

async fn select_backend(backend: Backend, build_dir: PathBuf) -> Result<Box<dyn BitBakeBackend>> {
    select_backend_with_timeout(backend, build_dir, None).await
}

async fn select_backend_with_timeout(
    backend: Backend,
    build_dir: PathBuf,
    cancellation_timeout: Option<Duration>,
) -> Result<Box<dyn BitBakeBackend>> {
    match backend {
        Backend::Process => {
            let backend = ProcessBackend::new(build_dir);
            let backend = if let Some(timeout) = cancellation_timeout {
                backend.with_cancellation_timeout(timeout)
            } else {
                backend
            };
            Ok(Box::new(backend))
        }
        Backend::Bridge => {
            let script = env::var_os("YOCTUI_BRIDGE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../..")
                        .join("bridge/yoctui_bridge.py")
                });
            let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
            BridgeBackend::spawn(&python, script, build_dir)
                .await
                .map(|backend| Box::new(backend) as Box<dyn BitBakeBackend>)
                .context("could not start the BitBake bridge; source oe-init-build-env or use --backend process")
        }
    }
}

async fn begin_build(backend: &mut Box<dyn BitBakeBackend>, app: &mut App, request: BuildRequest) {
    match backend.start_build(request).await {
        Ok(()) => {
            let _ = update(app, Action::BuildStarted);
        }
        Err(error) => {
            let _ = update(
                app,
                Action::Failure(AppError::new(
                    "BitBake",
                    error.to_string(),
                    "check backend diagnostics and retry",
                )),
            );
        }
    }
}

async fn open_in_editor(
    guard: &TerminalGuard,
    app: &mut App,
    path: PathBuf,
    preferred_editor: Option<&str>,
) {
    let editor = preferred_editor
        .map(Into::into)
        .or_else(|| env::var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into());
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for $EDITOR: {error}"
        ));
        return;
    }
    let editor_result =
        tokio::task::spawn_blocking(move || ProcessCommand::new(editor).arg(path).status()).await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after $EDITOR: {error}"
        ));
    } else if let Ok(Err(error)) = editor_result {
        app.notification = Some(format!("Could not start $EDITOR: {error}"));
    } else if let Err(error) = editor_result {
        app.notification = Some(format!("$EDITOR task failed: {error}"));
    }
}

async fn open_yocto_shell(guard: &TerminalGuard, app: &mut App) {
    let shell = env::var_os("SHELL").unwrap_or_else(|| "/bin/sh".into());
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for the Yocto shell: {error}"
        ));
        return;
    }
    let shell_result =
        tokio::task::spawn_blocking(move || ProcessCommand::new(shell).status()).await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after the Yocto shell: {error}"
        ));
    } else if let Ok(Err(error)) = shell_result {
        app.notification = Some(format!("Could not start the Yocto shell: {error}"));
    } else if let Err(error) = shell_result {
        app.notification = Some(format!("Yocto shell task failed: {error}"));
    } else {
        app.notification = Some("Returned from the inherited Yocto shell.".into());
    }
}

fn devtool_source_dir(build_dir: &Path, recipe: &str) -> PathBuf {
    build_dir.join("workspace").join("sources").join(recipe)
}

async fn devtool_modify(
    guard: &TerminalGuard,
    app: &mut App,
    build_dir: &Path,
    recipe: String,
) -> Option<PathBuf> {
    let source_dir = devtool_source_dir(build_dir, &recipe);
    let build_dir = build_dir.to_path_buf();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for devtool: {error}"
        ));
        return None;
    }
    let result = tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        if !source_dir.is_dir() {
            let status = ProcessCommand::new("devtool")
                .args(["modify", &recipe])
                .current_dir(build_dir)
                .status()
                .context("could not start devtool modify")?;
            if !status.success() {
                anyhow::bail!("devtool modify exited with {status}");
            }
        }
        Ok(source_dir)
    })
    .await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after devtool: {error}"
        ));
        None
    } else {
        match result {
            Ok(Ok(source_dir)) => Some(source_dir),
            Ok(Err(error)) => {
                app.notification =
                    Some(format!("Could not prepare the Devtool workspace: {error}"));
                None
            }
            Err(error) => {
                app.notification = Some(format!("Devtool task failed: {error}"));
                None
            }
        }
    }
}

fn recipe_editor_files(root: &Path) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
        for entry in fs::read_dir(directory)? {
            if files.len() >= 512 {
                break;
            }
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                if entry.file_name() != ".git" {
                    visit(root, &path, files)?;
                }
            } else if file_type.is_file()
                && entry.metadata()?.len() <= 1_048_576
                && let Ok(relative) = path.strip_prefix(root)
            {
                files.push(relative.to_path_buf());
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    visit(root, root, &mut files)?;
    files.sort();
    Ok(files)
}

async fn open_workspace_editor(app: &mut App, recipe: String, root: PathBuf) {
    let root_for_scan = root.clone();
    let files = tokio::task::spawn_blocking(move || recipe_editor_files(&root_for_scan)).await;
    match files {
        Ok(Ok(files)) => {
            if let Some(Effect::LoadRecipeEditorFile(path)) = update(
                app,
                Action::OpenRecipeEditor {
                    recipe,
                    root,
                    files,
                },
            ) {
                load_recipe_editor_file(app, path).await;
            }
        }
        Ok(Err(error)) => {
            app.notification = Some(format!("Could not list workspace files: {error}"))
        }
        Err(error) => app.notification = Some(format!("Workspace file scan failed: {error}")),
    }
}

async fn load_layer_browser_directory(
    app: &mut App,
    layer: String,
    root: PathBuf,
    directory: PathBuf,
) {
    let scan = directory.clone();
    match tokio::task::spawn_blocking(move || {
        let mut entries = fs::read_dir(&scan)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let is_dir = entry.file_type().ok()?.is_dir();
                let path = entry.path();
                (entry.file_name() != ".git").then_some(LayerBrowserEntry { path, is_dir })
            })
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| {
            (
                !entry.is_dir,
                entry.path.file_name().map(|name| name.to_owned()),
            )
        });
        Ok::<_, std::io::Error>(entries)
    })
    .await
    {
        Ok(Ok(entries)) => {
            if let Some(Effect::LoadLayerBrowserPreview(path)) = update(
                app,
                Action::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                    entries,
                },
            ) {
                load_layer_browser_preview(app, path).await;
            }
        }
        Ok(Err(error)) => {
            app.notification = Some(format!("Could not read layer directory: {error}"))
        }
        Err(error) => app.notification = Some(format!("Layer directory scan failed: {error}")),
    }
}

async fn load_layer_browser_preview(app: &mut App, path: PathBuf) {
    match tokio::task::spawn_blocking(move || {
        fs::read_to_string(path).map(|content| content.chars().take(24_000).collect::<String>())
    })
    .await
    {
        Ok(Ok(content)) => {
            let _ = update(app, Action::LoadLayerBrowserPreview(content));
        }
        Ok(Err(error)) => app.notification = Some(format!("Could not preview layer file: {error}")),
        Err(error) => app.notification = Some(format!("Layer preview failed: {error}")),
    }
}

async fn load_recipe_editor_file(app: &mut App, path: PathBuf) {
    let result = tokio::task::spawn_blocking(move || fs::read_to_string(path)).await;
    match result {
        Ok(Ok(content)) => {
            let _ = update(app, Action::LoadRecipeEditorContent(content));
        }
        Ok(Err(error)) => app.notification = Some(format!("Could not read recipe file: {error}")),
        Err(error) => app.notification = Some(format!("Recipe file load failed: {error}")),
    }
}

async fn save_recipe_editor_file(app: &mut App, path: PathBuf, content: String) {
    let result = tokio::task::spawn_blocking(move || fs::write(path, content)).await;
    match result {
        Ok(Ok(())) => {
            let _ = update(app, Action::RecipeEditorSaved);
        }
        Ok(Err(error)) => app.notification = Some(format!("Could not save recipe file: {error}")),
        Err(error) => app.notification = Some(format!("Recipe file save failed: {error}")),
    }
}

fn bbmask_assignment(value: &str) -> Result<String> {
    if value.contains(['\n', '\r']) {
        anyhow::bail!("BBMASK must be entered on one line");
    }
    Ok(format!(
        "BBMASK = \"{}\"",
        value.replace('\\', "\\\\").replace('"', "\\\"")
    ))
}

async fn write_bbmask(build_dir: &Path, value: String) -> Result<()> {
    let path = build_dir.join("conf").join("local.conf");
    tokio::task::spawn_blocking(move || -> Result<()> {
        let assignment = bbmask_assignment(&value)?;
        let mut content = fs::read_to_string(&path)
            .with_context(|| format!("could not read {}", path.display()))?;
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&assignment);
        content.push('\n');
        fs::write(&path, content).with_context(|| format!("could not write {}", path.display()))
    })
    .await
    .context("BBMASK write task failed")?
}

async fn refresh_workspace(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    success_message: &str,
) {
    match backend.inspect_workspace().await {
        Ok(workspace) => {
            let _ = update(app, Action::WorkspaceLoaded(workspace));
            match backend.list_recipes(None).await {
                Ok(mut recipes) => {
                    recipes.sort_by(|left, right| left.name.cmp(&right.name));
                    app.workspace.recipes = recipes;
                }
                Err(error) => app.notification = Some(format!("Recipes unavailable: {error}")),
            }
            match backend.list_layers().await {
                Ok(layers) => app.workspace.layers = layers,
                Err(error) => app.notification = Some(format!("Layers unavailable: {error}")),
            }
            if app.notification.is_none() {
                app.notification = Some(success_message.into());
            }
        }
        Err(error) => {
            app.notification = Some(format!(
                "BBMASK was saved, but the workspace refresh failed: {error}"
            ));
        }
    }
}

async fn devtool_update_recipe(
    guard: &TerminalGuard,
    app: &mut App,
    build_dir: &Path,
    recipe: String,
) -> bool {
    let build_dir = build_dir.to_path_buf();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for devtool: {error}"
        ));
        return false;
    }
    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        let status = ProcessCommand::new("devtool")
            .args(["update-recipe", &recipe])
            .current_dir(build_dir)
            .status()
            .context("could not start devtool update-recipe")?;
        if !status.success() {
            anyhow::bail!("devtool update-recipe exited with {status}");
        }
        Ok(())
    })
    .await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after devtool: {error}"
        ));
        false
    } else {
        match result {
            Ok(Ok(())) => true,
            Ok(Err(error)) => {
                app.notification = Some(format!("Could not update recipe via devtool: {error}"));
                false
            }
            Err(error) => {
                app.notification = Some(format!("Devtool task failed: {error}"));
                false
            }
        }
    }
}

async fn devtool_finish(
    guard: &TerminalGuard,
    app: &mut App,
    build_dir: &Path,
    request: yoctui_model::DevtoolFinishRequest,
) -> bool {
    if !request.destination.is_dir() {
        app.notification = Some(format!(
            "Devtool finish destination is not a directory: {}",
            request.destination.display()
        ));
        return false;
    }
    let build_dir = build_dir.to_path_buf();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for devtool: {error}"
        ));
        return false;
    }
    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        let status = ProcessCommand::new("devtool")
            .arg("finish")
            .arg(&request.recipe)
            .arg(&request.destination)
            .current_dir(build_dir)
            .status()
            .context("could not start devtool finish")?;
        if !status.success() {
            anyhow::bail!("devtool finish exited with {status}");
        }
        Ok(())
    })
    .await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after devtool: {error}"
        ));
        false
    } else {
        match result {
            Ok(Ok(())) => true,
            Ok(Err(error)) => {
                app.notification = Some(format!("Could not finish via devtool: {error}"));
                false
            }
            Err(error) => {
                app.notification = Some(format!("Devtool task failed: {error}"));
                false
            }
        }
    }
}

async fn devtool_deploy(
    guard: &TerminalGuard,
    app: &mut App,
    build_dir: &Path,
    request: yoctui_model::DevtoolDeployRequest,
) {
    let build_dir = build_dir.to_path_buf();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for devtool: {error}"
        ));
        return;
    }
    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        let status = ProcessCommand::new("devtool")
            .arg("deploy-target")
            .arg(&request.recipe)
            .arg(&request.target)
            .current_dir(build_dir)
            .status()
            .context("could not start devtool deploy-target")?;
        if !status.success() {
            anyhow::bail!("devtool deploy-target exited with {status}");
        }
        Ok(())
    })
    .await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after devtool: {error}"
        ));
    } else {
        match result {
            Ok(Ok(())) => {
                app.notification = Some("Devtool deployment completed.".into());
            }
            Ok(Err(error)) => {
                app.notification = Some(format!("Could not deploy via devtool: {error}"));
            }
            Err(error) => {
                app.notification = Some(format!("Devtool task failed: {error}"));
            }
        }
    }
}

async fn devtool_reset(guard: &TerminalGuard, app: &mut App, build_dir: &Path, recipe: String) {
    let build_dir = build_dir.to_path_buf();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for devtool: {error}"
        ));
        return;
    }
    let result = tokio::task::spawn_blocking(move || -> Result<()> {
        let status = ProcessCommand::new("devtool")
            .args(["reset", &recipe])
            .current_dir(build_dir)
            .status()
            .context("could not start devtool reset")?;
        if !status.success() {
            anyhow::bail!("devtool reset exited with {status}");
        }
        Ok(())
    })
    .await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after devtool: {error}"
        ));
    } else {
        match result {
            Ok(Ok(())) => app.notification = Some("Devtool workspace reset.".into()),
            Ok(Err(error)) => {
                app.notification = Some(format!("Could not reset via devtool: {error}"))
            }
            Err(error) => app.notification = Some(format!("Devtool task failed: {error}")),
        }
    }
}

async fn tui(config: Config, targets: Vec<String>, session: Session) -> Result<()> {
    let Config {
        backend: backend_kind,
        build_dir,
        log_entries,
        log_bytes,
        refresh,
        cancellation_timeout,
        color,
        theme,
        animation_speed,
        reduced_motion,
        editor,
        session_path,
        ..
    } = config;
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(log_entries, log_bytes);
    app.backend = backend_kind.to_string();
    app.color_enabled = color;
    app.theme = theme;
    app.animation_speed = animation_speed;
    app.reduced_motion = reduced_motion;
    app.screen = session.last_screen.unwrap_or(Screen::Dashboard);
    app.logs.filter = session.log_filter;
    app.logs.recipe_filter = session.log_recipe_filter;
    app.logs.task_filter = session.log_task_filter;
    app.logs.wrap = session.log_wrap;
    let session_build_dir = build_dir.clone();
    let mut backend =
        select_backend_with_timeout(backend_kind.clone(), build_dir, Some(cancellation_timeout))
            .await?;
    match backend.inspect_workspace().await {
        Ok(workspace) => {
            let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
            match backend.list_recipes(None).await {
                Ok(mut recipes) => {
                    recipes.sort_by(|left, right| left.name.cmp(&right.name));
                    app.workspace.recipes = recipes;
                }
                Err(error) => app.notification = Some(format!("Recipes unavailable: {error}")),
            }
            match backend.list_layers().await {
                Ok(layers) => app.workspace.layers = layers,
                Err(error) => app.notification = Some(format!("Layers unavailable: {error}")),
            }
        }
        Err(error) => {
            let _ = update(
                &mut app,
                Action::Failure(AppError::new(
                    "Backend",
                    error.to_string(),
                    "run `yoctui doctor` to diagnose the selected backend",
                )),
            );
        }
    }
    if !targets.is_empty() {
        app.build.target = targets.first().cloned()
    }
    let mut telemetry_sampler = HostTelemetrySampler::default();
    let mut next_telemetry_sample = Instant::now();
    #[cfg(unix)]
    let mut termination = termination_receiver()?;
    loop {
        #[cfg(unix)]
        if termination_requested(&mut termination) {
            break;
        }
        if matches!(
            app.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        ) && Instant::now() >= next_telemetry_sample
        {
            let telemetry = telemetry_sampler.sample(&session_build_dir);
            let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry));
            next_telemetry_sample = Instant::now() + Duration::from_secs(1);
        }
        let _ = update(&mut app, Action::Tick);
        terminal.draw(|f| render(f, &app))?;
        if event::poll(refresh)?
            && let Event::Key(k) = event::read()?
        {
            let Some(input) = input_from_key(k) else {
                continue;
            };
            if app.recipe_editor.is_some() {
                let effect = match input {
                    Input::Esc => update(&mut app, Action::CloseRecipeEditor),
                    Input::Up => update(&mut app, Action::SelectRecipeEditorFile { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectRecipeEditorFile { delta: 1 }),
                    Input::Enter
                        if app
                            .recipe_editor
                            .as_ref()
                            .is_some_and(|editor| editor.editing) =>
                    {
                        update(&mut app, Action::AppendRecipeEditor('\n'))
                    }
                    Input::Enter | Input::Char('e')
                        if app
                            .recipe_editor
                            .as_ref()
                            .is_some_and(|editor| !editor.editing) =>
                    {
                        update(&mut app, Action::ToggleRecipeEditorEditing)
                    }
                    Input::CtrlS => update(&mut app, Action::SaveRecipeEditor),
                    Input::CtrlB => update(&mut app, Action::BeginCurrentImageBuild),
                    Input::Backspace => update(&mut app, Action::BackspaceRecipeEditor),
                    Input::Char(character) => {
                        update(&mut app, Action::AppendRecipeEditor(character))
                    }
                    _ => None,
                };
                match effect {
                    Some(Effect::LoadRecipeEditorFile(path)) => {
                        load_recipe_editor_file(&mut app, path).await;
                    }
                    Some(Effect::SaveRecipeEditorFile { path, content }) => {
                        save_recipe_editor_file(&mut app, path, content).await;
                    }
                    _ => {}
                }
            } else if app.focus == yoctui_model::FocusTarget::Navigator {
                let _ = match input {
                    Input::Up | Input::Char('k') => {
                        update(&mut app, Action::SelectNavigator { delta: -1 })
                    }
                    Input::Down | Input::Char('j') => {
                        update(&mut app, Action::SelectNavigator { delta: 1 })
                    }
                    Input::Enter => update(&mut app, Action::ActivateNavigator),
                    Input::Tab => update(&mut app, Action::CycleFocus { backwards: false }),
                    Input::BackTab => update(&mut app, Action::CycleFocus { backwards: true }),
                    Input::Esc => update(
                        &mut app,
                        Action::Focus(yoctui_model::FocusTarget::Workspace),
                    ),
                    _ => None,
                };
            } else if app.layer_browser.is_some() {
                let effect = match input {
                    Input::Up => update(&mut app, Action::SelectLayerBrowserEntry { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectLayerBrowserEntry { delta: 1 }),
                    Input::Enter => update(&mut app, Action::LayerBrowserEnter),
                    Input::Esc => update(&mut app, Action::LayerBrowserUp),
                    Input::Char('e') => update(&mut app, Action::EditSelectedLayerBrowserFile),
                    _ => None,
                };
                match effect {
                    Some(Effect::LoadLayerBrowserDirectory {
                        layer,
                        root,
                        directory,
                    }) => load_layer_browser_directory(&mut app, layer, root, directory).await,
                    Some(Effect::LoadLayerBrowserPreview(path)) => {
                        load_layer_browser_preview(&mut app, path).await
                    }
                    Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                        if let Some(Effect::LoadRecipeEditorFile(path)) = update(
                            &mut app,
                            Action::OpenRecipeEditor {
                                recipe: format!("Layer: {layer}"),
                                root,
                                files: vec![file],
                            },
                        ) {
                            load_recipe_editor_file(&mut app, path).await;
                        }
                    }
                    _ => {}
                }
            } else if app.devtool_reset_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolReset),
                    Input::Esc => update(&mut app, Action::CancelDevtoolReset),
                    _ => None,
                };
                if let Some(Effect::DevtoolReset(recipe)) = effect {
                    devtool_reset(&guard, &mut app, &session_build_dir, recipe).await;
                }
            } else if app.devtool_update_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolUpdateRecipe),
                    Input::Esc => update(&mut app, Action::CancelDevtoolUpdateRecipe),
                    _ => None,
                };
                if let Some(Effect::DevtoolUpdateRecipe(recipe)) = effect
                    && devtool_update_recipe(&guard, &mut app, &session_build_dir, recipe).await
                {
                    refresh_workspace(
                        &mut backend,
                        &mut app,
                        "Devtool recipe metadata updated and workspace refreshed.",
                    )
                    .await;
                }
            } else if app.devtool_finish_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolFinish),
                    Input::Esc => update(&mut app, Action::CancelDevtoolFinishConfirmation),
                    _ => None,
                };
                if let Some(Effect::DevtoolFinish(request)) = effect
                    && devtool_finish(&guard, &mut app, &session_build_dir, request).await
                {
                    refresh_workspace(
                        &mut backend,
                        &mut app,
                        "Devtool changes finished and workspace refreshed.",
                    )
                    .await;
                }
            } else if app.devtool_finish_recipe.is_some() {
                let _ = match input {
                    Input::Char(character) => {
                        update(&mut app, Action::AppendDevtoolFinishDestination(character))
                    }
                    Input::Backspace => update(&mut app, Action::BackspaceDevtoolFinishDestination),
                    Input::Enter => update(&mut app, Action::PreviewDevtoolFinish),
                    Input::Esc => update(&mut app, Action::CancelDevtoolFinish),
                    _ => None,
                };
            } else if app.devtool_deploy_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolDeploy),
                    Input::Esc => update(&mut app, Action::CancelDevtoolDeployConfirmation),
                    _ => None,
                };
                if let Some(Effect::DevtoolDeploy(request)) = effect {
                    devtool_deploy(&guard, &mut app, &session_build_dir, request).await;
                }
            } else if app.devtool_deploy_recipe.is_some() {
                let _ = match input {
                    Input::Char(character) => {
                        update(&mut app, Action::AppendDevtoolDeployTarget(character))
                    }
                    Input::Backspace => update(&mut app, Action::BackspaceDevtoolDeployTarget),
                    Input::Enter => update(&mut app, Action::PreviewDevtoolDeploy),
                    Input::Esc => update(&mut app, Action::CancelDevtoolDeploy),
                    _ => None,
                };
            } else if app.bbmask_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmBbmaskWrite),
                    Input::Esc => update(&mut app, Action::CancelBbmaskWrite),
                    _ => None,
                };
                if let Some(Effect::WriteBbmask(value)) = effect {
                    match write_bbmask(&session_build_dir, value).await {
                        Ok(()) => {
                            refresh_workspace(
                                &mut backend,
                                &mut app,
                                "BBMASK saved and workspace metadata refreshed.",
                            )
                            .await
                        }
                        Err(error) => {
                            app.notification = Some(format!("Could not save BBMASK: {error}"))
                        }
                    }
                }
            } else if app.bbmask_editing {
                let _ = match input {
                    Input::Char(character) => update(&mut app, Action::AppendBbmask(character)),
                    Input::Backspace => update(&mut app, Action::BackspaceBbmask),
                    Input::Enter => update(&mut app, Action::PreviewBbmaskEdit),
                    Input::Esc => update(&mut app, Action::CancelBbmaskEdit),
                    _ => None,
                };
            } else if app.build_completion_open {
                let _ = update(&mut app, Action::DismissBuildCompletion);
            } else if app.image_picker.is_some() {
                let _ = match input {
                    Input::Up => update(&mut app, Action::SelectImage { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectImage { delta: 1 }),
                    Input::Enter => update(&mut app, Action::ConfirmImagePicker),
                    Input::Esc => update(&mut app, Action::CancelImagePicker),
                    _ => None,
                };
            } else if app.recipe_task_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmRecipeTask),
                    Input::Esc => update(&mut app, Action::CancelRecipeTask),
                    _ => None,
                };
                if let Some(Effect::Start(request)) = effect {
                    begin_build(&mut backend, &mut app, request).await;
                }
            } else if app.build_options_open {
                let effect = match input {
                    Input::Char('b') => update(&mut app, Action::BeginBuildTargetTask(None)),
                    Input::Char('c') => {
                        update(&mut app, Action::BeginBuildTargetTask(Some("clean".into())))
                    }
                    Input::Char('m') => update(
                        &mut app,
                        Action::BeginBuildTargetTask(Some("menuconfig".into())),
                    ),
                    Input::Char('e') => update(&mut app, Action::BeginBuildTargetEdit),
                    Input::Esc => update(&mut app, Action::CloseBuildOptions),
                    _ => None,
                };
                if let Some(Effect::Start(request)) = effect {
                    begin_build(&mut backend, &mut app, request).await;
                }
            } else if app.build_target_editing {
                let effect = match input {
                    Input::Char(character) => {
                        update(&mut app, Action::AppendBuildTarget(character))
                    }
                    Input::Backspace => update(&mut app, Action::BackspaceBuildTarget),
                    Input::Enter => update(&mut app, Action::ConfirmBuildTarget),
                    Input::Esc => update(&mut app, Action::CancelBuildTargetEdit),
                    _ => None,
                };
                if let Some(Effect::Start(request)) = effect {
                    begin_build(&mut backend, &mut app, request).await;
                }
            } else if input == Input::Char('!') {
                open_yocto_shell(&guard, &mut app).await;
            } else if input == Input::Char('i') {
                let images = app
                    .workspace
                    .recipes
                    .iter()
                    .map(|recipe| recipe.name.as_str())
                    .filter(|name| name.contains("image"))
                    .map(str::to_owned)
                    .collect();
                let _ = update(&mut app, Action::OpenImagePicker(images));
            } else if input == Input::Char('B') {
                let _ = update(&mut app, Action::OpenBuildOptions);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('b') {
                let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
            } else if app.screen == yoctui_model::Screen::Dashboard
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::ScrollBuildTasks { delta });
            } else if app.screen == yoctui_model::Screen::BuildHistory
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectBuildHistory { delta });
            } else if app.screen == yoctui_model::Screen::Dependencies
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectDependency { delta });
            } else if app.screen == yoctui_model::Screen::Dependencies && input == Input::Enter {
                let _ = update(&mut app, Action::OpenSelectedDependency);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('d') {
                let root = match update(&mut app, Action::BeginSelectedRecipeDevtoolModify) {
                    Some(Effect::DevtoolModify(recipe)) => {
                        devtool_modify(&guard, &mut app, &session_build_dir, recipe.clone())
                            .await
                            .map(|root| (recipe, root))
                    }
                    _ => None,
                };
                if let Some((recipe, root)) = root {
                    open_workspace_editor(&mut app, recipe, root).await;
                }
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('D') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('u') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('F') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('P') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('g') {
                if let Some(Effect::GetDependencies(recipe)) =
                    update(&mut app, Action::BeginSelectedRecipeDependencies)
                {
                    match backend.get_dependencies(recipe.clone()).await {
                        Ok(dependencies) => {
                            let _ = update(
                                &mut app,
                                Action::DependenciesLoaded(RecipeDependencies {
                                    recipe,
                                    build: dependencies.build,
                                    runtime: dependencies.runtime,
                                }),
                            );
                        }
                        Err(error) => {
                            let _ = update(
                                &mut app,
                                Action::Failure(AppError::new(
                                    "Dependencies",
                                    error.to_string(),
                                    "use a bridge connected to a BitBake server that supports get_dependencies",
                                )),
                            );
                        }
                    }
                }
            } else if input == Input::Char('b') {
                let _ = update(&mut app, Action::BeginCurrentImageBuild);
            } else if app.screen == yoctui_model::Screen::Errors
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectError { delta });
            } else if app.screen == yoctui_model::Screen::Errors && input == Input::Enter {
                let _ = update(&mut app, Action::JumpToSelectedError);
            } else if app.screen == yoctui_model::Screen::Errors && input == Input::Char('o') {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::OpenSelectedErrorSource)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if app.screen == yoctui_model::Screen::Recipes
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectRecipe { delta });
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('C') {
                let _ = update(&mut app, Action::BeginSelectedRecipeClean);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('M') {
                let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('S') {
                let _ = update(&mut app, Action::BeginSelectedRecipeCleanState);
            } else if app.screen == yoctui_model::Screen::Layers
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectLayer { delta });
            } else if app.screen == yoctui_model::Screen::Layers && input == Input::Enter {
                if let Some(Effect::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                }) = update(&mut app, Action::BeginSelectedLayerBrowser)
                {
                    load_layer_browser_directory(&mut app, layer, root, directory).await;
                }
            } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('o') {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::OpenSelectedLayer)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('e') {
                if let Some(Effect::OpenWorkspaceEditor { label, root }) =
                    update(&mut app, Action::BeginSelectedLayerWorkspaceEditor)
                {
                    open_workspace_editor(&mut app, label, root).await;
                }
            } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('R') {
                if matches!(
                    update(&mut app, Action::BeginLayerRelationships),
                    Some(Effect::GetLayerRelationships)
                ) {
                    match backend.get_layer_relationships().await {
                        Ok(layers) => {
                            let _ = update(
                                &mut app,
                                Action::LayerRelationshipsLoaded(LayerRelationships {
                                    layers: layers
                                        .into_iter()
                                        .map(|layer| LayerRelationship {
                                            name: layer.name,
                                            priority: layer.priority,
                                            compatible: layer.compatible,
                                            depends: layer.depends,
                                            overlays: layer.overlays,
                                            appends: layer.appends,
                                        })
                                        .collect(),
                                }),
                            );
                        }
                        Err(error) => {
                            let _ = update(
                                &mut app,
                                Action::Failure(AppError::new(
                                    "Layers",
                                    error.to_string(),
                                    "use a bridge connected to a BitBake server that supports get_layer_relationships",
                                )),
                            );
                        }
                    }
                }
            } else if app.screen == yoctui_model::Screen::Configuration
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectConfigVariable { delta });
            } else if app.screen == yoctui_model::Screen::Configuration && input == Input::Char('o')
            {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::OpenSelectedConfigSource)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if app.screen == yoctui_model::Screen::Bbmask && input == Input::Char('e') {
                let _ = update(&mut app, Action::BeginBbmaskEdit);
            } else if matches!(
                app.screen,
                yoctui_model::Screen::Recipes
                    | yoctui_model::Screen::Layers
                    | yoctui_model::Screen::Configuration
            ) && input == Input::Char('/')
            {
                let _ = update(&mut app, Action::BeginMetadataSearch);
            } else if app.metadata_searching {
                match input {
                    Input::Char(character) => {
                        let _ = update(&mut app, Action::AppendMetadataQuery(character));
                    }
                    Input::Enter | Input::Esc => {
                        let _ = update(&mut app, Action::FinishMetadataSearch);
                    }
                    Input::Backspace => {
                        let _ = update(&mut app, Action::BackspaceMetadataQuery);
                    }
                    _ => {}
                }
            } else if app.logs.searching {
                match input {
                    Input::Char(character) => {
                        let _ = update(&mut app, Action::AppendLogQuery(character));
                    }
                    Input::Enter | Input::Esc => {
                        let _ = update(&mut app, Action::FinishLogSearch);
                    }
                    Input::Backspace => {
                        let _ = update(&mut app, Action::BackspaceLogQuery);
                    }
                    _ => {}
                }
            } else if let Some(action) = key_action(input) {
                if matches!(action, Action::Cancel) {
                    if let Some(Effect::Cancel) = update(&mut app, action)
                        && let Err(error) = backend.cancel_build().await
                    {
                        let _ = update(
                            &mut app,
                            Action::Failure(AppError::new(
                                "Cancellation",
                                error.to_string(),
                                "check whether BitBake is still running",
                            )),
                        );
                    }
                } else {
                    let _ = update(&mut app, action);
                }
            }
        }
        if let Ok(Ok(event)) =
            tokio::time::timeout(Duration::from_millis(1), backend.next_event()).await
        {
            match event {
                BackendEvent::BuildCompleted { success, exit_code } => {
                    let _ = update(&mut app, Action::BuildCompleted { success, exit_code });
                }
                event => {
                    if let Some(action) = action_from_event(event) {
                        let _ = update(&mut app, action);
                    }
                }
            }
        }
        if app.should_quit {
            break;
        }
    }
    backend.shutdown().await?;
    write_session(
        session_path.as_deref(),
        &Session {
            last_target: app.build.target,
            last_screen: Some(app.screen),
            log_filter: app.logs.filter,
            log_recipe_filter: app.logs.recipe_filter,
            log_task_filter: app.logs.task_filter,
            log_wrap: app.logs.wrap,
            last_backend: Some(backend_kind),
            recent_build_dirs: std::iter::once(session_build_dir)
                .chain(session.recent_build_dirs)
                .fold(Vec::new(), |mut directories, directory| {
                    if !directories.contains(&directory) && directories.len() < 10 {
                        directories.push(directory);
                    }
                    directories
                }),
        },
    )?;
    Ok(())
}

#[cfg(unix)]
fn termination_receiver() -> Result<tokio::sync::mpsc::Receiver<()>> {
    let (sender, receiver) = tokio::sync::mpsc::channel(1);
    let mut sigterm = signal(SignalKind::terminate())?;
    tokio::spawn(async move {
        sigterm.recv().await;
        let _ = sender.send(()).await;
    });
    Ok(receiver)
}

#[cfg(unix)]
fn termination_requested(receiver: &mut tokio::sync::mpsc::Receiver<()>) -> bool {
    receiver.try_recv().is_ok()
}

fn input_from_key(key: KeyEvent) -> Option<Input> {
    match key.code {
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlC),
        KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlS),
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlB),
        KeyCode::Tab => Some(Input::Tab),
        KeyCode::BackTab => Some(Input::BackTab),
        KeyCode::Char(character) => Some(Input::Char(character)),
        KeyCode::Esc => Some(Input::Esc),
        KeyCode::Enter => Some(Input::Enter),
        KeyCode::Up => Some(Input::Up),
        KeyCode::Down => Some(Input::Down),
        KeyCode::Backspace => Some(Input::Backspace),
        KeyCode::Left => Some(Input::Left),
        KeyCode::Right => Some(Input::Right),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_retention_and_backend_settings() {
        let config: FileConfig = toml::from_str(
            "backend = 'process'\nlog_retention_entries = 42\nlog_retention_bytes = 1024\nrefresh_ms = 50\ncancellation_timeout_ms = 250\ndefault_target = 'core-image-minimal'\neditor = 'nano'",
        )
        .unwrap();
        assert!(matches!(config.backend, Some(Backend::Process)));
        assert_eq!(config.log_retention_entries, Some(42));
        assert_eq!(config.default_target.as_deref(), Some("core-image-minimal"));
        assert_eq!(config.editor.as_deref(), Some("nano"));
        assert_eq!(config.cancellation_timeout_ms, Some(250));
    }

    #[test]
    fn session_round_trip_preserves_preferences() {
        let directory = std::env::temp_dir().join(format!("yoctui-session-{}", std::process::id()));
        let path = directory.join("session.toml");
        write_session(
            Some(&path),
            &Session {
                last_target: Some("core-image-minimal".into()),
                last_screen: Some(Screen::Logs),
                log_filter: Some(Severity::Warning),
                log_recipe_filter: Some("busybox".into()),
                log_task_filter: Some("do_compile".into()),
                log_wrap: true,
                last_backend: Some(Backend::Process),
                recent_build_dirs: vec![PathBuf::from("/build")],
            },
        )
        .unwrap();
        assert_eq!(
            read_session(Some(&path)).unwrap(),
            Session {
                last_target: Some("core-image-minimal".into()),
                last_screen: Some(Screen::Logs),
                log_filter: Some(Severity::Warning),
                log_recipe_filter: Some("busybox".into()),
                log_task_filter: Some("do_compile".into()),
                log_wrap: true,
                last_backend: Some(Backend::Process),
                recent_build_dirs: vec![PathBuf::from("/build")],
            }
        );
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn normalizes_task_progress_event() {
        assert!(matches!(
            action_from_event(BackendEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: 25,
            }),
            Some(Action::TaskProgress { progress: 25, .. })
        ));
    }

    #[test]
    fn parses_cpu_counters_from_proc_stat() {
        assert_eq!(
            parse_cpu_counters("cpu  100 20 30 400 50 0 0 0 0 0"),
            Some(CpuCounters {
                total: 600,
                idle: 450,
            })
        );
        assert_eq!(parse_cpu_counters("intr 1 2 3"), None);
    }

    #[test]
    fn bbmask_assignment_is_single_line_and_shell_quoted() {
        assert_eq!(
            bbmask_assignment("meta-broken/.* \"quoted\"").unwrap(),
            "BBMASK = \"meta-broken/.* \\\"quoted\\\"\""
        );
        assert!(bbmask_assignment("bad\nvalue").is_err());
    }

    #[tokio::test]
    async fn writes_an_explicit_bbmask_assignment_to_local_conf() {
        let build_dir = std::env::temp_dir().join(format!("yoctui-bbmask-{}", std::process::id()));
        let conf_dir = build_dir.join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let local_conf = conf_dir.join("local.conf");
        fs::write(&local_conf, "MACHINE = \"qemuarm\"\n").unwrap();

        write_bbmask(&build_dir, "meta-broken/.*".into())
            .await
            .unwrap();

        assert_eq!(
            fs::read_to_string(&local_conf).unwrap(),
            "MACHINE = \"qemuarm\"\nBBMASK = \"meta-broken/.*\"\n"
        );
        fs::remove_file(local_conf).unwrap();
        fs::remove_dir(conf_dir).unwrap();
        fs::remove_dir(build_dir).unwrap();
    }

    #[test]
    fn ctrl_c_is_not_the_regular_cancel_key() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(input_from_key(key), Some(Input::CtrlC));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn queued_termination_requests_exit_the_tui_loop() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        sender.send(()).await.unwrap();
        assert!(termination_requested(&mut receiver));
    }

    #[test]
    fn devtool_source_path_uses_the_standard_workspace_layout() {
        assert_eq!(
            devtool_source_dir(Path::new("/build"), "busybox"),
            PathBuf::from("/build/workspace/sources/busybox")
        );
    }
}
