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
    io::{Read as _, Write as _},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    time::{Duration, Instant, SystemTime},
};
#[cfg(unix)]
use std::{ffi::CString, os::unix::ffi::OsStrExt};
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use yoctui_app::{
    BuildJobCoordinator, DevtoolJobCoordinator, Input, config_compare_dialog_action,
    config_edit_confirmation_action, config_edit_dialog_action, config_scope_picker_action,
    config_source_picker_action, config_workspace_action, devtool_modify_confirmation_action,
    errors_action, focus_action, key_action, logs_action, recipe_editor_action, settings_action,
    tasks_action,
};
use yoctui_bitbake::{
    BackendEvent, BitBakeBackend, BridgeBackend, DevtoolCommandSpec, DevtoolInspector,
    DevtoolJobRunner, DevtoolRunnerEvent, ProcessBackend, VariableValue,
};
use yoctui_model::{
    Action, AnimationSpeed, App, AppError, BuildRequest, BuildStatus, ConfigEditRequest,
    DevtoolOperation, DevtoolWorkspace, Dialog, Effect, GitFileState, HostTelemetry,
    LayerBrowserEntry, LayerInspectorMode, LayerRelationship, LayerRelationships, PreviewKind,
    RecipeDependencies, RecipeIdentity, Screen, Severity, Theme, VariableDetail, VariableIdentity,
    update, validate_config_edit_request,
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
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
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
    log_build_filter: Option<String>,
    #[serde(default)]
    log_wrap: Option<bool>,
    #[serde(default)]
    log_follow: Option<bool>,
    #[serde(default)]
    theme: Option<Theme>,
    #[serde(default)]
    animation_speed: Option<AnimationSpeed>,
    #[serde(default)]
    reduced_motion: Option<bool>,
    #[serde(default)]
    color_enabled: Option<bool>,
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
    let temporary = path.with_extension("toml.tmp");
    fs::write(&temporary, toml::to_string(session)?)
        .with_context(|| format!("could not write session file {}", temporary.display()))?;
    fs::rename(&temporary, path)
        .with_context(|| format!("could not replace session file {}", path.display()))
}

fn persist_settings(path: Option<&Path>, session: &mut Session, app: &App) -> Result<()> {
    let mut updated = session.clone();
    updated.theme = Some(app.theme);
    updated.animation_speed = Some(app.animation_speed);
    updated.reduced_motion = Some(app.reduced_motion);
    updated.color_enabled = Some(app.color_enabled);
    updated.log_wrap = Some(app.logs.wrap);
    updated.log_follow = Some(app.logs.follow);
    write_session(path, &updated)?;
    *session = updated;
    Ok(())
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
        color: !cli.no_color && session.color_enabled.or(file.color).unwrap_or(true),
        theme: session.theme.or(file.theme).unwrap_or_default(),
        animation_speed: session
            .animation_speed
            .or(file.animation_speed)
            .unwrap_or_default(),
        reduced_motion: session
            .reduced_motion
            .or(file.reduced_motion)
            .unwrap_or(false),
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
        let mut build_jobs = BuildJobCoordinator::default();
        let workspace = backend.inspect_workspace().await?;
        let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
        if targets.is_empty() {
            println!("headless inspection completed");
            return Ok(());
        }
        let request = BuildRequest {
            targets,
            task: None,
            force: false,
        };
        if let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) {
            for action in actions {
                let _ = update(&mut app, action);
            }
        }
        if let Err(error) = backend.start_build(request).await {
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(&mut app, action);
            }
            return Err(error).context("could not start bitbake");
        }
        loop {
            let event = backend.next_event().await?;
            if let BackendEvent::Log(entry) = &event {
                println!("{}", entry.message);
            }
            let completion = match &event {
                BackendEvent::BuildCompleted { success, exit_code } => Some((*success, *exit_code)),
                _ => None,
            };
            for action in build_jobs.actions_for_backend_event(event, SystemTime::now()) {
                let _ = update(&mut app, action);
            }
            if let Some((success, exit_code)) = completion {
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
        }
    }
    .await;
    let shutdown = backend.shutdown().await;
    result?;
    shutdown?;
    Ok(())
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

async fn begin_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    request: BuildRequest,
) {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Notify("A build background job is already active.".into()),
        );
        return;
    };
    for action in actions {
        let _ = update(app, action);
    }
    match backend.start_build(request).await {
        Ok(()) => {}
        Err(error) => {
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
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

async fn begin_devtool_job(
    app: &mut App,
    coordinator: &mut DevtoolJobCoordinator,
    runner: &mut Option<DevtoolJobRunner>,
    build_dir: &Path,
    cancellation_timeout: Duration,
    operation: DevtoolOperation,
) -> bool {
    let Some(actions) = coordinator.queue(operation.clone(), SystemTime::now()) else {
        let _ = update(
            app,
            Action::Notify("A Devtool background job is already active.".into()),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let command = match DevtoolCommandSpec::from_operation(&operation) {
        Ok(command) => command,
        Err(error) => {
            for action in coordinator.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            return false;
        }
    };
    let mut started = DevtoolJobRunner::new(build_dir.to_path_buf())
        .with_cancellation_timeout(cancellation_timeout);
    match started.start(command).await {
        Ok(()) => {
            *runner = Some(started);
            true
        }
        Err(error) => {
            for action in coordinator.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

async fn poll_devtool_job(
    app: &mut App,
    coordinator: &mut DevtoolJobCoordinator,
    runner: &mut Option<DevtoolJobRunner>,
) -> Option<DevtoolOperation> {
    let active = runner.as_mut()?;
    match tokio::time::timeout(Duration::from_millis(1), active.next_event()).await {
        Ok(Ok(event)) => {
            let completed_operation = matches!(event, DevtoolRunnerEvent::Completed { .. })
                .then(|| coordinator.active_operation().cloned())
                .flatten();
            let terminal = matches!(
                event,
                DevtoolRunnerEvent::Completed { .. }
                    | DevtoolRunnerEvent::Failed { .. }
                    | DevtoolRunnerEvent::Cancelled { .. }
                    | DevtoolRunnerEvent::Lost { .. }
            );
            for action in coordinator.actions_for_event(event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *runner = None;
            }
            completed_operation
        }
        Ok(Err(error)) => {
            for action in coordinator.actions_for_event(
                DevtoolRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *runner = None;
            None
        }
        Err(_) => None,
    }
}

fn editor_path_error(path: &Path) -> Option<String> {
    match path.try_exists() {
        Ok(true) => None,
        Ok(false) => Some(format!(
            "Cannot open {} because the reported path no longer exists.",
            path.display()
        )),
        Err(error) => Some(format!(
            "Cannot inspect the reported path {}: {error}",
            path.display()
        )),
    }
}

fn run_editor_process(
    editor: &std::ffi::OsStr,
    path: &Path,
) -> std::io::Result<std::process::ExitStatus> {
    ProcessCommand::new(editor).arg(path).status()
}

fn editor_exit_error(status: std::process::ExitStatus, path: &str) -> Option<String> {
    (!status.success()).then(|| format!("$EDITOR exited with {status} while opening {path}."))
}

async fn open_in_editor(
    guard: &TerminalGuard,
    app: &mut App,
    path: PathBuf,
    preferred_editor: Option<&str>,
) {
    if let Some(error) = editor_path_error(&path) {
        app.notification = Some(error);
        return;
    }
    let editor = preferred_editor
        .map(Into::into)
        .or_else(|| env::var_os("EDITOR"))
        .unwrap_or_else(|| "vi".into());
    let path_label = path.display().to_string();
    if let Err(error) = guard.suspend() {
        app.notification = Some(format!(
            "Could not suspend the terminal for $EDITOR: {error}"
        ));
        return;
    }
    let editor_result =
        tokio::task::spawn_blocking(move || run_editor_process(&editor, &path)).await;
    let resume_result = guard.resume();
    if let Err(error) = resume_result {
        app.notification = Some(format!(
            "Could not restore the terminal after $EDITOR: {error}"
        ));
    } else if let Ok(Err(error)) = editor_result {
        app.notification = Some(format!("Could not start $EDITOR: {error}"));
    } else if let Ok(Ok(status)) = editor_result
        && let Some(error) = editor_exit_error(status, &path_label)
    {
        app.notification = Some(error);
    } else if let Err(error) = editor_result {
        app.notification = Some(format!("$EDITOR task failed: {error}"));
    }
}

async fn copy_to_clipboard(app: &mut App, content: String) {
    let result = tokio::task::spawn_blocking(move || -> Result<&'static str> {
        let candidates: [(&str, &[&str]); 3] = [
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ];
        for (program, args) in candidates {
            let Ok(mut child) = ProcessCommand::new(program)
                .args(args)
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            else {
                continue;
            };
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(content.as_bytes())?;
            }
            if child.wait()?.success() {
                return Ok(program);
            }
        }
        anyhow::bail!("install wl-copy, xclip, or xsel and provide a graphical clipboard")
    })
    .await;
    app.notification = Some(match result {
        Ok(Ok(program)) => format!("Selected log details copied with {program}."),
        Ok(Err(error)) => format!("Could not copy selected log details: {error}"),
        Err(error) => format!("Clipboard task failed: {error}"),
    });
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

async fn inspect_selected_devtool(app: &mut App, build_dir: &Path) {
    if let Some(Effect::InspectDevtoolStatus(identity)) =
        update(app, Action::BeginSelectedRecipeDevtoolStatus)
    {
        let status = DevtoolInspector::default()
            .inspect(build_dir, identity)
            .await;
        let _ = update(app, Action::DevtoolStatusLoaded(status));
    }
}

async fn complete_devtool_modify(app: &mut App, build_dir: &Path, identity: RecipeIdentity) {
    let status = DevtoolInspector::default()
        .inspect(build_dir, identity)
        .await;
    apply_completed_devtool_modify_status(app, status).await;
}

async fn apply_completed_devtool_modify_status(app: &mut App, status: yoctui_model::DevtoolStatus) {
    let editor = if let Some(error) = &status.error {
        app.notification = Some(format!(
            "Devtool modify completed, but authoritative status refresh failed: {error:?}"
        ));
        None
    } else {
        match &status.workspace {
            DevtoolWorkspace::Present { source_path, .. } => {
                Some((status.identity.name.clone(), source_path.clone()))
            }
            DevtoolWorkspace::MissingDirectory { source_path } => {
                app.notification = Some(format!(
                    "Devtool modify completed, but the reported workspace source is missing: {}",
                    source_path.display()
                ));
                None
            }
            DevtoolWorkspace::NotMember => {
                app.notification = Some(
                    "Devtool modify completed, but the recipe is not reported in the workspace."
                        .into(),
                );
                None
            }
        }
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    if let Some((recipe, root)) = editor {
        open_workspace_editor(app, recipe, root).await;
    }
}

fn config_variable_loaded_action(requested: VariableIdentity, variable: VariableValue) -> Action {
    Action::VariableLoaded(VariableDetail {
        identity: VariableIdentity {
            name: requested.name,
            recipe: variable.recipe,
        },
        effective_value: variable.value,
        unexpanded_value: variable.unexpanded_value,
        provenance: variable.provenance,
        operations: variable.operations,
        active_overrides: variable.active_overrides,
    })
}

async fn load_config_variable(
    app: &mut App,
    backend: &mut dyn BitBakeBackend,
    identity: VariableIdentity,
) {
    match backend
        .get_variable(identity.name.clone(), identity.recipe.clone())
        .await
    {
        Ok(variable) => {
            let _ = update(app, config_variable_loaded_action(identity, variable));
        }
        Err(error) => {
            let _ = update(
                app,
                Action::VariableDetailFailed {
                    identity,
                    message: error.to_string(),
                },
            );
        }
    }
}

async fn inspect_selected_config_variable(app: &mut App, backend: &mut dyn BitBakeBackend) {
    if let Some(Effect::GetVariable(identity)) = update(app, Action::BeginSelectedConfigDetail) {
        load_config_variable(app, backend, identity).await;
    }
}

fn config_copy_effect(app: &mut App, input: Input) -> Option<Effect> {
    config_workspace_action(false, input).and_then(|action| update(app, action))
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

fn scan_layer_directory(scan: &Path) -> io::Result<Vec<LayerBrowserEntry>> {
    let git_output = ProcessCommand::new("git")
        .args([
            "status",
            "--porcelain=v1",
            "--ignored",
            "--untracked-files=all",
            "--",
            ".",
        ])
        .current_dir(scan)
        .output()
        .ok()
        .filter(|output| output.status.success());
    let git_lines = git_output.as_ref().map(|output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let status = line.get(..2)?;
                let path = line.get(3..)?;
                Some((status.to_owned(), scan.join(path)))
            })
            .collect::<Vec<_>>()
    });
    let mut entries = fs::read_dir(scan)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name();
            if name == ".git" {
                return None;
            }
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().is_some_and(|value| value.is_dir());
            let git = git_lines
                .as_ref()
                .map_or(GitFileState::Unavailable, |lines| {
                    lines
                        .iter()
                        .find(|(_, changed)| {
                            changed == &path || (is_dir && changed.starts_with(&path))
                        })
                        .map_or(GitFileState::Clean, |(status, _)| match status.as_str() {
                            "??" => GitFileState::Untracked,
                            "!!" => GitFileState::Ignored,
                            _ => GitFileState::Modified,
                        })
                });
            Some(LayerBrowserEntry {
                path,
                is_dir,
                depth: 0,
                is_hidden: name.to_string_lossy().starts_with('.'),
                size: metadata.as_ref().map(|value| value.len()),
                modified: metadata.and_then(|value| value.modified().ok()),
                git,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        (
            !entry.is_dir,
            entry.path.file_name().map(|name| name.to_owned()),
        )
    });
    Ok(entries)
}

async fn load_layer_browser_directory(
    app: &mut App,
    layer: String,
    root: PathBuf,
    directory: PathBuf,
) {
    let scan = directory.clone();
    match tokio::task::spawn_blocking(move || scan_layer_directory(&scan)).await {
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

fn read_layer_preview(path: &Path) -> io::Result<(String, PreviewKind, bool)> {
    const MAX_PREVIEW_BYTES: usize = 64 * 1024;
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(MAX_PREVIEW_BYTES as u64)
        .read_to_end(&mut bytes)?;
    let truncated = size > bytes.len() as u64;
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        Ok((String::new(), PreviewKind::Binary, truncated))
    } else {
        Ok((
            String::from_utf8(bytes).expect("UTF-8 was validated"),
            PreviewKind::Text,
            truncated,
        ))
    }
}

async fn load_layer_browser_preview(app: &mut App, path: PathBuf) {
    let preview_path = path.clone();
    match tokio::task::spawn_blocking(move || read_layer_preview(&preview_path)).await {
        Ok(Ok((content, kind, truncated))) => {
            let _ = update(
                app,
                Action::LoadLayerBrowserPreview {
                    path,
                    content,
                    kind,
                    truncated,
                },
            );
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

fn is_exact_config_assignment(line: &str, name: &str) -> bool {
    let line = line.trim_end_matches('\r').trim_start();
    if line.starts_with('#') {
        return false;
    }
    let Some(remainder) = line.strip_prefix(name) else {
        return false;
    };
    if !remainder.chars().next().is_some_and(char::is_whitespace) {
        return false;
    }
    let expression = remainder.trim_start();
    ["??=", "?=", ":=", "+=", "=+", ".=", "=.", "="]
        .iter()
        .any(|operator| expression.starts_with(operator))
}

fn replace_config_assignment(content: &str, name: &str, assignment: &str) -> String {
    let mut output = String::with_capacity(content.len().max(assignment.len()) + 1);
    let mut replaced = false;
    for segment in content.split_inclusive('\n') {
        let ending = if segment.ends_with("\r\n") {
            "\r\n"
        } else if segment.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let body = segment
            .strip_suffix(ending)
            .expect("the detected line ending is a suffix");
        if is_exact_config_assignment(body, name) {
            if !replaced {
                output.push_str(assignment);
                output.push_str(ending);
                replaced = true;
            }
        } else {
            output.push_str(segment);
        }
    }
    if replaced {
        return output;
    }

    let ending = if content.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    if !output.is_empty() && !output.ends_with('\n') {
        output.push_str(ending);
    }
    output.push_str(assignment);
    output.push_str(ending);
    output
}

fn write_config_assignment_atomic(build_dir: &Path, request: &ConfigEditRequest) -> Result<()> {
    validate_config_edit_request(request, build_dir).map_err(anyhow::Error::msg)?;
    let destination = &request.destination;
    let metadata = fs::metadata(destination)
        .with_context(|| format!("could not inspect {}", destination.display()))?;
    if !metadata.is_file() {
        anyhow::bail!("{} is not a regular file", destination.display());
    }
    let content = fs::read_to_string(destination)
        .with_context(|| format!("could not read {}", destination.display()))?;
    let updated = replace_config_assignment(&content, &request.identity.name, &request.assignment);
    let parent = destination
        .parent()
        .context("configuration destination has no parent directory")?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .context("configuration destination file name is not valid UTF-8")?;
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let mut temporary = None;
    for attempt in 0..16 {
        let candidate = parent.join(format!(
            ".{file_name}.yoctui-{}-{nonce}-{attempt}.tmp",
            std::process::id()
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&candidate)
        {
            Ok(file) => {
                temporary = Some((candidate, file));
                break;
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(error).with_context(|| {
                    format!(
                        "could not create temporary configuration file in {}",
                        parent.display()
                    )
                });
            }
        }
    }
    let (temporary_path, mut temporary_file) =
        temporary.context("could not allocate a unique temporary configuration file")?;
    let write_result = (|| -> Result<()> {
        temporary_file
            .set_permissions(metadata.permissions())
            .with_context(|| {
                format!(
                    "could not preserve permissions on {}",
                    temporary_path.display()
                )
            })?;
        temporary_file
            .write_all(updated.as_bytes())
            .with_context(|| format!("could not write {}", temporary_path.display()))?;
        temporary_file
            .sync_all()
            .with_context(|| format!("could not sync {}", temporary_path.display()))?;
        drop(temporary_file);
        fs::rename(&temporary_path, destination)
            .with_context(|| format!("could not atomically replace {}", destination.display()))?;
        if let Ok(directory) = fs::File::open(parent) {
            let _ = directory.sync_all();
        }
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    write_result
}

async fn write_config_assignment(build_dir: &Path, request: ConfigEditRequest) -> Result<()> {
    let build_dir = build_dir.to_path_buf();
    tokio::task::spawn_blocking(move || write_config_assignment_atomic(&build_dir, &request))
        .await
        .context("configuration write task failed")?
}

fn finish_config_edit_refresh(
    app: &mut App,
    identity: VariableIdentity,
    result: std::result::Result<VariableValue, yoctui_bitbake::BackendError>,
) {
    match result {
        Ok(variable) => {
            let _ = update(
                app,
                config_variable_loaded_action(identity.clone(), variable),
            );
            let _ = update(app, Action::ConfigEditRefreshSucceeded { identity });
        }
        Err(error) => {
            let _ = update(
                app,
                Action::ConfigEditRefreshFailed {
                    identity,
                    message: error.to_string(),
                },
            );
        }
    }
}

async fn execute_config_edit_write(
    backend: &mut dyn BitBakeBackend,
    app: &mut App,
    build_dir: &Path,
    request: ConfigEditRequest,
) {
    let identity = request.identity.clone();
    match write_config_assignment(build_dir, request).await {
        Ok(()) => {
            if let Some(Effect::GetVariable(identity)) = update(
                app,
                Action::ConfigEditWriteSucceeded {
                    identity: identity.clone(),
                },
            ) {
                let result = backend
                    .get_variable(identity.name.clone(), identity.recipe.clone())
                    .await;
                finish_config_edit_refresh(app, identity, result);
            }
        }
        Err(error) => {
            let _ = update(
                app,
                Action::ConfigEditWriteFailed {
                    identity,
                    message: error.to_string(),
                },
            );
        }
    }
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
                Ok(recipes) => {
                    let _ = update(app, Action::RecipesLoaded(recipes));
                }
                Err(error) => app.notification = Some(format!("Recipes unavailable: {error}")),
            }
            match backend.list_layers().await {
                Ok(layers) => {
                    let _ = update(app, Action::LayersLoaded(layers));
                }
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

async fn tui(config: Config, targets: Vec<String>, mut session: Session) -> Result<()> {
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
    app.logs.recipe_filter = session.log_recipe_filter.clone();
    app.logs.task_filter = session.log_task_filter.clone();
    app.logs.build_filter = session.log_build_filter.clone();
    app.logs.wrap = session.log_wrap.unwrap_or(false);
    app.logs.follow = session.log_follow.unwrap_or(true);
    let session_build_dir = build_dir.clone();
    let mut backend =
        select_backend_with_timeout(backend_kind.clone(), build_dir, Some(cancellation_timeout))
            .await?;
    match backend.inspect_workspace().await {
        Ok(workspace) => {
            let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
            match backend.list_recipes(None).await {
                Ok(recipes) => {
                    let _ = update(&mut app, Action::RecipesLoaded(recipes));
                }
                Err(error) => app.notification = Some(format!("Recipes unavailable: {error}")),
            }
            match backend.list_layers().await {
                Ok(layers) => {
                    let _ = update(&mut app, Action::LayersLoaded(layers));
                }
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
    let mut build_jobs = BuildJobCoordinator::default();
    let mut devtool_jobs = DevtoolJobCoordinator::default();
    let mut devtool_runner = None;
    let mut pending_devtool_modify = None;
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
            if app.command_palette_open {
                let _ = match input {
                    Input::Up => update(&mut app, Action::SelectCommandPalette { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectCommandPalette { delta: 1 }),
                    Input::Enter => update(&mut app, Action::ActivateCommandPalette),
                    Input::Esc => update(&mut app, Action::CloseCommandPalette),
                    Input::Backspace => update(&mut app, Action::BackspaceCommandPaletteQuery),
                    Input::Char(character) => {
                        update(&mut app, Action::AppendCommandPaletteQuery(character))
                    }
                    _ => None,
                };
            } else if matches!(app.active_dialog(), Some(Dialog::RecipeEditor(_))) {
                let editing = app.active_dialog().is_some_and(
                    |dialog| matches!(dialog, Dialog::RecipeEditor(editor) if editor.editing),
                );
                let effect = recipe_editor_action(editing, input)
                    .and_then(|action| update(&mut app, action));
                match effect {
                    Some(Effect::LoadRecipeEditorFile(path)) => {
                        load_recipe_editor_file(&mut app, path).await;
                    }
                    Some(Effect::SaveRecipeEditorFile { path, content }) => {
                        save_recipe_editor_file(&mut app, path, content).await;
                    }
                    _ => {}
                }
            } else if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolModifyConfirmation(_))
            ) {
                let effect = devtool_modify_confirmation_action(input)
                    .and_then(|action| update(&mut app, action));
                if let Some(Effect::DevtoolModify(identity)) = effect {
                    let recipe = identity.name.clone();
                    if begin_devtool_job(
                        &mut app,
                        &mut devtool_jobs,
                        &mut devtool_runner,
                        &session_build_dir,
                        cancellation_timeout,
                        DevtoolOperation::Modify { recipe },
                    )
                    .await
                    {
                        pending_devtool_modify = Some(identity);
                    }
                }
            } else if matches!(
                app.focus,
                yoctui_model::FocusTarget::Navigator | yoctui_model::FocusTarget::Inspector
            ) {
                if let Some(action) = focus_action(app.focus, input) {
                    let _ = update(&mut app, action);
                }
            } else if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
                let _ = match input {
                    Input::Char('Y') => update(&mut app, Action::ConfirmQuit),
                    Input::Esc => update(&mut app, Action::CancelQuit),
                    _ => None,
                };
            } else if app.layer_browser.is_some()
                && !app.metadata_searching
                && app.focus != yoctui_model::FocusTarget::Dialog
            {
                let effect = match input {
                    Input::Tab => update(&mut app, Action::CycleFocus { backwards: false }),
                    Input::BackTab => update(&mut app, Action::CycleFocus { backwards: true }),
                    Input::Up => update(&mut app, Action::SelectLayerBrowserEntry { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectLayerBrowserEntry { delta: 1 }),
                    Input::Enter => update(&mut app, Action::LayerBrowserEnter),
                    Input::Right | Input::Char('l') => update(&mut app, Action::LayerBrowserExpand),
                    Input::Esc => update(&mut app, Action::CloseLayerBrowser),
                    Input::Left | Input::Char('h') => update(&mut app, Action::LayerBrowserUp),
                    Input::Char('r') => update(&mut app, Action::RefreshLayerBrowser),
                    Input::Char('e') => update(&mut app, Action::EditSelectedLayerBrowserFile),
                    Input::Char('.') => update(&mut app, Action::ToggleLayerBrowserHidden),
                    Input::Char('/') => update(&mut app, Action::BeginMetadataSearch),
                    Input::Char('g') => update(
                        &mut app,
                        Action::SetLayerInspectorMode(LayerInspectorMode::Git),
                    ),
                    Input::Char('m') => update(
                        &mut app,
                        Action::SetLayerInspectorMode(LayerInspectorMode::Metadata),
                    ),
                    Input::Char('d') => update(
                        &mut app,
                        Action::SetLayerInspectorMode(LayerInspectorMode::Dependencies),
                    ),
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
            } else if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolResetConfirmation(_))
            ) {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolReset),
                    Input::Esc => update(&mut app, Action::CancelDevtoolReset),
                    _ => None,
                };
                if let Some(Effect::DevtoolReset(recipe)) = effect {
                    begin_devtool_job(
                        &mut app,
                        &mut devtool_jobs,
                        &mut devtool_runner,
                        &session_build_dir,
                        cancellation_timeout,
                        DevtoolOperation::Reset { recipe },
                    )
                    .await;
                }
            } else if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolUpdateConfirmation(_))
            ) {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolUpdateRecipe),
                    Input::Esc => update(&mut app, Action::CancelDevtoolUpdateRecipe),
                    _ => None,
                };
                if let Some(Effect::DevtoolUpdateRecipe(recipe)) = effect {
                    begin_devtool_job(
                        &mut app,
                        &mut devtool_jobs,
                        &mut devtool_runner,
                        &session_build_dir,
                        cancellation_timeout,
                        DevtoolOperation::UpdateRecipe { recipe },
                    )
                    .await;
                }
            } else if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolFinishConfirmation(_))
            ) {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolFinish),
                    Input::Esc => update(&mut app, Action::CancelDevtoolFinishConfirmation),
                    _ => None,
                };
                if let Some(Effect::DevtoolFinish(request)) = effect {
                    begin_devtool_job(
                        &mut app,
                        &mut devtool_jobs,
                        &mut devtool_runner,
                        &session_build_dir,
                        cancellation_timeout,
                        request.into(),
                    )
                    .await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::DevtoolFinish { .. })) {
                let _ = match input {
                    Input::Char(character) => {
                        update(&mut app, Action::AppendDevtoolFinishDestination(character))
                    }
                    Input::Backspace => update(&mut app, Action::BackspaceDevtoolFinishDestination),
                    Input::Enter => update(&mut app, Action::PreviewDevtoolFinish),
                    Input::Esc => update(&mut app, Action::CancelDevtoolFinish),
                    _ => None,
                };
            } else if matches!(
                app.active_dialog(),
                Some(Dialog::DevtoolDeployConfirmation(_))
            ) {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmDevtoolDeploy),
                    Input::Esc => update(&mut app, Action::CancelDevtoolDeployConfirmation),
                    _ => None,
                };
                if let Some(Effect::DevtoolDeploy(request)) = effect {
                    begin_devtool_job(
                        &mut app,
                        &mut devtool_jobs,
                        &mut devtool_runner,
                        &session_build_dir,
                        cancellation_timeout,
                        request.into(),
                    )
                    .await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::DevtoolDeploy { .. })) {
                let _ = match input {
                    Input::Char(character) => {
                        update(&mut app, Action::AppendDevtoolDeployTarget(character))
                    }
                    Input::Backspace => update(&mut app, Action::BackspaceDevtoolDeployTarget),
                    Input::Enter => update(&mut app, Action::PreviewDevtoolDeploy),
                    Input::Esc => update(&mut app, Action::CancelDevtoolDeploy),
                    _ => None,
                };
            } else if matches!(app.active_dialog(), Some(Dialog::BbmaskConfirmation(_))) {
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
            } else if matches!(app.active_dialog(), Some(Dialog::BbmaskEdit { .. })) {
                let _ = match input {
                    Input::Char(character) => update(&mut app, Action::AppendBbmask(character)),
                    Input::Backspace => update(&mut app, Action::BackspaceBbmask),
                    Input::Enter => update(&mut app, Action::PreviewBbmaskEdit),
                    Input::Esc => update(&mut app, Action::CancelBbmaskEdit),
                    _ => None,
                };
            } else if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
                let action = if input == Input::Enter
                    && app.build.status == BuildStatus::Failed
                    && app.build.errors > 0
                {
                    Action::OpenBuildCompletionErrors
                } else {
                    Action::DismissBuildCompletion
                };
                let _ = update(&mut app, action);
            } else if matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))) {
                let _ = match input {
                    Input::Up => update(&mut app, Action::SelectImage { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectImage { delta: 1 }),
                    Input::Enter => update(&mut app, Action::ConfirmImagePicker),
                    Input::Esc => update(&mut app, Action::CancelImagePicker),
                    _ => None,
                };
            } else if matches!(app.active_dialog(), Some(Dialog::RecipeTaskPicker(_))) {
                let _ = match input {
                    Input::Up => update(&mut app, Action::SelectRecipeTask { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectRecipeTask { delta: 1 }),
                    Input::Enter => update(&mut app, Action::PreviewSelectedRecipeTask),
                    Input::Esc => update(&mut app, Action::CancelRecipeTaskPicker),
                    _ => None,
                };
            } else if matches!(app.active_dialog(), Some(Dialog::RecipeTaskLogPicker(_))) {
                let effect = match input {
                    Input::Up => update(&mut app, Action::SelectRecipeTaskLog { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectRecipeTaskLog { delta: 1 }),
                    Input::Enter => update(&mut app, Action::OpenSelectedRecipeTaskLog),
                    Input::Esc => update(&mut app, Action::CancelRecipeTaskLogPicker),
                    _ => None,
                };
                if let Some(Effect::OpenInEditor(path)) = effect {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::RecipePatchPicker(_))) {
                let effect = match input {
                    Input::Up => update(&mut app, Action::SelectRecipePatch { delta: -1 }),
                    Input::Down => update(&mut app, Action::SelectRecipePatch { delta: 1 }),
                    Input::Enter => update(&mut app, Action::OpenSelectedRecipePatch),
                    Input::Esc => update(&mut app, Action::CancelRecipePatchPicker),
                    _ => None,
                };
                if let Some(Effect::OpenInEditor(path)) = effect {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::ConfigSourcePicker(_))) {
                let effect =
                    config_source_picker_action(input).and_then(|action| update(&mut app, action));
                if let Some(Effect::OpenInEditor(path)) = effect {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::ConfigScopePicker(_))) {
                let effect =
                    config_scope_picker_action(input).and_then(|action| update(&mut app, action));
                if let Some(Effect::GetVariable(identity)) = effect {
                    load_config_variable(&mut app, backend.as_mut(), identity).await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::ConfigComparison(_))) {
                if let Some(action) = config_compare_dialog_action(input) {
                    let _ = update(&mut app, action);
                }
            } else if matches!(app.active_dialog(), Some(Dialog::ConfigEdit { .. })) {
                if let Some(action) = config_edit_dialog_action(input) {
                    let _ = update(&mut app, action);
                }
            } else if matches!(app.active_dialog(), Some(Dialog::ConfigEditConfirmation(_))) {
                if let Some(action) = config_edit_confirmation_action(input)
                    && let Some(Effect::WriteConfigAssignment(request)) = update(&mut app, action)
                {
                    execute_config_edit_write(
                        backend.as_mut(),
                        &mut app,
                        &session_build_dir,
                        request,
                    )
                    .await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::RecipeTaskConfirmation(_))) {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmRecipeTask),
                    Input::Esc => update(&mut app, Action::CancelRecipeTask),
                    _ => None,
                };
                if let Some(Effect::Start(request)) = effect {
                    begin_build(&mut backend, &mut app, &mut build_jobs, request).await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::BuildOptions)) {
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
                    begin_build(&mut backend, &mut app, &mut build_jobs, request).await;
                }
            } else if matches!(app.active_dialog(), Some(Dialog::BuildTarget { .. })) {
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
                    begin_build(&mut backend, &mut app, &mut build_jobs, request).await;
                }
            } else if app.notification.is_some()
                && !(app.screen == Screen::Settings
                    && app.settings_dirty
                    && input == Input::Char('r'))
            {
                if input == Input::Enter {
                    let _ = update(&mut app, Action::ActivateNotification);
                } else if input == Input::Esc {
                    let _ = update(&mut app, Action::DismissNotification);
                }
            } else if app.screen == Screen::Settings && settings_action(input).is_some() {
                let action = settings_action(input).expect("settings action was checked");
                if matches!(update(&mut app, action), Some(Effect::PersistSettings)) {
                    let result = persist_settings(session_path.as_deref(), &mut session, &app);
                    let persistence_action = match result {
                        Ok(()) => Action::SettingsPersisted,
                        Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                    };
                    let _ = update(&mut app, persistence_action);
                }
            } else if app.screen == Screen::Tasks
                && tasks_action(app.task_filter_editing, input).is_some()
            {
                let action =
                    tasks_action(app.task_filter_editing, input).expect("Tasks action was checked");
                let _ = update(&mut app, action);
            } else if app.screen == Screen::Logs && logs_action(app.logs.searching, input).is_some()
            {
                let action =
                    logs_action(app.logs.searching, input).expect("Logs action was checked");
                match update(&mut app, action) {
                    Some(Effect::OpenInEditor(path)) => {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                    Some(Effect::CopyToClipboard(content)) => {
                        copy_to_clipboard(&mut app, content).await;
                    }
                    _ => {}
                }
            } else if app.screen == Screen::Errors && errors_action(input).is_some() {
                let action = errors_action(input).expect("Errors action was checked");
                if let Some(Effect::OpenInEditor(path)) = update(&mut app, action) {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
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
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('f') {
                let _ = update(&mut app, Action::BeginSelectedRecipeForceTask);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('v') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDevshell);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('K') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDiffconfig);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('z') {
                let _ = update(&mut app, Action::BeginSelectedRecipeDiffsigs);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('V') {
                let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('X') {
                let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('e') {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::OpenSelectedRecipeProvider)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('o') {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::BeginSelectedRecipeTaskLog)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('p') {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::BeginSelectedRecipePatchReview)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
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
                    Some(Effect::OpenWorkspaceEditor { label, root }) => Some((label, root)),
                    _ => None,
                };
                if let Some((recipe, root)) = root {
                    open_workspace_editor(&mut app, recipe, root).await;
                }
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('t') {
                inspect_selected_devtool(&mut app, &session_build_dir).await;
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
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Enter {
                if let Some(Effect::GetRecipeMetadata(recipe)) =
                    update(&mut app, Action::BeginSelectedRecipeMetadata)
                {
                    match backend.get_recipe_metadata(recipe.clone()).await {
                        Ok(metadata) => {
                            let _ = update(&mut app, Action::RecipeMetadataLoaded(metadata));
                        }
                        Err(error) => {
                            let _ = update(
                                &mut app,
                                Action::RecipeMetadataFailed {
                                    recipe,
                                    message: error.to_string(),
                                },
                            );
                        }
                    }
                }
                inspect_selected_devtool(&mut app, &session_build_dir).await;
            } else if input == Input::Char('b') {
                let _ = update(&mut app, Action::BeginCurrentImageBuild);
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
                && matches!(
                    input,
                    Input::Up | Input::Down | Input::Char('k') | Input::Char('j')
                )
            {
                if let Some(action) = config_workspace_action(false, input) {
                    let _ = update(&mut app, action);
                }
            } else if app.screen == yoctui_model::Screen::Configuration && input == Input::Enter {
                inspect_selected_config_variable(&mut app, backend.as_mut()).await;
            } else if app.screen == yoctui_model::Screen::Configuration
                && matches!(
                    input,
                    Input::Char('s') | Input::Char('c') | Input::Char('E')
                )
            {
                if let Some(action) = config_workspace_action(false, input) {
                    let _ = update(&mut app, action);
                }
            } else if app.screen == yoctui_model::Screen::Configuration
                && matches!(input, Input::Char('C') | Input::Char('U'))
            {
                if let Some(Effect::CopyToClipboard(content)) = config_copy_effect(&mut app, input)
                {
                    copy_to_clipboard(&mut app, content).await;
                }
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
                    if devtool_jobs.active_job_id().is_some() {
                        if let Some(job_action) = devtool_jobs.request_cancellation() {
                            let _ = update(&mut app, job_action);
                        }
                        let cancellation = if let Some(runner) = devtool_runner.as_mut() {
                            runner.cancel().await.map(|_| ())
                        } else {
                            Err(yoctui_bitbake::DevtoolRunnerError::NotRunning)
                        };
                        if let Err(error) = cancellation {
                            for action in devtool_jobs
                                .cancellation_failed(error.to_string(), SystemTime::now())
                            {
                                let _ = update(&mut app, action);
                            }
                        }
                    } else if let Some(Effect::Cancel) = update(&mut app, action) {
                        if let Some(job_action) = build_jobs.request_cancellation() {
                            let _ = update(&mut app, job_action);
                        }
                        if let Err(error) = backend.cancel_build().await {
                            for action in
                                build_jobs.cancellation_failed(error.to_string(), SystemTime::now())
                            {
                                let _ = update(&mut app, action);
                            }
                        }
                    }
                } else {
                    let _ = update(&mut app, action);
                }
            }
        }
        let completed_devtool =
            poll_devtool_job(&mut app, &mut devtool_jobs, &mut devtool_runner).await;
        if matches!(
            completed_devtool,
            Some(DevtoolOperation::Modify { ref recipe })
                if pending_devtool_modify
                    .as_ref()
                    .is_some_and(|identity| &identity.name == recipe)
        ) {
            if let Some(identity) = pending_devtool_modify.take() {
                complete_devtool_modify(&mut app, &session_build_dir, identity).await;
            }
        } else if devtool_jobs.active_operation().is_none() {
            pending_devtool_modify = None;
        }
        match tokio::time::timeout(Duration::from_millis(1), backend.next_event()).await {
            Ok(Ok(event)) => {
                for action in build_jobs.actions_for_backend_event(event, SystemTime::now()) {
                    let _ = update(&mut app, action);
                }
            }
            Ok(Err(error)) => {
                for action in build_jobs.backend_lost(error.to_string(), SystemTime::now()) {
                    let _ = update(&mut app, action);
                }
            }
            Err(_) => {}
        }
        if app.should_quit {
            break;
        }
    }
    backend.shutdown().await?;
    session.last_target = app.build.target;
    session.last_screen = Some(app.screen);
    session.log_filter = app.logs.filter;
    session.log_recipe_filter = app.logs.recipe_filter;
    session.log_task_filter = app.logs.task_filter;
    session.log_build_filter = app.logs.build_filter;
    session.log_wrap = Some(app.logs.wrap);
    session.log_follow = Some(app.logs.follow);
    session.theme = Some(app.theme);
    session.animation_speed = Some(app.animation_speed);
    session.reduced_motion = Some(app.reduced_motion);
    session.color_enabled = Some(app.color_enabled);
    session.last_backend = Some(backend_kind);
    session.recent_build_dirs = std::iter::once(session_build_dir)
        .chain(session.recent_build_dirs)
        .fold(Vec::new(), |mut directories, directory| {
            if !directories.contains(&directory) && directories.len() < 10 {
                directories.push(directory);
            }
            directories
        });
    write_session(session_path.as_deref(), &session)?;
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
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlP),
        KeyCode::F(5) => Some(Input::F5),
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
                log_build_filter: Some("core-image-minimal".into()),
                log_wrap: Some(true),
                log_follow: Some(false),
                theme: Some(Theme::MatrixGreen),
                animation_speed: Some(AnimationSpeed::Slow),
                reduced_motion: Some(true),
                color_enabled: Some(true),
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
                log_build_filter: Some("core-image-minimal".into()),
                log_wrap: Some(true),
                log_follow: Some(false),
                theme: Some(Theme::MatrixGreen),
                animation_speed: Some(AnimationSpeed::Slow),
                reduced_motion: Some(true),
                color_enabled: Some(true),
                last_backend: Some(Backend::Process),
                recent_build_dirs: vec![PathBuf::from("/build")],
            }
        );
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&directory).unwrap();
    }

    #[test]
    fn settings_session_overrides_config_but_cli_no_color_remains_authoritative() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-settings-precedence-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let config_path = directory.join("config.toml");
        fs::write(
            &config_path,
            "theme = 'dark'\nanimation_speed = 'fast'\nreduced_motion = false\ncolor = true\n",
        )
        .unwrap();
        let cli = Cli::try_parse_from([
            "yoctui",
            "--config",
            config_path.to_str().unwrap(),
            "--no-color",
        ])
        .unwrap();
        let session = Session {
            theme: Some(Theme::MatrixGreen),
            animation_speed: Some(AnimationSpeed::Slow),
            reduced_motion: Some(true),
            color_enabled: Some(true),
            ..Session::default()
        };

        let resolved = resolve_config(&cli, &session).unwrap();
        assert_eq!(resolved.theme, Theme::MatrixGreen);
        assert_eq!(resolved.animation_speed, AnimationSpeed::Slow);
        assert!(resolved.reduced_motion);
        assert!(!resolved.color);

        fs::remove_file(config_path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn settings_persistence_preserves_unrelated_session_fields() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-settings-save-{}", std::process::id()));
        let path = directory.join("session.toml");
        let mut session = Session {
            last_target: Some("core-image-minimal".into()),
            recent_build_dirs: vec![PathBuf::from("/build")],
            ..Session::default()
        };
        let mut app = App::new(10, 1_000);
        app.theme = Theme::HighContrast;
        app.animation_speed = AnimationSpeed::Slow;
        app.reduced_motion = true;
        app.color_enabled = false;
        app.logs.wrap = true;
        app.logs.follow = false;

        persist_settings(Some(&path), &mut session, &app).unwrap();
        let saved = read_session(Some(&path)).unwrap();
        assert_eq!(saved.last_target.as_deref(), Some("core-image-minimal"));
        assert_eq!(saved.recent_build_dirs, [PathBuf::from("/build")]);
        assert_eq!(saved.theme, Some(Theme::HighContrast));
        assert_eq!(saved.animation_speed, Some(AnimationSpeed::Slow));
        assert_eq!(saved.reduced_motion, Some(true));
        assert_eq!(saved.color_enabled, Some(false));
        assert_eq!(saved.log_wrap, Some(true));
        assert_eq!(saved.log_follow, Some(false));

        fs::remove_file(path).unwrap();
        fs::remove_dir(directory).unwrap();
    }

    #[test]
    fn settings_persistence_failure_does_not_replace_session_state() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-settings-failure-{}", std::process::id()));
        fs::write(&directory, "not a directory").unwrap();
        let path = directory.join("session.toml");
        let mut session = Session {
            theme: Some(Theme::Dark),
            ..Session::default()
        };
        let mut app = App::new(10, 1_000);
        app.theme = Theme::Light;

        assert!(persist_settings(Some(&path), &mut session, &app).is_err());
        assert_eq!(session.theme, Some(Theme::Dark));

        fs::remove_file(directory).unwrap();
    }

    #[test]
    fn normalizes_task_progress_event() {
        assert!(matches!(
            yoctui_app::model_action_from_backend_event(BackendEvent::TaskProgress {
                recipe: "busybox".into(),
                task: "do_compile".into(),
                progress: Some(25),
            }),
            Some(Action::TaskProgress {
                progress: Some(25),
                ..
            })
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

    fn config_edit_request(build_dir: &Path, name: &str, value: &str) -> ConfigEditRequest {
        ConfigEditRequest {
            identity: VariableIdentity {
                name: name.into(),
                recipe: None,
            },
            value: value.into(),
            destination: build_dir.join("conf/local.conf"),
            assignment: yoctui_model::config_edit_assignment(name, value).unwrap(),
        }
    }

    #[cfg(unix)]
    #[test]
    fn config_edit_write_replaces_or_appends_atomically_with_permissions_and_crlf() {
        use std::os::unix::fs::PermissionsExt;

        let build_dir =
            std::env::temp_dir().join(format!("yoctui-config-edit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&build_dir);
        let conf_dir = build_dir.join("conf");
        fs::create_dir_all(&conf_dir).unwrap();
        let local_conf = conf_dir.join("local.conf");
        fs::write(
            &local_conf,
            "# MACHINE = \"commented\"\r\nMACHINE ??= \"old\"\r\nMACHINE = \"later\"\r\nMACHINE_EXTRA = \"keep\"\r\n",
        )
        .unwrap();
        fs::set_permissions(&local_conf, fs::Permissions::from_mode(0o640)).unwrap();

        write_config_assignment_atomic(
            &build_dir,
            &config_edit_request(&build_dir, "MACHINE", "qemux86-64"),
        )
        .unwrap();
        let replaced = fs::read_to_string(&local_conf).unwrap();
        assert!(replaced.contains("# MACHINE = \"commented\"\r\n"));
        assert!(replaced.contains("MACHINE_EXTRA = \"keep\"\r\n"));
        assert_eq!(replaced.matches("MACHINE = \"qemux86-64\"").count(), 1);
        assert!(!replaced.replace("\r\n", "").contains('\n'));
        assert_eq!(
            fs::metadata(&local_conf).unwrap().permissions().mode() & 0o777,
            0o640
        );

        write_config_assignment_atomic(
            &build_dir,
            &config_edit_request(&build_dir, "DISTRO", "poky"),
        )
        .unwrap();
        let appended = fs::read_to_string(&local_conf).unwrap();
        assert!(appended.ends_with("DISTRO = \"poky\"\r\n"));
        assert!(!appended.replace("\r\n", "").contains('\n'));
        fs::remove_dir_all(build_dir).unwrap();
    }

    #[test]
    fn config_edit_write_rejects_tampering_and_leaves_failed_destination_untouched() {
        let build_dir =
            std::env::temp_dir().join(format!("yoctui-config-reject-{}", std::process::id()));
        let _ = fs::remove_dir_all(&build_dir);
        fs::create_dir_all(build_dir.join("conf")).unwrap();
        let local_conf = build_dir.join("conf/local.conf");
        fs::write(&local_conf, "MACHINE = \"old\"\n").unwrap();
        let original = fs::read(&local_conf).unwrap();
        let mut request = config_edit_request(&build_dir, "MACHINE", "qemux86-64");
        request.assignment = "MACHINE = \"tampered\"".into();
        assert!(write_config_assignment_atomic(&build_dir, &request).is_err());
        assert_eq!(fs::read(&local_conf).unwrap(), original);

        let failed_build =
            std::env::temp_dir().join(format!("yoctui-config-failure-{}", std::process::id()));
        let _ = fs::remove_dir_all(&failed_build);
        let failed_destination = failed_build.join("conf/local.conf");
        fs::create_dir_all(&failed_destination).unwrap();
        let sentinel = failed_destination.join("sentinel");
        fs::write(&sentinel, "unchanged").unwrap();
        assert!(
            write_config_assignment_atomic(
                &failed_build,
                &config_edit_request(&failed_build, "MACHINE", "qemux86-64"),
            )
            .is_err()
        );
        assert_eq!(fs::read_to_string(sentinel).unwrap(), "unchanged");
        assert!(
            fs::read_dir(failed_build.join("conf"))
                .unwrap()
                .all(|entry| !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .contains(".yoctui-"))
        );
        fs::remove_dir_all(build_dir).unwrap();
        fs::remove_dir_all(failed_build).unwrap();
    }

    #[test]
    fn config_edit_write_refresh_success_replaces_detail_and_failure_preserves_it() {
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        let mut app = App::new(10, 1_000);
        let old = VariableDetail {
            identity: identity.clone(),
            effective_value: Some("old".into()),
            unexpanded_value: None,
            provenance: Some("conf/local.conf:1".into()),
            operations: vec![],
            active_overrides: vec![],
        };
        app.variable_details.insert(identity.clone(), old.clone());
        let _ = update(
            &mut app,
            Action::ConfigEditWriteSucceeded {
                identity: identity.clone(),
            },
        );
        finish_config_edit_refresh(
            &mut app,
            identity.clone(),
            Ok(VariableValue {
                value: Some("qemux86-64".into()),
                provenance: Some("conf/local.conf:1".into()),
                ..VariableValue::default()
            }),
        );
        assert_eq!(
            app.variable_details[&identity].effective_value.as_deref(),
            Some("qemux86-64")
        );
        assert_eq!(
            app.notification.as_deref(),
            Some("MACHINE saved and refreshed.")
        );

        app.variable_details.insert(identity.clone(), old.clone());
        let _ = update(
            &mut app,
            Action::ConfigEditWriteSucceeded {
                identity: identity.clone(),
            },
        );
        finish_config_edit_refresh(
            &mut app,
            identity.clone(),
            Err(yoctui_bitbake::BackendError::Bridge("offline".into())),
        );
        assert_eq!(app.variable_details.get(&identity), Some(&old));
        assert!(!app.variable_detail_loading.contains(&identity));
        assert!(app.notification.as_deref().unwrap().contains("offline"));
    }

    #[test]
    #[ignore = "requires YOCTUI_LIVE_BUILD_DIR; validates a copy and never writes the live file"]
    fn config_edit_write_live_snapshot_is_validated_without_mutating_yocto() {
        let live_build = PathBuf::from(
            std::env::var("YOCTUI_LIVE_BUILD_DIR")
                .expect("YOCTUI_LIVE_BUILD_DIR must identify an initialized Yocto build"),
        );
        let live_local_conf = live_build.join("conf/local.conf");
        let live_before = fs::read(&live_local_conf).unwrap();
        let snapshot_build =
            std::env::temp_dir().join(format!("yoctui-config-live-{}", std::process::id()));
        let _ = fs::remove_dir_all(&snapshot_build);
        fs::create_dir_all(snapshot_build.join("conf")).unwrap();
        fs::write(snapshot_build.join("conf/local.conf"), &live_before).unwrap();

        write_config_assignment_atomic(
            &snapshot_build,
            &config_edit_request(&snapshot_build, "MACHINE", "qemux86-64"),
        )
        .unwrap();
        let snapshot = fs::read_to_string(snapshot_build.join("conf/local.conf")).unwrap();
        assert!(snapshot.contains("MACHINE = \"qemux86-64\""));
        assert_eq!(fs::read(&live_local_conf).unwrap(), live_before);
        fs::remove_dir_all(snapshot_build).unwrap();
    }

    #[test]
    fn ctrl_c_is_not_the_regular_cancel_key() {
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(input_from_key(key), Some(Input::CtrlC));
    }

    #[test]
    fn focus_keys_decode_without_losing_direction() {
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
            Some(Input::Tab)
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
            Some(Input::BackTab)
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
            Some(Input::Esc)
        );
    }

    #[test]
    fn dialog_input_routing_prevents_pane_shortcuts_from_leaking() {
        let mut app = App::new(10, 1_000);
        app.focus = yoctui_model::FocusTarget::Inspector;
        let _ = update(&mut app, Action::OpenBuildOptions);
        let selection = app.navigator_selection;

        let tab = input_from_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)).unwrap();
        assert_eq!(yoctui_app::focus_action(app.focus, tab), None);
        let _ = update(&mut app, Action::SelectNavigator { delta: 1 });

        assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
        assert_eq!(app.navigator_selection, selection);
        assert_eq!(app.focus, yoctui_model::FocusTarget::Dialog);
    }

    #[test]
    fn command_palette_input_decodes_search_edit_and_activation_keys() {
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)),
            Some(Input::CtrlP)
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
            Some(Input::Char('x'))
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(Input::Backspace)
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            Some(Input::Enter)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn queued_termination_requests_exit_the_tui_loop() {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        sender.send(()).await.unwrap();
        assert!(termination_requested(&mut receiver));
    }

    #[test]
    fn config_workspace_terminal_result_preserves_typed_scope_and_history() {
        let action = config_variable_loaded_action(
            VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            },
            VariableValue {
                recipe: Some("base-files".into()),
                value: Some("qemux86-64".into()),
                provenance: Some("/build/conf/local.conf:3".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                operations: vec![yoctui_model::VariableOperation {
                    operation: "set".into(),
                    file: Some("/build/conf/local.conf".into()),
                    line: Some(3),
                    value: Some("${DEFAULT_MACHINE}".into()),
                }],
                active_overrides: vec!["qemux86-64".into()],
            },
        );
        assert!(matches!(
            action,
            Action::VariableLoaded(VariableDetail {
                identity: VariableIdentity {
                    name,
                    recipe: Some(recipe),
                },
                unexpanded_value: Some(unexpanded),
                operations,
                active_overrides,
                ..
            }) if name == "MACHINE"
                && recipe == "base-files"
                && unexpanded == "${DEFAULT_MACHINE}"
                && operations.len() == 1
                && active_overrides == ["qemux86-64"]
        ));
    }

    #[test]
    fn config_copy_terminal_route_emits_existing_clipboard_effect() {
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: Some("qemux86-64".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
        assert_eq!(
            config_copy_effect(&mut app, Input::Char('C')),
            Some(Effect::CopyToClipboard("qemux86-64".into()))
        );
        assert_eq!(
            config_copy_effect(&mut app, Input::Char('U')),
            Some(Effect::CopyToClipboard("${DEFAULT_MACHINE}".into()))
        );
    }

    #[test]
    fn config_source_terminal_picker_routes_authoritative_path() {
        let mut app = App::new(10, 1_000);
        app.workspace.build_dir = Some("/build".into());
        app.dialogs.push_back(Dialog::ConfigSourcePicker(
            yoctui_model::ConfigSourcePicker {
                identity: VariableIdentity {
                    name: "MACHINE".into(),
                    recipe: None,
                },
                sources: vec![yoctui_model::ConfigSourceChoice {
                    operation: "set".into(),
                    path: "conf/local.conf".into(),
                    line: Some(12),
                }],
                selection: 0,
            },
        ));
        let effect =
            config_source_picker_action(Input::Enter).and_then(|action| update(&mut app, action));
        assert_eq!(
            effect,
            Some(Effect::OpenInEditor("/build/conf/local.conf".into()))
        );
        assert!(app.active_dialog().is_none());
    }

    #[test]
    fn config_scope_terminal_picker_emits_recipe_scoped_query() {
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: "base-files".into(),
            ..yoctui_model::Recipe::default()
        });
        let _ = update(&mut app, Action::OpenConfigScopePicker);
        let _ = config_scope_picker_action(Input::Down).and_then(|action| update(&mut app, action));
        let effect =
            config_scope_picker_action(Input::Enter).and_then(|action| update(&mut app, action));
        assert_eq!(
            effect,
            Some(Effect::GetVariable(VariableIdentity {
                name: "MACHINE".into(),
                recipe: Some("base-files".into()),
            }))
        );
        assert_eq!(app.config_scope.as_deref(), Some("base-files"));
    }

    #[test]
    fn layer_tree_scanner_is_shallow_sorted_and_keeps_hidden_metadata() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-layer-tree-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(directory.join("recipes-demo")).unwrap();
        fs::write(directory.join("demo.bb"), "SUMMARY = \"demo\"").unwrap();
        fs::write(directory.join(".hidden"), "hidden").unwrap();

        let entries = scan_layer_directory(&directory).unwrap();
        assert_eq!(
            entries[0].path.file_name().unwrap().to_string_lossy(),
            "recipes-demo"
        );
        assert!(entries.iter().any(|entry| entry.is_hidden));
        assert!(entries.iter().all(|entry| entry.depth == 0));
        assert!(!entries.iter().any(|entry| entry.path.ends_with(".git")));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn layer_tree_preview_bounds_large_text_and_rejects_binary() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-layer-tree-preview-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        let text = directory.join("large.conf");
        fs::write(&text, vec![b'A'; 70 * 1024]).unwrap();
        let (content, kind, truncated) = read_layer_preview(&text).unwrap();
        assert_eq!(kind, PreviewKind::Text);
        assert_eq!(content.len(), 64 * 1024);
        assert!(truncated);

        let binary = directory.join("image.bin");
        fs::write(&binary, [0, 159, 146, 150]).unwrap();
        let (content, kind, truncated) = read_layer_preview(&binary).unwrap();
        assert_eq!(kind, PreviewKind::Binary);
        assert!(content.is_empty());
        assert!(!truncated);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn layer_tree_scanner_reports_git_states_without_requiring_git() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-layer-tree-git-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join(".gitignore"), "ignored.bin\n").unwrap();
        fs::write(directory.join("tracked.bb"), "SUMMARY = \"first\"\n").unwrap();
        let initialized = ProcessCommand::new("git")
            .arg("init")
            .arg("-q")
            .current_dir(&directory)
            .status()
            .is_ok_and(|status| status.success());
        if initialized {
            assert!(
                ProcessCommand::new("git")
                    .args(["add", ".gitignore", "tracked.bb"])
                    .current_dir(&directory)
                    .status()
                    .unwrap()
                    .success()
            );
            assert!(
                ProcessCommand::new("git")
                    .args([
                        "-c",
                        "user.name=Yoctui Test",
                        "-c",
                        "user.email=yoctui@example.invalid",
                        "commit",
                        "-qm",
                        "fixture",
                    ])
                    .current_dir(&directory)
                    .status()
                    .unwrap()
                    .success()
            );
            fs::write(directory.join("tracked.bb"), "SUMMARY = \"changed\"\n").unwrap();
            fs::write(directory.join("new.bb"), "SUMMARY = \"new\"\n").unwrap();
            fs::write(directory.join("ignored.bin"), [1, 2, 3]).unwrap();
            let entries = scan_layer_directory(&directory).unwrap();
            let state = |name: &str| {
                entries
                    .iter()
                    .find(|entry| entry.path.ends_with(name))
                    .unwrap()
                    .git
            };
            assert_eq!(state("tracked.bb"), GitFileState::Modified);
            assert_eq!(state("new.bb"), GitFileState::Untracked);
            assert_eq!(state("ignored.bin"), GitFileState::Ignored);
        } else {
            assert!(
                scan_layer_directory(&directory)
                    .unwrap()
                    .iter()
                    .all(|entry| entry.git == GitFileState::Unavailable)
            );
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recipe_bitbake_action_terminal_keys_decode_to_typed_routes() {
        let force = input_from_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE)).unwrap();
        let devshell =
            input_from_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)).unwrap();
        assert_eq!(
            yoctui_app::recipes_workspace_action(false, force),
            Some(Action::BeginSelectedRecipeForceTask)
        );
        assert_eq!(
            yoctui_app::recipes_workspace_action(false, devshell),
            Some(Action::BeginSelectedRecipeDevshell)
        );
    }

    #[test]
    fn recipe_navigation_terminal_keys_and_missing_paths_are_typed() {
        for (key, expected) in [
            ('e', Action::OpenSelectedRecipeProvider),
            ('o', Action::BeginSelectedRecipeTaskLog),
            ('p', Action::BeginSelectedRecipePatchReview),
            ('d', Action::BeginSelectedRecipeDevtoolModify),
            ('u', Action::BeginSelectedRecipeDevtoolUpdateRecipe),
            ('F', Action::BeginSelectedRecipeDevtoolFinish),
            ('P', Action::BeginSelectedRecipeDevtoolDeploy),
            ('D', Action::BeginSelectedRecipeDevtoolReset),
        ] {
            let input =
                input_from_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE)).unwrap();
            assert_eq!(
                yoctui_app::recipes_workspace_action(false, input),
                Some(expected)
            );
        }
        let missing = std::env::temp_dir().join(format!(
            "yoctui-recipe-navigation-missing-{}",
            std::process::id()
        ));
        let _ = fs::remove_file(&missing);
        let error = editor_path_error(&missing).unwrap();
        assert!(error.contains("no longer exists"), "{error}");

        let missing_editor = std::ffi::OsStr::new("yoctui-editor-that-does-not-exist");
        assert_eq!(
            run_editor_process(missing_editor, Path::new("/tmp"))
                .unwrap_err()
                .kind(),
            std::io::ErrorKind::NotFound
        );
        let status = ProcessCommand::new("/bin/sh")
            .args(["-c", "exit 7"])
            .status()
            .unwrap();
        let error = editor_exit_error(status, "/tmp/recipe.bb").unwrap();
        assert!(error.contains("exit status: 7"), "{error}");
    }

    #[test]
    fn recipe_qa_action_terminal_keys_decode_to_typed_routes() {
        for (key, expected) in [
            ('V', Action::BeginSelectedRecipeCveCheck),
            ('X', Action::BeginSelectedRecipeSpdx),
        ] {
            let input =
                input_from_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE)).unwrap();
            assert_eq!(
                yoctui_app::recipes_workspace_action(false, input),
                Some(expected)
            );
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn devtool_job_lifecycle_cli_polling_retains_output_during_navigation() {
        use std::os::unix::fs::PermissionsExt;

        let script =
            std::env::temp_dir().join(format!("yoctui-devtool-lifecycle-{}", std::process::id()));
        fs::write(&script, "#!/bin/sh\nprintf 'background output\\n'\n").unwrap();
        let mut permissions = fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script, permissions).unwrap();

        let operation = DevtoolOperation::Reset {
            recipe: "busybox".into(),
        };
        let command = DevtoolCommandSpec::with_executable(script.clone(), &operation).unwrap();
        let mut coordinator = DevtoolJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        for action in coordinator
            .queue(operation, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = update(&mut app, action);
        }
        let id = coordinator.active_job_id().unwrap();
        let mut started = DevtoolJobRunner::new(std::env::temp_dir());
        started.start(command).await.unwrap();
        let mut runner = Some(started);
        app.screen = Screen::Dashboard;
        let mut completed = None;
        tokio::time::timeout(Duration::from_secs(2), async {
            while runner.is_some() {
                let result = poll_devtool_job(&mut app, &mut coordinator, &mut runner).await;
                if result.is_some() {
                    completed = result;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();

        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert_eq!(job.output[0].message, "background output");
        assert_eq!(
            job.output[0].source,
            yoctui_model::BackgroundJobOutputSource::Stdout
        );
        assert_eq!(
            completed,
            Some(DevtoolOperation::Reset {
                recipe: "busybox".into()
            })
        );
        assert_eq!(app.screen, Screen::Dashboard);
        fs::remove_file(script).unwrap();
    }

    #[tokio::test]
    async fn devtool_modify_completion_uses_authoritative_source_and_preserves_failures() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-devtool-modify-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("main.c"), "int main(void) { return 0; }\n").unwrap();
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        let mut app = App::new(10, 1_000);
        app.workspace.recipes.push(yoctui_model::Recipe {
            name: identity.name.clone(),
            file: Some(identity.file.clone()),
            ..yoctui_model::Recipe::default()
        });
        let job_id = yoctui_model::BackgroundJobId(1_u64 << 63);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id: job_id,
                kind: yoctui_model::BackgroundJobKind::Devtool,
                title: "Devtool modify busybox".into(),
                context: yoctui_model::BackgroundJobContext {
                    recipe: Some("busybox".into()),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id: job_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id: job_id });
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id: job_id,
                result: yoctui_model::BackgroundJobResult {
                    summary: "Devtool completed successfully".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );

        apply_completed_devtool_modify_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity: identity.clone(),
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: directory.clone(),
                    recipe_file: Some(identity.file.clone()),
                },
                git: yoctui_model::DevtoolGitState::NotRepository,
                error: None,
            },
        )
        .await;
        assert!(app.devtool_statuses.contains_key(&identity));
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeEditor(editor))
                if editor.recipe == "busybox"
                    && editor.root == directory
                    && editor.content.contains("int main")
        ));
        assert_eq!(
            app.background_jobs.get(job_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Succeeded
        );

        let _ = update(&mut app, Action::CloseRecipeEditor);
        let missing = directory.join("missing");
        apply_completed_devtool_modify_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity: identity.clone(),
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::MissingDirectory {
                    source_path: missing.clone(),
                },
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: None,
            },
        )
        .await;
        assert!(app.active_dialog().is_none());
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains(&missing.display().to_string()))
        );
        apply_completed_devtool_modify_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity: identity.clone(),
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                    exit_code: Some(7),
                    message: "status failed".into(),
                }),
            },
        )
        .await;
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status refresh failed"))
        );
        apply_completed_devtool_modify_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity: identity.clone(),
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::Present {
                    source_path: missing,
                    recipe_file: Some(identity.file),
                },
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: None,
            },
        )
        .await;
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("Could not list workspace files"))
        );
        assert_eq!(
            app.background_jobs.get(job_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Succeeded
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
