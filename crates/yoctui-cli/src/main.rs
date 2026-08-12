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
    collections::BTreeMap,
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
    BuildJobCoordinator, DevtoolJobCoordinator, Input, build_environment_action,
    config_compare_dialog_action, config_edit_confirmation_action, config_scope_picker_action,
    config_source_picker_action, config_workspace_action, dependency_workspace_action,
    devtool_deploy_confirmation_action, devtool_deploy_dialog_action,
    devtool_finish_confirmation_action, devtool_finish_picker_action,
    devtool_modify_confirmation_action, devtool_reset_confirmation_action,
    devtool_update_confirmation_action, errors_action, focus_action, images_workspace_action,
    key_action, logs_action, maintenance_dialog_action, maintenance_workspace_action,
    model_action_from_backend_event, package_workspace_action, qa_dialog_action,
    qa_layer_capability_action, qa_layer_runner_action, qa_report_error_action,
    qa_report_response_action, qa_task_capability_action, qa_workspace_action,
    qemu_actions_for_runner_event, qemu_cancellation_confirmation_action,
    qemu_launch_confirmation_action, qemu_launch_dialog_action, recipe_editor_action,
    sdk_actions_for_runner_event, sdk_build_confirmation_action,
    sdk_cancellation_confirmation_action, sdk_native_confirmation_action, sdk_native_dialog_action,
    sdk_publish_confirmation_action, sdk_publish_dialog_action, sdk_workspace_action,
    security_actions_for_mapper_event, security_dialog_action, security_workspace_action,
    settings_action, signature_task_picker_action, signature_workspace_action, tasks_action,
    test_actions_for_runner_event, test_cancellation_confirmation_action,
    test_comparison_confirmation_action, test_comparison_dialog_action,
    test_comparison_workspace_action, test_junit_confirmation_action, test_junit_dialog_action,
    test_launch_confirmation_action, test_launch_dialog_action,
    test_result_actions_for_runner_event, test_result_import_dialog_action,
    test_results_import_action, test_results_workspace_action, testing_workspace_action,
    wic_actions_for_runner_event, wic_cancellation_confirmation_action,
    wic_create_confirmation_action, wic_create_dialog_action, wic_device_picker_action,
    wic_write_confirmation_action, wic_write_phrase_action,
};
use yoctui_bitbake::{
    BackendEvent, BitBakeBackend, BridgeBackend, BuildEnvironmentAdapter, DevtoolCommandSpec,
    DevtoolInspector, DevtoolJobRunner, DevtoolRunnerEvent, ImageArtifactAdapter,
    ImageArtifactCancellation, PackageDataAdapter, PackageDataCancellation, ProcessBackend,
    QaConfiguredLayerInput, QaFamilyTaskBinding, QaLayerCapabilityInput,
    QaLayerCapabilityInspector, QaLayerCommandSpec, QaLayerJobRunner, QaLayerRunnerEvent,
    QaReportAdapter, QaReportAdapterError, QaReportCancellation, QaReportCandidate, QaReportOrigin,
    QaReportRootInput, QaReportScanInput, QaTaskCapabilityInput, QaTaskCapabilityInspector,
    QaTaskScopeInput, QemuAdapterError, QemuCapabilityInspector, QemuCommandSpec, QemuJobRunner,
    QemuRunnerEvent, SdkArtifactAdapter, SdkArtifactCancellation, SdkArtifactScanOutcome,
    SdkToolAdapter, SdkToolAdapterError, SdkToolCommandSpec, SdkToolJobRunner, SdkToolRunnerEvent,
    SecurityCapabilityInput, SecurityCapabilityInspector, SecurityMapperCommandSpec,
    SecurityMapperJobRunner, SecurityMapperRunnerEvent, SecurityReportAdapter,
    SecurityReportAdapterError, SecurityReportCancellation, SecurityReportScanOutcome,
    SignatureAdapter, SignatureCancellation, TestResultAdapter, TestResultJob, TestResultOperation,
    TestResultRunnerEvent, TestRunnerAdapter, TestRunnerEvent, TestRunnerJob, VariableValue,
    WicAdapterError, WicCapabilityInspector, WicCreateCommandSpec, WicDeviceInspector,
    WicDeviceInventoryResponse, WicJobRunner, WicRunnerEvent,
};
use yoctui_model::{
    Action, AnimationSpeed, App, AppError, BuildRequest, BuildStatus, ConfigEditRequest,
    DevtoolOperation, DevtoolWorkspace, Dialog, Effect, GitFileState, HostTelemetry,
    ImageArtifactInventoryState, ImageArtifactRequest, LayerBrowserEntry, LayerInspectorMode,
    LayerRelationship, LayerRelationships, PackageDetailRequest, PackageInventoryRequest,
    PreviewKind, QaAction, QaCheckFamily, QaCheckId, QaEffect, QaFindingScope, QaLayerIdentity,
    QaLayerSessionId, QaReportFormat, QaReportIdentity, QaReportRequest, QaScope, QaSessionId,
    QaSessionStatus, QaSourceLocation, QemuCapability, QemuLaunchDraft, QemuLaunchPreview,
    QemuLaunchRequest, QemuSessionId, RecipeIdentity, Screen, SdkArtifactInventoryRequest,
    SdkNativePreview, SdkOperation, SdkPublishPreview, SdkSessionId, SdkToolCapability,
    SecurityAction, SecurityEffect, SecurityOperation, SecurityReportRequest, SecurityScope,
    SecuritySessionId, SecuritySessionStatus, Severity, SignatureComparisonRequest,
    SignatureTarget, TestComparison, TestOperation, TestSessionId, TestWorkspaceView, Theme,
    VariableDetail, VariableIdentity, WicCapability, WicCreateDraft, WicCreatePreview,
    WicCreateRequest, WicDeviceInventoryRequest, WicOperation, WicSessionId, update,
    validate_config_edit_request,
};
use yoctui_ui::render;

mod maintenance_cli;

use maintenance_cli::MaintenanceCliCoordinator;
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
    build_dir_configured: bool,
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
    let configured_build_dir = cli
        .build_dir
        .clone()
        .or_else(|| env::var_os("YOCTUI_BUILD_DIR").map(PathBuf::from))
        .or(file.build_dir);
    let build_dir = configured_build_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from("/"));
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
        build_dir_configured: configured_build_dir.is_some(),
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
    select_backend_with_environment(backend, build_dir, cancellation_timeout, None).await
}

async fn select_backend_with_environment(
    backend: Backend,
    build_dir: PathBuf,
    cancellation_timeout: Option<Duration>,
    environment: Option<BTreeMap<String, String>>,
) -> Result<Box<dyn BitBakeBackend>> {
    match backend {
        Backend::Process => {
            let backend = ProcessBackend::new(build_dir);
            let backend = if let Some(environment) = environment {
                backend.with_environment(environment)
            } else {
                backend
            };
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
            let bridge = if let Some(environment) = environment {
                BridgeBackend::spawn_with_environment(&python, script, build_dir, environment).await
            } else {
                BridgeBackend::spawn(&python, script, build_dir).await
            };
            bridge
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
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Notify("A build background job is already active.".into()),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    match backend.start_build(request).await {
        Ok(()) => true,
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
            false
        }
    }
}

async fn begin_test_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    id: TestSessionId,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::FailTestSession {
                id,
                message: "another managed BitBake build is already active".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(background_job_id) = build_jobs.active_job_id() else {
        let _ = update(
            app,
            Action::FailTestSession {
                id,
                message: "Testing build coordinator did not retain its job identity".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return false;
    };
    let _ = update(
        app,
        Action::AttachTestBuildSession {
            id,
            background_job_id,
        },
    );
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

fn test_build_action_for_event(
    app: &App,
    id: TestSessionId,
    event: &BackendEvent,
) -> Option<Action> {
    let cancelling = app
        .test_session(id)
        .and_then(|session| session.background_job_id)
        .and_then(|job_id| app.background_jobs.get(job_id))
        .is_some_and(|job| job.status == yoctui_model::BackgroundJobStatus::Cancelling);
    match event {
        BackendEvent::BuildStarted => Some(Action::TestSessionRunning { id }),
        BackendEvent::BuildCompleted {
            success: true,
            exit_code,
        } => Some(Action::CompleteTestSession {
            id,
            exit_code: exit_code.unwrap_or(0),
            result_paths: Vec::new(),
            finished_at: SystemTime::now(),
        }),
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } if cancelling => Some(Action::CancelTestSession {
            id,
            exit_code: *exit_code,
            finished_at: SystemTime::now(),
        }),
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } => Some(Action::FailTestSession {
            id,
            message: exit_code.map_or_else(
                || "Testing BitBake task failed without an exit code".into(),
                |code| format!("Testing BitBake task failed with exit code {code}"),
            ),
            exit_code: *exit_code,
            finished_at: SystemTime::now(),
        }),
        BackendEvent::CommandFailed { code, message } => Some(Action::FailTestSession {
            id,
            message: format!("{code}: {message}"),
            exit_code: None,
            finished_at: SystemTime::now(),
        }),
        BackendEvent::Disconnected => Some(Action::LoseTestSession {
            id,
            message: "BitBake backend disconnected during Testing".into(),
            finished_at: SystemTime::now(),
        }),
        _ => None,
    }
}

async fn begin_security_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    id: SecuritySessionId,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Security(SecurityAction::FailSession {
                id,
                message: "another managed BitBake build is already active".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(background_job_id) = build_jobs.active_job_id() else {
        let _ = update(
            app,
            Action::Security(SecurityAction::LoseSession {
                id,
                message: "Security build coordinator did not retain its job identity".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    let _ = update(
        app,
        Action::Security(SecurityAction::AttachBackgroundJob {
            id,
            background_job_id,
        }),
    );
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

fn security_build_action_for_event(
    app: &App,
    id: SecuritySessionId,
    event: &BackendEvent,
) -> Option<Action> {
    let cancelling = app
        .security
        .sessions
        .iter()
        .find(|session| session.preview.id == id)
        .is_some_and(|session| session.status == SecuritySessionStatus::Cancelling);
    match event {
        BackendEvent::BuildStarted => Some(Action::Security(SecurityAction::SessionRunning(id))),
        BackendEvent::BuildCompleted { success: true, .. } => {
            Some(Action::Security(SecurityAction::CompleteSession {
                id,
                result_paths: Vec::new(),
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted { success: false, .. } if cancelling => {
            Some(Action::Security(SecurityAction::CancelSession {
                id,
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } => Some(Action::Security(SecurityAction::FailSession {
            id,
            message: exit_code.map_or_else(
                || "Security BitBake task failed without an exit code".into(),
                |code| format!("Security BitBake task failed with exit code {code}"),
            ),
            finished_at: SystemTime::now(),
        })),
        BackendEvent::CommandFailed { code, message } => {
            Some(Action::Security(SecurityAction::FailSession {
                id,
                message: format!("{code}: {message}"),
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::Disconnected => Some(Action::Security(SecurityAction::LoseSession {
            id,
            message: "BitBake backend disconnected during Security operation".into(),
            finished_at: SystemTime::now(),
        })),
        _ => None,
    }
}

async fn begin_qa_build(
    backend: &mut Box<dyn BitBakeBackend>,
    app: &mut App,
    build_jobs: &mut BuildJobCoordinator,
    session: QaSessionId,
    request: BuildRequest,
) -> bool {
    let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) else {
        let _ = update(
            app,
            Action::Qa(QaAction::FailSession {
                session,
                message: "another managed BitBake build is already active".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    for action in actions {
        let _ = update(app, action);
    }
    let Some(background_job) = build_jobs.active_job_id() else {
        let _ = update(
            app,
            Action::Qa(QaAction::LoseSession {
                session,
                message: "QA build coordinator did not retain its job identity".into(),
                finished_at: SystemTime::now(),
            }),
        );
        return false;
    };
    let _ = update(
        app,
        Action::Qa(QaAction::AttachBackgroundJob {
            session,
            background_job,
        }),
    );
    match backend.start_build(request).await {
        Ok(()) => true,
        Err(error) => {
            let _ = update(
                app,
                Action::Qa(QaAction::FailSession {
                    session,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(app, action);
            }
            false
        }
    }
}

fn qa_build_action_for_event(
    app: &App,
    session: QaSessionId,
    event: &BackendEvent,
) -> Option<Action> {
    let qa_session = app
        .qa
        .sessions
        .iter()
        .find(|candidate| candidate.id == session)?;
    match event {
        BackendEvent::BuildStarted => Some(Action::Qa(QaAction::SessionRunning(session))),
        BackendEvent::BuildCompleted { success: true, .. } => {
            Some(Action::Qa(QaAction::CompleteSession {
                session,
                result_paths: qa_session.operation.report_roots.clone(),
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted { success: false, .. }
            if qa_session.status == QaSessionStatus::Cancelling =>
        {
            Some(Action::Qa(QaAction::CancelSession {
                session,
                finished_at: SystemTime::now(),
            }))
        }
        BackendEvent::BuildCompleted {
            success: false,
            exit_code,
        } => Some(Action::Qa(QaAction::FailSession {
            session,
            message: exit_code.map_or_else(
                || "QA BitBake task failed without an exit code".into(),
                |code| format!("QA BitBake task failed with exit code {code}"),
            ),
            finished_at: SystemTime::now(),
        })),
        BackendEvent::CommandFailed { code, message } => Some(Action::Qa(QaAction::FailSession {
            session,
            message: format!("{code}: {message}"),
            finished_at: SystemTime::now(),
        })),
        BackendEvent::Disconnected => Some(Action::Qa(QaAction::LoseSession {
            session,
            message: "BitBake backend disconnected during QA".into(),
            finished_at: SystemTime::now(),
        })),
        _ => None,
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

const SDK_TOOL_OPERATION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

struct SdkArtifactBackgroundOperation {
    request: SdkArtifactInventoryRequest,
    cancellation: SdkArtifactCancellation,
    handle: tokio::task::JoinHandle<
        Result<yoctui_bitbake::SdkArtifactResponse, yoctui_bitbake::SdkArtifactAdapterError>,
    >,
}

struct SdkCapabilityBackgroundOperation {
    handle: tokio::task::JoinHandle<SdkToolCapability>,
}

struct SdkCliOperation {
    id: SdkSessionId,
    operation: SdkOperation,
    starting: Option<tokio::task::JoinHandle<(SdkToolJobRunner, Result<(), SdkToolAdapterError>)>>,
    runner: Option<SdkToolJobRunner>,
    timeout_wait: Option<
        tokio::task::JoinHandle<(
            SdkToolJobRunner,
            Result<SdkToolRunnerEvent, SdkToolAdapterError>,
        )>,
    >,
    cancellation:
        Option<tokio::task::JoinHandle<(SdkToolJobRunner, Result<bool, SdkToolAdapterError>)>>,
}

fn sdk_tool_adapter_for_workspace(
    app: &App,
    build_directory: &Path,
) -> Result<SdkToolAdapter, String> {
    let build_directory = fs::canonicalize(build_directory)
        .map_err(|error| format!("active build directory is unavailable: {error}"))?;
    if !build_directory.is_dir() {
        return Err("active build directory is not a canonical directory".into());
    }
    let sdk_deploy_root = app
        .workspace
        .variables
        .get("SDK_DEPLOY")
        .map(PathBuf::from)
        .ok_or_else(|| "SDK_DEPLOY is unavailable from the active Yocto workspace".to_owned())?;
    let mut workspace_roots = vec![build_directory.clone()];
    if let Some(source_directory) = app.workspace.source_dir.as_ref() {
        let source_directory = fs::canonicalize(source_directory)
            .map_err(|error| format!("active Yocto source directory is unavailable: {error}"))?;
        if !source_directory.is_dir() {
            return Err("active Yocto source directory is not a canonical directory".into());
        }
        workspace_roots.push(source_directory);
    }
    workspace_roots.sort();
    workspace_roots.dedup();
    Ok(SdkToolAdapter::new(
        build_directory,
        sdk_deploy_root,
        workspace_roots,
    ))
}

fn begin_sdk_capability_operation(
    app: &mut App,
    adapter: Option<&SdkToolAdapter>,
    operation: &mut Option<SdkCapabilityBackgroundOperation>,
    effect: Effect,
) {
    if !matches!(effect, Effect::InspectSdkTools) {
        return;
    }
    if let Some(stale) = operation.take() {
        stale.handle.abort();
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::SdkToolCapabilityLoaded(SdkToolCapability::Failed {
                message: "SDK tools cannot be inspected without a canonical build directory, SDK_DEPLOY, and authoritative workspace roots".into(),
            }),
        );
        return;
    };
    let handle = tokio::task::spawn_blocking(move || adapter.capability());
    *operation = Some(SdkCapabilityBackgroundOperation { handle });
}

async fn poll_sdk_capability_operation(
    app: &mut App,
    operation: &mut Option<SdkCapabilityBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let capability = match operation.handle.await {
        Ok(capability) => capability,
        Err(error) => SdkToolCapability::Failed {
            message: format!("SDK capability inspection task was lost: {error}"),
        },
    };
    let _ = update(app, Action::SdkToolCapabilityLoaded(capability));
}

fn begin_sdk_artifact_operation(
    app: &mut App,
    adapter: Option<&SdkArtifactAdapter>,
    operation: &mut Option<SdkArtifactBackgroundOperation>,
    effect: Effect,
) {
    let Effect::GetSdkArtifacts(request) = effect else {
        return;
    };
    if let Some(stale) = operation.take() {
        stale.cancellation.cancel();
        stale.handle.abort();
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::SdkArtifactInventoryFailed {
                request,
                message: "SDK_DEPLOY is unavailable from the active Yocto workspace".into(),
            },
        );
        return;
    };
    let cancellation = SdkArtifactCancellation::default();
    let worker_cancellation = cancellation.clone();
    let worker_request = request.clone();
    let handle = tokio::spawn(async move {
        adapter
            .scan_with_cancellation(worker_request, worker_cancellation)
            .await
    });
    *operation = Some(SdkArtifactBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

async fn poll_sdk_artifact_operation(
    app: &mut App,
    operation: &mut Option<SdkArtifactBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let action = match operation.handle.await {
        Ok(Ok(response)) => match response.outcome {
            SdkArtifactScanOutcome::Empty => Action::SdkArtifactInventoryLoaded {
                request: response.request,
                artifacts: Vec::new(),
                limitations: Vec::new(),
            },
            SdkArtifactScanOutcome::Complete(artifacts) => Action::SdkArtifactInventoryLoaded {
                request: response.request,
                artifacts,
                limitations: Vec::new(),
            },
            SdkArtifactScanOutcome::Partial {
                artifacts,
                limitations,
            } => Action::SdkArtifactInventoryLoaded {
                request: response.request,
                artifacts,
                limitations,
            },
        },
        Ok(Err(error)) => Action::SdkArtifactInventoryFailed {
            request: operation.request,
            message: error.to_string(),
        },
        Err(error) => Action::SdkArtifactInventoryFailed {
            request: operation.request,
            message: format!("SDK artifact background task was lost: {error}"),
        },
    };
    let _ = update(app, action);
}

fn sdk_command_for_operation(
    adapter: &SdkToolAdapter,
    operation: &SdkOperation,
) -> Result<SdkToolCommandSpec, SdkToolAdapterError> {
    match operation {
        SdkOperation::Publish(request) => {
            let preview = SdkPublishPreview::new(
                request.executable.clone(),
                request.artifact.clone(),
                request.destination.clone(),
            )
            .map_err(|message| SdkToolAdapterError::InvalidRequest(message.into()))?;
            adapter.publication_command(&preview)
        }
        SdkOperation::Native(request) => {
            let preview = SdkNativePreview::new(request.clone())
                .map_err(|message| SdkToolAdapterError::InvalidRequest(message.into()))?;
            adapter.native_command(&preview)
        }
    }
}

fn begin_sdk_job(
    app: &mut App,
    owned: &mut Option<SdkCliOperation>,
    adapter: Option<&SdkToolAdapter>,
    cancellation_timeout: Duration,
    operation_timeout: Duration,
    id: SdkSessionId,
    operation: SdkOperation,
) {
    if owned.is_some() {
        let _ = update(
            app,
            Action::FailSdkSession {
                id,
                message: "another managed SDK tool process is already owned by the CLI".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::FailSdkSession {
                id,
                message: "SDK tool execution is unavailable for the active workspace".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    };
    let worker_operation = operation.clone();
    let starting = tokio::spawn(async move {
        let mut runner = SdkToolJobRunner::new()
            .with_cancellation_timeout(cancellation_timeout)
            .with_operation_timeout(operation_timeout);
        let result = match sdk_command_for_operation(&adapter, &worker_operation) {
            Ok(command) => runner.start(command).await,
            Err(error) => Err(error),
        };
        (runner, result)
    });
    *owned = Some(SdkCliOperation {
        id,
        operation,
        starting: Some(starting),
        runner: None,
        timeout_wait: None,
        cancellation: None,
    });
}

fn begin_sdk_cancellation(
    app: &mut App,
    operation: &mut Option<SdkCliOperation>,
    id: SdkSessionId,
) {
    let Some(active) = operation.as_mut().filter(|active| active.id == id) else {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "the CLI does not own this SDK tool process".into(),
            },
        );
        return;
    };
    if active.cancellation.is_some() {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "SDK tool cancellation is already in progress".into(),
            },
        );
        return;
    }
    if active.timeout_wait.is_some() {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "SDK tool timeout finalization is already in progress".into(),
            },
        );
        return;
    }
    if let Some(starting) = active.starting.take() {
        starting.abort();
        let _ = update(
            app,
            Action::CancelSdkSession {
                id,
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        *operation = None;
        return;
    }
    let Some(mut runner) = active.runner.take() else {
        let _ = update(
            app,
            Action::RejectSdkSessionCancellation {
                id,
                message: "the SDK tool runner is unavailable".into(),
            },
        );
        return;
    };
    active.cancellation = Some(tokio::spawn(async move {
        let result = runner.cancel().await;
        (runner, result)
    }));
}

async fn poll_sdk_job(
    app: &mut App,
    operation: &mut Option<SdkCliOperation>,
) -> Option<SdkOperation> {
    let active = operation.as_mut()?;
    if active
        .starting
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.starting.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(()))) => active.runner = Some(runner),
            Ok((_, Err(error))) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::FailSdkSession {
                        id,
                        message: error.to_string(),
                        exit_code: None,
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return None;
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseSdkSession {
                        id,
                        message: format!("SDK tool startup task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return None;
            }
        }
    }
    if active.starting.is_some() {
        return None;
    }
    if active
        .cancellation
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.cancellation.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(_))) => active.runner = Some(runner),
            Ok((runner, Err(error))) => {
                active.runner = Some(runner);
                let _ = update(
                    app,
                    Action::RejectSdkSessionCancellation {
                        id: active.id,
                        message: error.to_string(),
                    },
                );
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseSdkSession {
                        id,
                        message: format!("SDK tool cancellation task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return None;
            }
        }
    }
    if active.cancellation.is_some() {
        return None;
    }
    if active
        .timeout_wait
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.timeout_wait.take().expect("checked above");
        let event = match handle.await {
            Ok((_runner, Ok(event))) => event,
            Ok((_runner, Err(error))) => SdkToolRunnerEvent::Lost {
                message: error.to_string(),
            },
            Err(error) => SdkToolRunnerEvent::Lost {
                message: format!("SDK timeout finalization task was lost: {error}"),
            },
        };
        let id = active.id;
        for action in sdk_actions_for_runner_event(id, event, SystemTime::now()) {
            let _ = update(app, action);
        }
        *operation = None;
        return None;
    }
    if active.timeout_wait.is_some() {
        return None;
    }
    let runner = active.runner.as_mut()?;
    if runner.operation_timeout_due_within(Duration::from_millis(2)) {
        let mut runner = active.runner.take().expect("runner checked above");
        active.timeout_wait = Some(tokio::spawn(async move {
            let result = runner.next_event().await;
            (runner, result)
        }));
        return None;
    }
    match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await {
        Ok(Ok(event)) => {
            let completed = matches!(event, SdkToolRunnerEvent::Completed { .. });
            let terminal = completed
                || matches!(
                    event,
                    SdkToolRunnerEvent::Failed { .. }
                        | SdkToolRunnerEvent::Cancelled { .. }
                        | SdkToolRunnerEvent::TimedOut { .. }
                        | SdkToolRunnerEvent::Lost { .. }
                );
            let completed_operation = completed.then(|| active.operation.clone());
            for action in sdk_actions_for_runner_event(active.id, event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *operation = None;
            }
            completed_operation
        }
        Ok(Err(error)) => {
            let id = active.id;
            for action in sdk_actions_for_runner_event(
                id,
                SdkToolRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *operation = None;
            None
        }
        Err(_) => None,
    }
}

struct TestSessionCliOperation {
    id: TestSessionId,
    runner: TestRunnerJob,
}

struct TestResultImportCliOperation {
    request: yoctui_model::TestResultImportRequest,
    deadline: tokio::time::Instant,
    handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::TestResultImportResponse, String>,
    >,
}

struct TestResultCliOperation {
    runner: TestResultJob,
    operation: TestResultOperation,
    comparison: Option<TestComparison>,
}

struct TestCliCoordinator {
    runner_adapter: TestRunnerAdapter,
    result_adapter: TestResultAdapter,
    session: Option<TestSessionCliOperation>,
    import: Option<TestResultImportCliOperation>,
    result: Option<TestResultCliOperation>,
}

impl TestCliCoordinator {
    fn new(
        build_directory: PathBuf,
        path_directories: Vec<PathBuf>,
        ptest: yoctui_model::PtestCapability,
    ) -> Self {
        Self {
            runner_adapter: TestRunnerAdapter::new(
                build_directory,
                path_directories.clone(),
                ptest,
            ),
            result_adapter: TestResultAdapter::new(path_directories),
            session: None,
            import: None,
            result: None,
        }
    }

    async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        match effect {
            Effect::InspectTestCapability => {
                let next = update(
                    app,
                    Action::TestCapabilityLoaded(self.runner_adapter.capability()),
                );
                if matches!(next, Some(Effect::InspectResultToolCapability)) {
                    let _ = update(
                        app,
                        Action::ResultToolCapabilityLoaded(self.result_adapter.capability()),
                    );
                }
                true
            }
            Effect::InspectResultToolCapability => {
                let _ = update(
                    app,
                    Action::ResultToolCapabilityLoaded(self.result_adapter.capability()),
                );
                true
            }
            Effect::StartTestSession { id, operation } => {
                self.begin_session(app, id, operation).await;
                true
            }
            Effect::CancelTestSession(id)
                if self.session.as_ref().is_some_and(|active| active.id == id) =>
            {
                let result = self
                    .session
                    .as_mut()
                    .expect("exact active Testing session was checked")
                    .runner
                    .cancel()
                    .await;
                if let Err(error) = result {
                    let _ = update(
                        app,
                        Action::RejectTestSessionCancellation {
                            id,
                            message: error.to_string(),
                        },
                    );
                }
                true
            }
            Effect::ImportTestResults(request) => {
                if let Some(previous) = self.import.take() {
                    previous.handle.abort();
                }
                let adapter = self.result_adapter.clone();
                let owned_request = request.clone();
                self.import = Some(TestResultImportCliOperation {
                    request,
                    deadline: tokio::time::Instant::now() + Duration::from_secs(30),
                    handle: tokio::task::spawn_blocking(move || {
                        adapter
                            .import(&owned_request)
                            .map_err(|error| error.to_string())
                    }),
                });
                true
            }
            Effect::CompareTestResults(request) => {
                self.begin_comparison(app, request).await;
                true
            }
            Effect::InspectTestJunitDestination {
                result,
                destination,
            } => {
                let inspection = self.result_adapter.inspect_junit_destination(destination);
                let _ = update(
                    app,
                    Action::TestJunitDestinationInspected { result, inspection },
                );
                true
            }
            Effect::ExportTestJunit(request) => {
                self.begin_junit(app, request).await;
                true
            }
            _ => false,
        }
    }

    async fn begin_session(&mut self, app: &mut App, id: TestSessionId, operation: TestOperation) {
        if self.session.is_some() {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: "another selftest runner is already active".into(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        }
        let TestOperation::Selftest(request) = operation else {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: "managed BitBake Testing reached the selftest runner".into(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        };
        let command = match self.runner_adapter.command(&request) {
            Ok(command) => command,
            Err(error) => {
                let _ = update(
                    app,
                    Action::FailTestSession {
                        id,
                        message: error.to_string(),
                        exit_code: None,
                        finished_at: SystemTime::now(),
                    },
                );
                return;
            }
        };
        let mut runner = TestRunnerJob::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::FailTestSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        }
        self.session = Some(TestSessionCliOperation { id, runner });
    }

    async fn begin_comparison(
        &mut self,
        app: &mut App,
        request: yoctui_model::TestComparisonRequest,
    ) {
        if self.result.is_some() {
            let _ = update(
                app,
                Action::TestComparisonFailed {
                    request,
                    message: "another resulttool operation is already active".into(),
                },
            );
            return;
        }
        let baseline = app
            .test_results
            .records()
            .iter()
            .find(|record| record.identity == request.baseline)
            .cloned();
        let candidate = app
            .test_results
            .records()
            .iter()
            .find(|record| record.identity == request.candidate)
            .cloned();
        let Some((baseline, candidate)) = baseline.zip(candidate) else {
            let _ = update(
                app,
                Action::TestComparisonFailed {
                    request,
                    message: "an exact comparison input is unavailable".into(),
                },
            );
            return;
        };
        let preview = self
            .result_adapter
            .capability()
            .executable()
            .and_then(|executable| {
                yoctui_model::TestComparisonPreview::new(executable, request.clone())
            });
        let command = preview.map_err(str::to_owned).and_then(|preview| {
            self.result_adapter
                .comparison_command(&preview, &baseline, &candidate)
                .map_err(|error| error.to_string())
        });
        let comparison = TestComparison::between(&baseline, &candidate);
        let (command, comparison) = match (command, comparison.map_err(str::to_owned)) {
            (Ok(command), Ok(comparison)) => (command, comparison),
            (Err(message), _) | (_, Err(message)) => {
                let _ = update(app, Action::TestComparisonFailed { request, message });
                return;
            }
        };
        let mut runner = TestResultJob::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::TestComparisonFailed {
                    request,
                    message: error.to_string(),
                },
            );
            return;
        }
        self.result = Some(TestResultCliOperation {
            runner,
            operation: TestResultOperation::Comparison(request),
            comparison: Some(comparison),
        });
    }

    async fn begin_junit(&mut self, app: &mut App, request: yoctui_model::TestJunitExportRequest) {
        if self.result.is_some() {
            let _ = update(
                app,
                Action::TestJunitExportFailed {
                    request,
                    message: "another resulttool operation is already active".into(),
                },
            );
            return;
        }
        let record = app
            .test_results
            .records()
            .iter()
            .find(|record| record.identity == request.result)
            .cloned();
        let Some(record) = record else {
            let _ = update(
                app,
                Action::TestJunitExportFailed {
                    request,
                    message: "the exact JUnit source result is unavailable".into(),
                },
            );
            return;
        };
        let preview = self
            .result_adapter
            .capability()
            .executable()
            .and_then(|executable| {
                yoctui_model::TestJunitExportPreview::new(executable, request.clone())
            });
        let command = preview.map_err(str::to_owned).and_then(|preview| {
            self.result_adapter
                .junit_command(&preview, &record)
                .map_err(|error| error.to_string())
        });
        let command = match command {
            Ok(command) => command,
            Err(message) => {
                let _ = update(app, Action::TestJunitExportFailed { request, message });
                return;
            }
        };
        let mut runner = TestResultJob::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::TestJunitExportFailed {
                    request,
                    message: error.to_string(),
                },
            );
            return;
        }
        self.result = Some(TestResultCliOperation {
            runner,
            operation: TestResultOperation::Junit(request),
            comparison: None,
        });
    }

    async fn poll(&mut self, app: &mut App) {
        let mut followups = Vec::new();
        if let Some(operation) = self.session.as_mut()
            && let Ok(event) =
                tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await
        {
            let terminal = event.is_err()
                || matches!(
                    event,
                    Ok(TestRunnerEvent::Completed { .. }
                        | TestRunnerEvent::Failed { .. }
                        | TestRunnerEvent::Cancelled { .. }
                        | TestRunnerEvent::TimedOut { .. }
                        | TestRunnerEvent::Lost { .. })
                );
            match event {
                Ok(event) => {
                    for action in
                        test_actions_for_runner_event(operation.id, event, SystemTime::now())
                    {
                        if let Some(effect) = update(app, action) {
                            followups.push(effect);
                        }
                    }
                }
                Err(error) => {
                    let _ = update(
                        app,
                        Action::LoseTestSession {
                            id: operation.id,
                            message: error.to_string(),
                            finished_at: SystemTime::now(),
                        },
                    );
                }
            }
            if terminal {
                self.session = None;
            }
        }
        if self
            .import
            .as_ref()
            .is_some_and(|operation| tokio::time::Instant::now() >= operation.deadline)
        {
            let operation = self.import.take().expect("expired import was checked");
            operation.handle.abort();
            let _ = update(
                app,
                Action::TestResultsTimedOut {
                    request: operation.request,
                },
            );
        } else if self
            .import
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            let operation = self.import.take().expect("finished import was checked");
            let action = match operation.handle.await {
                Ok(Ok(response)) => test_results_import_action(response),
                Ok(Err(message)) => Action::TestResultsFailed {
                    request: operation.request,
                    message,
                },
                Err(error) if error.is_cancelled() => Action::TestResultsCancelled {
                    request: operation.request,
                },
                Err(error) => Action::TestResultsLost {
                    request: operation.request,
                    message: error.to_string(),
                },
            };
            if let Some(effect) = update(app, action) {
                followups.push(effect);
            }
        }
        if let Some(operation) = self.result.as_mut()
            && let Ok(event) =
                tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await
        {
            let terminal = event.is_err()
                || matches!(
                    event,
                    Ok(TestResultRunnerEvent::Completed { .. }
                        | TestResultRunnerEvent::Failed { .. }
                        | TestResultRunnerEvent::Cancelled { .. }
                        | TestResultRunnerEvent::TimedOut { .. }
                        | TestResultRunnerEvent::Lost { .. })
                );
            let event = match event {
                Ok(event) => event,
                Err(error) => TestResultRunnerEvent::Lost {
                    operation: Some(operation.operation.clone()),
                    message: error.to_string(),
                },
            };
            for action in test_result_actions_for_runner_event(
                event,
                operation.comparison.clone(),
                Vec::new(),
            ) {
                if let Some(effect) = update(app, action) {
                    followups.push(effect);
                }
            }
            if terminal {
                self.result = None;
            }
        }
        for effect in followups {
            let _ = self.handle_effect(app, effect).await;
        }
    }
}

struct SecurityCapabilityCliOperation {
    handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_model::SecurityCapabilitySnapshot, String>,
    >,
}

struct SecurityReportCliOperation {
    request: SecurityReportRequest,
    cancellation: SecurityReportCancellation,
    handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::SecurityReportResponse, SecurityReportAdapterError>,
    >,
}

struct SecurityMapperCliOperation {
    id: SecuritySessionId,
    runner: SecurityMapperJobRunner,
}

struct SecurityCliCoordinator {
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
    report_adapter: SecurityReportAdapter,
    capability: Option<SecurityCapabilityCliOperation>,
    report: Option<SecurityReportCliOperation>,
    mapper: Option<SecurityMapperCliOperation>,
}

impl SecurityCliCoordinator {
    fn new(build_directory: PathBuf, path_directories: Vec<PathBuf>) -> Self {
        Self {
            build_directory,
            path_directories,
            report_adapter: SecurityReportAdapter::new(),
            capability: None,
            report: None,
            mapper: None,
        }
    }

    fn owns_mapper(&self, id: SecuritySessionId) -> bool {
        self.mapper.as_ref().is_some_and(|active| active.id == id)
    }

    async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        let Effect::Security(effect) = effect else {
            return false;
        };
        match effect {
            SecurityEffect::InspectCapability => self.begin_capability_inspection(app),
            SecurityEffect::StartPackageMap {
                id,
                executable,
                arguments,
            } => {
                self.begin_mapper(app, id, executable, arguments).await;
            }
            SecurityEffect::CancelSession(id) => self.cancel_mapper(app, id).await,
            SecurityEffect::ImportReports(request) => self.begin_report_scan(request),
            SecurityEffect::OpenPath(_) | SecurityEffect::OpenUrl(_) => return false,
            SecurityEffect::StartBuild { .. } => return false,
        }
        true
    }

    fn begin_capability_inspection(&mut self, app: &mut App) {
        if let Some(stale) = self.capability.take() {
            stale.handle.abort();
        }
        let input = security_capability_input(
            app,
            self.build_directory.clone(),
            self.path_directories.clone(),
        );
        let handle = tokio::task::spawn_blocking(move || {
            let input = input?;
            SecurityCapabilityInspector::new(input)
                .inspect()
                .map_err(|error| error.to_string())
        });
        self.capability = Some(SecurityCapabilityCliOperation { handle });
    }

    fn begin_report_scan(&mut self, request: SecurityReportRequest) {
        if let Some(stale) = self.report.take() {
            stale.cancellation.cancel();
            stale.handle.abort();
        }
        let cancellation = SecurityReportCancellation::default();
        let worker_cancellation = cancellation.clone();
        let worker_request = request.clone();
        let adapter = self.report_adapter.clone();
        let handle = tokio::spawn(async move {
            adapter
                .scan_with_cancellation(worker_request, worker_cancellation)
                .await
        });
        self.report = Some(SecurityReportCliOperation {
            request,
            cancellation,
            handle,
        });
    }

    async fn begin_mapper(
        &mut self,
        app: &mut App,
        id: SecuritySessionId,
        executable: PathBuf,
        arguments: Vec<String>,
    ) {
        if self.owns_mapper(id) {
            let _ = update(
                app,
                Action::Notify(
                    "The exact Security package-mapping process is already owned by the CLI."
                        .into(),
                ),
            );
            return;
        }
        if self.mapper.is_some() {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: "another Security package-mapping process is already active".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let preview = app
            .security
            .sessions
            .iter()
            .find(|session| session.preview.id == id)
            .map(|session| session.preview.clone());
        let Some(preview) = preview else {
            let _ = update(
                app,
                Action::Security(SecurityAction::LoseSession {
                    id,
                    message: "the exact Security package-mapping session is unavailable".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        };
        if !matches!(
            &preview.operation,
            SecurityOperation::PackageMap {
                executable: expected_executable,
                arguments: expected_arguments,
            } if expected_executable == &executable && expected_arguments == &arguments
        ) {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: "Security package-mapping effect does not match its preview".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let command = match SecurityMapperCommandSpec::from_preview(&preview) {
            Ok(command) => command,
            Err(error) => {
                let _ = update(
                    app,
                    Action::Security(SecurityAction::FailSession {
                        id,
                        message: error.to_string(),
                        finished_at: SystemTime::now(),
                    }),
                );
                return;
            }
        };
        let mut runner = SecurityMapperJobRunner::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::Security(SecurityAction::FailSession {
                    id,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        self.mapper = Some(SecurityMapperCliOperation { id, runner });
    }

    async fn cancel_mapper(&mut self, app: &mut App, id: SecuritySessionId) {
        let Some(active) = self.mapper.as_mut().filter(|active| active.id == id) else {
            let _ = update(
                app,
                Action::Security(SecurityAction::RejectCancellation {
                    id,
                    message: "the CLI does not own this Security package-mapping process".into(),
                }),
            );
            return;
        };
        if let Err(error) = active.runner.cancel(id).await {
            let _ = update(
                app,
                Action::Security(SecurityAction::RejectCancellation {
                    id,
                    message: error.to_string(),
                }),
            );
        }
    }

    async fn poll(&mut self, app: &mut App) {
        self.poll_capability(app).await;
        self.poll_report(app).await;
        self.poll_mapper(app).await;
    }

    async fn poll_capability(&mut self, app: &mut App) {
        if !self
            .capability
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self.capability.take().expect("finished capability checked");
        let action = match operation.handle.await {
            Ok(Ok(capability)) => SecurityAction::CapabilityLoaded(capability),
            Ok(Err(message)) => SecurityAction::CapabilityFailed(message),
            Err(error) => SecurityAction::CapabilityFailed(format!(
                "Security capability task was lost: {error}"
            )),
        };
        let _ = update(app, Action::Security(action));
    }

    async fn poll_report(&mut self, app: &mut App) {
        if !self
            .report
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self.report.take().expect("finished report scan checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => match response.outcome {
                SecurityReportScanOutcome::Empty => SecurityAction::ReportsLoaded {
                    request: response.request,
                    reports: Vec::new(),
                    limitations: Vec::new(),
                },
                SecurityReportScanOutcome::Complete(reports) => SecurityAction::ReportsLoaded {
                    request: response.request,
                    reports,
                    limitations: Vec::new(),
                },
                SecurityReportScanOutcome::Partial {
                    reports,
                    limitations,
                } => SecurityAction::ReportsLoaded {
                    request: response.request,
                    reports,
                    limitations,
                },
            },
            Ok(Err(SecurityReportAdapterError::Cancelled)) => {
                SecurityAction::ReportsCancelled(operation.request)
            }
            Ok(Err(SecurityReportAdapterError::Timeout(_))) => {
                SecurityAction::ReportsTimedOut(operation.request)
            }
            Ok(Err(SecurityReportAdapterError::WorkerLost(message))) => {
                SecurityAction::ReportsLost {
                    request: operation.request,
                    message,
                }
            }
            Ok(Err(error)) => SecurityAction::ReportsFailed {
                request: operation.request,
                message: error.to_string(),
            },
            Err(error) if error.is_cancelled() => {
                SecurityAction::ReportsCancelled(operation.request)
            }
            Err(error) => SecurityAction::ReportsLost {
                request: operation.request,
                message: error.to_string(),
            },
        };
        let _ = update(app, Action::Security(action));
    }

    async fn poll_mapper(&mut self, app: &mut App) {
        let mut followups = Vec::new();
        let Some(operation) = self.mapper.as_mut() else {
            return;
        };
        let result =
            tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await;
        let event = match result {
            Ok(Ok(event)) => Some(event),
            Ok(Err(error)) => Some(SecurityMapperRunnerEvent::Lost {
                id: operation.id,
                message: error.to_string(),
            }),
            Err(_) => None,
        };
        let Some(event) = event else {
            return;
        };
        let terminal = matches!(
            event,
            SecurityMapperRunnerEvent::Completed { .. }
                | SecurityMapperRunnerEvent::Failed { .. }
                | SecurityMapperRunnerEvent::Cancelled { .. }
                | SecurityMapperRunnerEvent::TimedOut { .. }
                | SecurityMapperRunnerEvent::Lost { .. }
        );
        for action in security_actions_for_mapper_event(event, SystemTime::now()) {
            if let Some(effect) = update(app, action) {
                followups.push(effect);
            }
        }
        if terminal {
            self.mapper = None;
        }
        for effect in followups {
            let _ = self.handle_effect(app, effect).await;
        }
    }

    async fn revalidate_open_path(&self, app: &App, path: &Path) -> Result<(), String> {
        let selected_identity = app
            .security
            .selected_report()
            .map(|report| report.identity().clone())
            .filter(|identity| identity.path == path);
        if let Some(identity) = selected_identity {
            let request =
                SecurityReportRequest::new(1, vec![path.to_path_buf()]).map_err(str::to_owned)?;
            let response = self
                .report_adapter
                .scan(request)
                .await
                .map_err(|error| error.to_string())?;
            if response
                .outcome
                .reports()
                .iter()
                .any(|report| report.identity() == &identity)
            {
                return Ok(());
            }
            return Err("the selected Security report changed before it could be opened".into());
        }
        let provider = app.security.scope.as_ref().and_then(|scope| match scope {
            SecurityScope::Recipe(identity) => Some(identity.file.as_path()),
            SecurityScope::Image { .. } => None,
        });
        if provider != Some(path) {
            return Err(
                "the requested path is not the selected Security report or provider".into(),
            );
        }
        let metadata = fs::symlink_metadata(path)
            .map_err(|error| format!("could not inspect the Security provider: {error}"))?;
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || fs::canonicalize(path).ok().as_deref() != Some(path)
        {
            return Err(
                "the selected Security provider is no longer a canonical regular file".into(),
            );
        }
        Ok(())
    }

    fn url_opener(&self) -> Option<PathBuf> {
        self.path_directories.iter().find_map(|directory| {
            let candidate = directory.join("xdg-open");
            let metadata = fs::symlink_metadata(&candidate).ok()?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || fs::canonicalize(&candidate).ok().as_ref() != Some(&candidate)
            {
                return None;
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o111 == 0 {
                    return None;
                }
            }
            Some(candidate)
        })
    }
}

fn security_capability_input(
    app: &App,
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
) -> std::result::Result<SecurityCapabilityInput, String> {
    let selected_recipe = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .and_then(|recipe| {
            recipe.file.clone().map(|file| {
                SecurityScope::Recipe(RecipeIdentity {
                    name: recipe.name.clone(),
                    file,
                })
            })
        });
    let selected_image = app.build.target.clone().and_then(|target| {
        let machine = app.workspace.variables.get("MACHINE")?.clone();
        let distro = app.workspace.variables.get("DISTRO")?.clone();
        Some(SecurityScope::Image {
            target,
            machine,
            distro,
        })
    });
    let mut available_scopes = [selected_recipe, selected_image]
        .into_iter()
        .flatten()
        .filter(SecurityScope::is_valid)
        .collect::<Vec<_>>();
    available_scopes.dedup();
    let scope = app
        .security
        .scope
        .clone()
        .filter(|scope| available_scopes.contains(scope))
        .or_else(|| available_scopes.first().cloned())
        .ok_or_else(|| {
            "Security needs an exact selected recipe provider or image target with MACHINE and DISTRO"
                .to_owned()
        })?;
    let reported_tasks = app
        .recipe_metadata
        .get(scope.target())
        .and_then(|metadata| metadata.tasks.clone())
        .unwrap_or_default();
    let explicit_paths = |names: &[&str]| {
        names
            .iter()
            .filter_map(|name| app.workspace.variables.get(*name))
            .map(PathBuf::from)
            .collect::<Vec<_>>()
    };
    let cve_roots = explicit_paths(&["CVE_CHECK_REPORT_ROOT", "CVE_CHECK_DIR", "DEPLOY_DIR_IMAGE"]);
    let sbom_roots = explicit_paths(&["DEPLOY_DIR_SPDX", "DEPLOY_DIR_SBOM", "DEPLOY_DIR_IMAGE"]);
    let image_build_emits_sbom = ["INHERIT", "IMAGE_CLASSES"]
        .iter()
        .filter_map(|name| app.workspace.variables.get(*name))
        .flat_map(|value| value.split_whitespace())
        .any(|value| {
            matches!(
                value,
                "create-spdx" | "create-spdx-2.0" | "create_sbom" | "create-sbom"
            )
        });
    Ok(SecurityCapabilityInput {
        release: app.workspace.release.clone(),
        build_directory,
        scope,
        available_scopes,
        reported_tasks,
        image_build_emits_sbom,
        cve_roots,
        sbom_roots,
        path_directories,
    })
}

async fn open_security_url(app: &mut App, opener: Option<PathBuf>, url: String) {
    if !url.starts_with("https://") || url.chars().any(char::is_control) {
        app.notification = Some("The selected Security advisory URL is invalid.".into());
        return;
    }
    let Some(opener) = opener else {
        app.notification =
            Some("Cannot open the Security advisory because xdg-open is unavailable.".into());
        return;
    };
    let result = tokio::task::spawn_blocking(move || {
        ProcessCommand::new(opener)
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    })
    .await;
    match result {
        Ok(Ok(status)) if status.success() => {}
        Ok(Ok(status)) => {
            app.notification = Some(format!("xdg-open exited with {status}."));
        }
        Ok(Err(error)) => {
            app.notification = Some(format!("Could not start xdg-open: {error}"));
        }
        Err(error) => {
            app.notification = Some(format!("Security URL opener task was lost: {error}"));
        }
    }
}

async fn route_independent_security_effect(
    guard: &TerminalGuard,
    app: &mut App,
    coordinator: &mut SecurityCliCoordinator,
    effect: Effect,
    editor: Option<&str>,
) -> bool {
    match &effect {
        Effect::Security(SecurityEffect::StartBuild { .. }) => false,
        Effect::Security(SecurityEffect::CancelSession(id)) if !coordinator.owns_mapper(*id) => {
            false
        }
        Effect::Security(SecurityEffect::OpenPath(path)) => {
            match coordinator.revalidate_open_path(app, path).await {
                Ok(()) => open_in_editor(guard, app, path.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Security(SecurityEffect::OpenUrl(url)) => {
            open_security_url(app, coordinator.url_opener(), url.clone()).await;
            true
        }
        Effect::Security(_) => coordinator.handle_effect(app, effect).await,
        _ => false,
    }
}

struct QaCapabilityCliOperation {
    handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::QaTaskCapabilityResponse, String>,
    >,
}

struct QaLayerCapabilityCliOperation {
    handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::QaLayerCapabilityResponse, String>,
    >,
}

struct QaReportCliOperation {
    request: QaReportRequest,
    cancellation: QaReportCancellation,
    handle: tokio::task::JoinHandle<
        std::result::Result<yoctui_bitbake::QaReportResponse, QaReportAdapterError>,
    >,
}

struct QaLayerCliOperation {
    id: QaLayerSessionId,
    runner: QaLayerJobRunner,
}

struct QaCliCoordinator {
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
    report_adapter: QaReportAdapter,
    capability: Option<QaCapabilityCliOperation>,
    layer_capability: Option<QaLayerCapabilityCliOperation>,
    report: Option<QaReportCliOperation>,
    layer: Option<QaLayerCliOperation>,
}

impl QaCliCoordinator {
    fn new(build_directory: PathBuf, path_directories: Vec<PathBuf>) -> Self {
        Self {
            build_directory,
            path_directories,
            report_adapter: QaReportAdapter::new(),
            capability: None,
            layer_capability: None,
            report: None,
            layer: None,
        }
    }

    fn owns_layer(&self, id: QaLayerSessionId) -> bool {
        self.layer.as_ref().is_some_and(|active| active.id == id)
    }

    async fn handle_effect(&mut self, app: &mut App, effect: Effect) -> bool {
        let Effect::Qa(effect) = effect else {
            return false;
        };
        match effect {
            QaEffect::InspectCapability { scope } => {
                self.begin_capability_inspection(app, scope);
            }
            QaEffect::InspectLayerCapability => self.begin_layer_capability_inspection(app),
            QaEffect::ImportReports(request) => self.begin_report_scan(app, request),
            QaEffect::StartLayerCheck {
                session,
                layer,
                executable,
                arguments,
            } => {
                self.begin_layer_check(app, session, layer, executable, arguments)
                    .await;
            }
            QaEffect::CancelLayerCheck(id) => self.cancel_layer_check(app, id).await,
            QaEffect::StartBuild { .. }
            | QaEffect::CancelBuild { .. }
            | QaEffect::OpenReport(_)
            | QaEffect::OpenProvider(_)
            | QaEffect::OpenSource(_)
            | QaEffect::OpenLayerRoot(_) => return false,
        }
        true
    }

    fn begin_capability_inspection(&mut self, app: &App, scope: Option<QaScope>) {
        if let Some(stale) = self.capability.take() {
            stale.handle.abort();
        }
        let input = qa_task_capability_input(app, self.build_directory.clone(), scope);
        let handle = tokio::task::spawn_blocking(move || {
            let input = input?;
            QaTaskCapabilityInspector::new(input)
                .inspect()
                .map_err(|error| error.to_string())
        });
        self.capability = Some(QaCapabilityCliOperation { handle });
    }

    fn begin_layer_capability_inspection(&mut self, app: &App) {
        if let Some(stale) = self.layer_capability.take() {
            stale.handle.abort();
        }
        let input = qa_layer_capability_input(
            app,
            self.build_directory.clone(),
            self.path_directories.clone(),
        );
        let handle = tokio::task::spawn_blocking(move || {
            let input = input?;
            QaLayerCapabilityInspector::inspect(input).map_err(|error| error.to_string())
        });
        self.layer_capability = Some(QaLayerCapabilityCliOperation { handle });
    }

    fn begin_report_scan(&mut self, app: &App, request: QaReportRequest) {
        if let Some(stale) = self.report.take() {
            stale.cancellation.cancel();
            stale.handle.abort();
        }
        let input = qa_report_scan_input(app, self.build_directory.clone(), request.clone());
        let cancellation = QaReportCancellation::default();
        let worker_cancellation = cancellation.clone();
        let adapter = self.report_adapter.clone();
        let handle = tokio::spawn(async move {
            match input {
                Ok(input) => {
                    adapter
                        .scan_with_cancellation(input, worker_cancellation)
                        .await
                }
                Err(message) => Err(QaReportAdapterError::InvalidRequest(message)),
            }
        });
        self.report = Some(QaReportCliOperation {
            request,
            cancellation,
            handle,
        });
    }

    async fn begin_layer_check(
        &mut self,
        app: &mut App,
        id: QaLayerSessionId,
        layer: QaLayerIdentity,
        executable: yoctui_model::QaExecutableIdentity,
        arguments: Vec<String>,
    ) {
        if self.owns_layer(id) {
            let _ = update(
                app,
                Action::Notify("The exact layer-QA process is already owned by the CLI.".into()),
            );
            return;
        }
        if self.layer.is_some() {
            let _ = update(
                app,
                Action::Qa(QaAction::FailLayerSession {
                    session: id,
                    exit_code: None,
                    message: "another layer-QA process is already active".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let preview = app
            .qa
            .layer_sessions
            .iter()
            .find(|session| session.id == id)
            .map(|session| session.operation.clone());
        let Some(preview) = preview else {
            let _ = update(
                app,
                Action::Qa(QaAction::LoseLayerSession {
                    session: id,
                    message: "the exact layer-QA session is unavailable".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        };
        if preview.layer != layer
            || preview.executable != executable
            || preview.arguments != arguments
        {
            let _ = update(
                app,
                Action::Qa(QaAction::FailLayerSession {
                    session: id,
                    exit_code: None,
                    message: "layer-QA effect does not match its confirmed preview".into(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        let command = match QaLayerCommandSpec::from_preview(id, &preview) {
            Ok(command) => command,
            Err(error) => {
                let _ = update(
                    app,
                    Action::Qa(QaAction::FailLayerSession {
                        session: id,
                        exit_code: None,
                        message: error.to_string(),
                        finished_at: SystemTime::now(),
                    }),
                );
                return;
            }
        };
        let mut runner = QaLayerJobRunner::new();
        if let Err(error) = runner.start(command).await {
            let _ = update(
                app,
                Action::Qa(QaAction::FailLayerSession {
                    session: id,
                    exit_code: None,
                    message: error.to_string(),
                    finished_at: SystemTime::now(),
                }),
            );
            return;
        }
        self.layer = Some(QaLayerCliOperation { id, runner });
    }

    async fn cancel_layer_check(&mut self, app: &mut App, id: QaLayerSessionId) {
        let Some(active) = self.layer.as_mut().filter(|active| active.id == id) else {
            let _ = update(
                app,
                Action::Qa(QaAction::RejectLayerCancellation {
                    session: id,
                    message: "the CLI does not own this layer-QA process".into(),
                }),
            );
            return;
        };
        if let Err(error) = active.runner.cancel(id).await {
            let _ = update(
                app,
                Action::Qa(QaAction::RejectLayerCancellation {
                    session: id,
                    message: error.to_string(),
                }),
            );
        }
    }

    async fn poll(&mut self, app: &mut App) {
        self.poll_capability(app).await;
        self.poll_layer_capability(app).await;
        self.poll_report(app).await;
        self.poll_layer(app).await;
    }

    async fn poll_capability(&mut self, app: &mut App) {
        if !self
            .capability
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self
            .capability
            .take()
            .expect("finished QA capability checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => qa_task_capability_action(response),
            Ok(Err(message)) => Action::Qa(QaAction::CapabilityFailed(message)),
            Err(error) => Action::Qa(QaAction::CapabilityFailed(format!(
                "QA capability task was lost: {error}"
            ))),
        };
        let _ = update(app, action);
    }

    async fn poll_layer_capability(&mut self, app: &mut App) {
        if !self
            .layer_capability
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self
            .layer_capability
            .take()
            .expect("finished layer-QA capability checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => qa_layer_capability_action(response),
            Ok(Err(message)) => Action::Qa(QaAction::LayerCapabilityFailed(message)),
            Err(error) => Action::Qa(QaAction::LayerCapabilityFailed(format!(
                "layer-QA capability task was lost: {error}"
            ))),
        };
        let _ = update(app, action);
    }

    async fn poll_report(&mut self, app: &mut App) {
        if !self
            .report
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return;
        }
        let operation = self.report.take().expect("finished QA report scan checked");
        let action = match operation.handle.await {
            Ok(Ok(response)) => qa_report_response_action(response),
            Ok(Err(error)) => qa_report_error_action(operation.request, error),
            Err(error) if error.is_cancelled() => {
                qa_report_error_action(operation.request, QaReportAdapterError::Cancelled)
            }
            Err(error) => qa_report_error_action(
                operation.request,
                QaReportAdapterError::WorkerLost(error.to_string()),
            ),
        };
        let _ = update(app, action);
    }

    async fn poll_layer(&mut self, app: &mut App) {
        let Some(operation) = self.layer.as_mut() else {
            return;
        };
        let result =
            tokio::time::timeout(Duration::from_millis(1), operation.runner.next_event()).await;
        let event = match result {
            Ok(Ok(event)) => Some(event),
            Ok(Err(error)) => Some(QaLayerRunnerEvent::Lost {
                id: operation.id,
                message: error.to_string(),
            }),
            Err(_) => None,
        };
        let Some(event) = event else {
            return;
        };
        let terminal = matches!(
            event,
            QaLayerRunnerEvent::Completed { .. }
                | QaLayerRunnerEvent::Failed { .. }
                | QaLayerRunnerEvent::Cancelled { .. }
                | QaLayerRunnerEvent::TimedOut { .. }
                | QaLayerRunnerEvent::Lost { .. }
        );
        let action = match event {
            QaLayerRunnerEvent::Completed {
                id,
                exit_code: Some(exit_code),
            } => {
                let result_paths = app
                    .qa
                    .layer_sessions
                    .iter()
                    .find(|session| session.id == id)
                    .map(|session| session.operation.report_roots.clone())
                    .unwrap_or_default();
                Some(Action::Qa(QaAction::CompleteLayerSession {
                    session: id,
                    exit_code,
                    result_paths,
                    finished_at: SystemTime::now(),
                }))
            }
            event => qa_layer_runner_action(event, SystemTime::now()),
        };
        let followup = action.and_then(|action| update(app, action));
        if terminal {
            self.layer = None;
        }
        if let Some(effect) = followup {
            let _ = self.handle_effect(app, effect).await;
        }
    }

    fn revalidate_report(&self, app: &App, identity: &QaReportIdentity) -> Result<(), String> {
        let retained = app
            .qa
            .inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .any(|report| &report.identity == identity);
        if !retained {
            return Err("the exact QA report is no longer retained".into());
        }
        self.report_adapter
            .revalidate(identity)
            .map_err(|error| error.to_string())
    }

    fn revalidate_provider(&self, app: &App, identity: &RecipeIdentity) -> Result<(), String> {
        let retained = app.qa.capability.snapshot().is_some_and(|snapshot| {
            snapshot
                .scopes
                .iter()
                .any(|scope| &scope.recipe == identity)
        });
        if !retained {
            return Err("the exact QA provider scope is no longer retained".into());
        }
        revalidate_canonical_regular_file(&identity.file, "QA provider")
    }

    fn revalidate_source(&self, app: &App, source: &QaSourceLocation) -> Result<(), String> {
        let retained = app
            .qa
            .inventory
            .reports()
            .unwrap_or_default()
            .iter()
            .flat_map(|report| report.findings.iter())
            .any(|finding| finding.source.as_ref() == Some(source));
        if !retained {
            return Err("the exact QA finding source is no longer retained".into());
        }
        revalidate_canonical_regular_file(&source.path, "QA finding source")
    }

    fn revalidate_layer(&self, app: &App, layer: &QaLayerIdentity) -> Result<(), String> {
        let retained = app.qa.layer_capability.snapshot().is_some_and(|snapshot| {
            snapshot
                .layers
                .iter()
                .any(|candidate| &candidate.identity == layer)
        });
        if !retained {
            return Err("the exact configured layer is no longer retained".into());
        }
        revalidate_canonical_directory(&layer.root, "configured QA layer")
    }
}

fn qa_task_capability_input(
    app: &App,
    build_directory: PathBuf,
    requested_scope: Option<QaScope>,
) -> std::result::Result<QaTaskCapabilityInput, String> {
    let mut scopes = app
        .workspace
        .recipes
        .iter()
        .filter_map(|recipe| {
            let file = recipe.file.clone()?;
            let identity = RecipeIdentity {
                name: recipe.name.clone(),
                file,
            };
            let reported_tasks = app
                .recipe_metadata
                .get(&recipe.name)
                .and_then(|metadata| metadata.tasks.clone())
                .unwrap_or_default();
            let family_tasks = qa_family_task_bindings(&reported_tasks);
            let is_kernel = family_tasks
                .iter()
                .any(|binding| binding.family == QaCheckFamily::KernelConfiguration);
            Some(QaTaskScopeInput {
                identity,
                reported_tasks,
                family_tasks,
                is_kernel,
                report_roots: qa_task_report_roots(app),
            })
        })
        .collect::<Vec<_>>();
    scopes.sort_by(|left, right| {
        left.identity
            .name
            .cmp(&right.identity.name)
            .then_with(|| left.identity.file.cmp(&right.identity.file))
    });
    scopes.dedup_by(|left, right| left.identity == right.identity);
    let selected = requested_scope
        .map(|scope| scope.recipe)
        .filter(|identity| scopes.iter().any(|scope| scope.identity == *identity))
        .or_else(|| {
            app.qa
                .scope
                .as_ref()
                .map(|scope| scope.recipe.clone())
                .filter(|identity| scopes.iter().any(|scope| scope.identity == *identity))
        })
        .or_else(|| {
            app.workspace
                .recipes
                .get(app.recipe_selection)
                .and_then(|recipe| {
                    recipe.file.clone().map(|file| RecipeIdentity {
                        name: recipe.name.clone(),
                        file,
                    })
                })
                .filter(|identity| scopes.iter().any(|scope| scope.identity == *identity))
        })
        .or_else(|| scopes.first().map(|scope| scope.identity.clone()))
        .ok_or_else(|| "QA needs at least one exact recipe/provider identity".to_owned())?;
    Ok(QaTaskCapabilityInput {
        release: app.workspace.release.clone(),
        build_directory,
        selected,
        scopes,
    })
}

fn qa_family_task_bindings(reported_tasks: &[String]) -> Vec<QaFamilyTaskBinding> {
    [
        (
            QaCheckFamily::KernelConfiguration,
            ["do_kernel_configcheck"].as_slice(),
        ),
        (QaCheckFamily::UriFetch, ["do_checkuri"].as_slice()),
        (QaCheckFamily::Patch, ["do_patch_qa"].as_slice()),
        (QaCheckFamily::License, ["do_populate_lic"].as_slice()),
        (QaCheckFamily::RecipePackage, ["do_package_qa"].as_slice()),
    ]
    .into_iter()
    .flat_map(|(family, candidates)| {
        candidates
            .iter()
            .filter(|candidate| reported_tasks.iter().any(|task| task == **candidate))
            .map(move |task| QaFamilyTaskBinding {
                family,
                task: (*task).into(),
            })
    })
    .collect()
}

fn qa_task_report_roots(app: &App) -> Vec<QaReportRootInput> {
    [
        (
            QaCheckFamily::KernelConfiguration,
            "KERNEL_CONFIGCHECK_REPORT_ROOT",
        ),
        (QaCheckFamily::UriFetch, "URI_QA_REPORT_ROOT"),
        (QaCheckFamily::Patch, "PATCH_QA_REPORT_ROOT"),
        (QaCheckFamily::License, "LICENSE_QA_REPORT_ROOT"),
        (QaCheckFamily::RecipePackage, "PACKAGE_QA_REPORT_ROOT"),
    ]
    .into_iter()
    .filter_map(|(family, name)| {
        app.workspace
            .variables
            .get(name)
            .map(|value| QaReportRootInput {
                family,
                path: PathBuf::from(value),
            })
    })
    .collect()
}

fn qa_layer_capability_input(
    app: &App,
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
) -> std::result::Result<QaLayerCapabilityInput, String> {
    let report_roots = ["YOCTO_CHECK_LAYER_REPORT_ROOT", "LAYER_QA_REPORT_ROOT"]
        .into_iter()
        .filter_map(|name| app.workspace.variables.get(name).map(PathBuf::from))
        .collect::<Vec<_>>();
    let layers = app
        .workspace
        .layers
        .iter()
        .map(|layer| {
            let identity = QaLayerIdentity::new(layer.name.clone(), layer.path.clone())
                .map_err(str::to_owned)?;
            let compatible_series = app
                .workspace
                .variables
                .get(&format!(
                    "LAYERSERIES_COMPAT_{}",
                    layer.name.replace('-', "_")
                ))
                .map(|value| value.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_default();
            Ok(QaConfiguredLayerInput {
                check: QaCheckId::new("layer-qa".into()).expect("static QA check ID is valid"),
                identity,
                compatible_series,
                report_roots: report_roots.clone(),
            })
        })
        .collect::<std::result::Result<Vec<_>, String>>()?;
    let selected_layer = app
        .qa
        .layer_selection
        .clone()
        .filter(|identity| layers.iter().any(|layer| layer.identity == *identity))
        .or_else(|| layers.first().map(|layer| layer.identity.clone()))
        .ok_or_else(|| "layer QA needs at least one exact configured layer".to_owned())?;
    Ok(QaLayerCapabilityInput {
        release: app.workspace.release.clone(),
        build_directory,
        selected_layer,
        layers,
        executable_search_path: path_directories,
    })
}

fn qa_report_scan_input(
    app: &App,
    build_directory: PathBuf,
    request: QaReportRequest,
) -> std::result::Result<QaReportScanInput, String> {
    let known_checks = app
        .qa
        .capability
        .snapshot()
        .into_iter()
        .flat_map(|snapshot| snapshot.checks.iter().map(|check| check.id.clone()))
        .chain(
            app.qa
                .layer_capability
                .snapshot()
                .into_iter()
                .flat_map(|snapshot| snapshot.layers.iter().map(|layer| layer.check.clone())),
        )
        .collect::<Vec<_>>();
    let known_scopes = app
        .qa
        .capability
        .snapshot()
        .into_iter()
        .flat_map(|snapshot| snapshot.scopes.iter().cloned().map(QaFindingScope::Recipe))
        .chain(
            app.qa
                .layer_capability
                .snapshot()
                .into_iter()
                .flat_map(|snapshot| {
                    snapshot
                        .layers
                        .iter()
                        .map(|layer| QaFindingScope::Layer(layer.identity.clone()))
                }),
        )
        .collect::<Vec<_>>();
    let candidates = request
        .paths
        .iter()
        .map(|path| qa_report_candidate(app, path))
        .collect::<std::result::Result<Vec<_>, String>>()?;
    Ok(QaReportScanInput {
        build_directory,
        request,
        candidates,
        known_checks,
        known_scopes,
    })
}

fn qa_report_candidate(app: &App, path: &Path) -> std::result::Result<QaReportCandidate, String> {
    let recipe_session = app
        .qa
        .sessions
        .iter()
        .rev()
        .find(|session| {
            session
                .result_paths
                .iter()
                .any(|candidate| candidate == path)
        })
        .map(|session| {
            (
                QaReportOrigin::Managed,
                session.operation.check.clone(),
                QaFindingScope::Recipe(session.operation.scope.clone()),
                session.operation.request.task.clone(),
                None,
            )
        });
    let layer_session = app
        .qa
        .layer_sessions
        .iter()
        .rev()
        .find(|session| {
            session
                .result_paths
                .iter()
                .any(|candidate| candidate == path)
        })
        .map(|session| {
            (
                QaReportOrigin::Managed,
                session.operation.check.clone(),
                QaFindingScope::Layer(session.operation.layer.clone()),
                None,
                Some("yocto-check-layer".into()),
            )
        });
    let retained_report = app
        .qa
        .inventory
        .reports()
        .unwrap_or_default()
        .iter()
        .find(|report| report.identity.path == path)
        .and_then(|report| {
            let producer = report.identity.producer.clone()?;
            let scope = report.identity.scope.clone()?;
            let (task, test_name) = match &scope {
                QaFindingScope::Recipe(recipe_scope) => {
                    let task = app.qa.capability.snapshot().and_then(|snapshot| {
                        snapshot
                            .checks
                            .iter()
                            .find(|check| check.id == producer && check.scope == *recipe_scope)
                            .and_then(|check| check.task.clone())
                    });
                    (task, None)
                }
                QaFindingScope::Layer(_) => (None, Some("yocto-check-layer".into())),
            };
            Some((QaReportOrigin::Import, producer, scope, task, test_name))
        });
    let selected_recipe = app.qa.selected_check().map(|check| {
        (
            QaReportOrigin::Import,
            check.id.clone(),
            QaFindingScope::Recipe(check.scope.clone()),
            check.task.clone(),
            None,
        )
    });
    let selected_layer = app.qa.selected_layer().map(|layer| {
        (
            QaReportOrigin::Import,
            layer.check.clone(),
            QaFindingScope::Layer(layer.identity.clone()),
            None,
            Some("yocto-check-layer".into()),
        )
    });
    let (origin, producer, scope, task, test_name) = recipe_session
        .or(layer_session)
        .or(retained_report)
        .or_else(|| match app.qa.view {
            yoctui_model::QaView::RecipeKernel => selected_recipe.or(selected_layer),
            yoctui_model::QaView::LayerQa => selected_layer.or(selected_recipe),
        })
        .ok_or_else(|| {
            "QA report import needs an exact check or configured-layer scope".to_owned()
        })?;
    let format = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => None,
        _ => qa_report_format(path),
    };
    Ok(QaReportCandidate {
        path: path.to_path_buf(),
        origin,
        format,
        producer,
        scope,
        task,
        test_name,
    })
}

fn qa_report_format(path: &Path) -> Option<QaReportFormat> {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json" | "jsonl") => Some(QaReportFormat::Json),
        Some("xml") => Some(QaReportFormat::Xml),
        Some("qa" | "txt") => Some(QaReportFormat::Text),
        Some("log") => Some(QaReportFormat::BitBakeLog),
        _ => None,
    }
}

fn revalidate_canonical_regular_file(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || fs::canonicalize(path).ok().as_deref() != Some(path)
    {
        return Err(format!("{label} is no longer a canonical regular file"));
    }
    Ok(())
}

fn revalidate_canonical_directory(path: &Path, label: &str) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| format!("could not inspect {label}: {error}"))?;
    if metadata.file_type().is_symlink()
        || !metadata.is_dir()
        || fs::canonicalize(path).ok().as_deref() != Some(path)
    {
        return Err(format!("{label} is no longer a canonical directory"));
    }
    Ok(())
}

async fn route_independent_qa_effect(
    guard: &TerminalGuard,
    app: &mut App,
    coordinator: &mut QaCliCoordinator,
    effect: Effect,
    editor: Option<&str>,
) -> bool {
    match &effect {
        Effect::Qa(QaEffect::StartBuild { .. } | QaEffect::CancelBuild { .. }) => false,
        Effect::Qa(QaEffect::OpenReport(identity)) => {
            match coordinator.revalidate_report(app, identity) {
                Ok(()) => {
                    open_in_editor(guard, app, identity.path.clone(), editor).await;
                }
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(QaEffect::OpenProvider(identity)) => {
            match coordinator.revalidate_provider(app, identity) {
                Ok(()) => open_in_editor(guard, app, identity.file.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(QaEffect::OpenSource(source)) => {
            match coordinator.revalidate_source(app, source) {
                Ok(()) => open_in_editor(guard, app, source.path.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(QaEffect::OpenLayerRoot(layer)) => {
            match coordinator.revalidate_layer(app, layer) {
                Ok(()) => open_in_editor(guard, app, layer.root.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Qa(_) => coordinator.handle_effect(app, effect).await,
        _ => false,
    }
}

fn initialized_path_directories() -> Vec<PathBuf> {
    env::var_os("PATH")
        .map(|path| env::split_paths(&path).collect())
        .unwrap_or_default()
}

async fn route_independent_maintenance_effect(
    guard: &TerminalGuard,
    app: &mut App,
    coordinator: &mut MaintenanceCliCoordinator,
    effect: Effect,
    editor: Option<&str>,
) -> bool {
    match effect {
        Effect::Maintenance(yoctui_model::MaintenanceEffect::OpenEvidence(identity)) => {
            match coordinator.revalidate_evidence(&identity) {
                Ok(path) => open_in_editor(guard, app, path, editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Maintenance(yoctui_model::MaintenanceEffect::Navigate(screen)) => {
            if let Some(next) = update(app, Action::Open(screen)) {
                let _ = coordinator.handle_effect(app, next).await;
            }
            true
        }
        Effect::Maintenance(_) => coordinator.handle_effect(app, effect).await,
        _ => false,
    }
}

fn maintenance_row_count(app: &App) -> usize {
    match app.maintenance.view {
        yoctui_model::MaintenanceView::Sstate => 2,
        yoctui_model::MaintenanceView::Services => 1,
        yoctui_model::MaintenanceView::Release => 4,
        yoctui_model::MaintenanceView::Integrations => 4,
    }
}

fn ptest_capability(app: &App) -> yoctui_model::PtestCapability {
    let suites = app.workspace.variables.get("TEST_SUITES");
    let features = app
        .workspace
        .variables
        .get("EXTRA_IMAGE_FEATURES")
        .or_else(|| app.workspace.variables.get("IMAGE_FEATURES"));
    match (suites, features) {
        (Some(suites), Some(features))
            if suites.split_whitespace().any(|value| value == "ptest")
                && features
                    .split_whitespace()
                    .any(|value| value == "ptest-pkgs") =>
        {
            yoctui_model::PtestCapability::Configured
        }
        (Some(_), Some(_)) => yoctui_model::PtestCapability::Unavailable(
            "active TEST_SUITES/IMAGE_FEATURES do not confirm ptest".into(),
        ),
        _ => yoctui_model::PtestCapability::Unavailable(
            "active TEST_SUITES and image features are unavailable".into(),
        ),
    }
}

fn testing_screen_action(app: &App, input: Input) -> Option<Action> {
    match app.test_view {
        TestWorkspaceView::Launches => testing_workspace_action(input),
        TestWorkspaceView::Results => {
            test_results_workspace_action(app.test_result_searching, app.test_result_drilled, input)
        }
        TestWorkspaceView::Comparison => test_comparison_workspace_action(input),
    }
}

fn sdk_build_is_populate(request: &BuildRequest) -> bool {
    matches!(
        request.task.as_deref(),
        Some("populate_sdk" | "populate_sdk_ext")
    )
}

fn sdk_refresh_after_build_event(
    app: &mut App,
    pending_sdk_build: &mut Option<BuildRequest>,
    event: &BackendEvent,
) -> Option<Effect> {
    match event {
        BackendEvent::BuildCompleted { success: true, .. } => {
            let request = pending_sdk_build.take()?;
            sdk_build_is_populate(&request)
                .then(|| update(app, Action::RefreshSdkArtifactInventory))
                .flatten()
        }
        BackendEvent::BuildCompleted { success: false, .. }
        | BackendEvent::CommandFailed { .. }
        | BackendEvent::Disconnected => {
            *pending_sdk_build = None;
            None
        }
        _ => None,
    }
}

struct QemuCliOperation {
    id: QemuSessionId,
    runner: Option<QemuJobRunner>,
    cancellation: Option<tokio::task::JoinHandle<(QemuJobRunner, Result<bool, QemuAdapterError>)>>,
}

fn qemu_preview_for_request(
    capability: &QemuCapability,
    request: &QemuLaunchRequest,
) -> Result<QemuLaunchPreview, String> {
    let kernel = request
        .kernel
        .as_ref()
        .map(|path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| "runqemu kernel path is not valid UTF-8".to_owned())
        })
        .transpose()?
        .unwrap_or_default();
    let rootfs = request
        .rootfs
        .as_ref()
        .map(|path| {
            path.to_str()
                .map(str::to_owned)
                .ok_or_else(|| "runqemu rootfs path is not valid UTF-8".to_owned())
        })
        .transpose()?
        .unwrap_or_default();
    let draft = QemuLaunchDraft {
        machine: request.machine.clone(),
        image: request.image.clone(),
        artifact_kind: request.artifact_kind,
        kernel,
        rootfs,
        networking: request.networking,
        display: request.display,
        serial: request.serial,
        memory_mib: request.memory_mib.to_string(),
        extra_arguments: request.extra_arguments.join(" "),
    };
    let preview = draft.preview(capability).map_err(str::to_owned)?;
    if &preview.request != request {
        return Err("runqemu request changed while rebuilding its preview".into());
    }
    Ok(preview)
}

async fn begin_qemu_job(
    app: &mut App,
    operation: &mut Option<QemuCliOperation>,
    build_dir: &Path,
    cancellation_timeout: Duration,
    id: QemuSessionId,
    request: QemuLaunchRequest,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::FailQemuSession {
                id,
                message: "another managed runqemu process is already owned by the CLI".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    }
    let start = qemu_preview_for_request(&app.qemu_capability, &request)
        .map_err(QemuAdapterError::InvalidRequest)
        .and_then(|preview| QemuCommandSpec::from_preview(&preview));
    let command = match start {
        Ok(command) => command,
        Err(error) => {
            let _ = update(
                app,
                Action::FailQemuSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
            return;
        }
    };
    let mut runner =
        QemuJobRunner::new(build_dir.to_path_buf()).with_cancellation_timeout(cancellation_timeout);
    match runner.start(command).await {
        Ok(()) => {
            *operation = Some(QemuCliOperation {
                id,
                runner: Some(runner),
                cancellation: None,
            });
        }
        Err(error) => {
            let _ = update(
                app,
                Action::FailQemuSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
        }
    }
}

fn begin_qemu_cancellation(
    app: &mut App,
    operation: &mut Option<QemuCliOperation>,
    id: QemuSessionId,
) {
    let Some(active) = operation.as_mut().filter(|active| active.id == id) else {
        let _ = update(
            app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "the CLI does not own this runqemu process".into(),
            },
        );
        return;
    };
    if active.cancellation.is_some() {
        let _ = update(
            app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "runqemu cancellation is already in progress".into(),
            },
        );
        return;
    }
    let Some(mut runner) = active.runner.take() else {
        let _ = update(
            app,
            Action::RejectQemuSessionCancellation {
                id,
                message: "the runqemu runner is unavailable".into(),
            },
        );
        return;
    };
    active.cancellation = Some(tokio::spawn(async move {
        let result = runner.cancel().await;
        (runner, result)
    }));
}

async fn poll_qemu_job(app: &mut App, operation: &mut Option<QemuCliOperation>) {
    let Some(active) = operation.as_mut() else {
        return;
    };
    if active
        .cancellation
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.cancellation.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(_))) => active.runner = Some(runner),
            Ok((runner, Err(error))) => {
                active.runner = Some(runner);
                let _ = update(
                    app,
                    Action::RejectQemuSessionCancellation {
                        id: active.id,
                        message: error.to_string(),
                    },
                );
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseQemuSession {
                        id,
                        message: format!("runqemu cancellation task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
        }
    }
    if active.cancellation.is_some() {
        return;
    }
    let Some(runner) = active.runner.as_mut() else {
        return;
    };
    match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await {
        Ok(Ok(event)) => {
            let terminal = matches!(
                event,
                QemuRunnerEvent::Completed { .. }
                    | QemuRunnerEvent::Failed { .. }
                    | QemuRunnerEvent::Cancelled { .. }
                    | QemuRunnerEvent::Lost { .. }
            );
            for action in qemu_actions_for_runner_event(active.id, event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *operation = None;
            }
        }
        Ok(Err(error)) => {
            let id = active.id;
            for action in qemu_actions_for_runner_event(
                id,
                QemuRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *operation = None;
        }
        Err(_) => {}
    }
}

struct WicCliOperation {
    id: WicSessionId,
    starting: Option<tokio::task::JoinHandle<(WicJobRunner, Result<(), WicAdapterError>)>>,
    runner: Option<WicJobRunner>,
    cancellation: Option<tokio::task::JoinHandle<(WicJobRunner, Result<bool, WicAdapterError>)>>,
}

fn wic_preview_for_request(
    capability: &WicCapability,
    request: &WicCreateRequest,
) -> Result<WicCreatePreview, String> {
    let output_directory = request
        .output_directory
        .to_str()
        .ok_or_else(|| "Wic output directory is not valid UTF-8".to_owned())?;
    let draft = WicCreateDraft {
        machine: request.machine.clone(),
        image: request.image.clone(),
        kickstart: request.kickstart.clone(),
        output_directory: output_directory.into(),
        generate_bmap: request.generate_bmap,
        compression: request.compression,
    };
    let preview = draft.preview(capability).map_err(str::to_owned)?;
    if &preview.request != request {
        return Err("Wic request changed while rebuilding its preview".into());
    }
    Ok(preview)
}

async fn begin_wic_job(
    app: &mut App,
    operation: &mut Option<WicCliOperation>,
    device_inspector: &WicDeviceInspector,
    build_dir: &Path,
    cancellation_timeout: Duration,
    id: WicSessionId,
    requested_operation: WicOperation,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::FailWicSession {
                id,
                message: "another managed Wic process is already owned by the CLI".into(),
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        return;
    }
    let mut runner =
        WicJobRunner::new(build_dir.to_path_buf()).with_cancellation_timeout(cancellation_timeout);
    let start = match requested_operation {
        WicOperation::Create(request) => {
            let output_directory = request.output_directory.clone();
            match wic_preview_for_request(&app.wic_capability, &request)
                .map_err(WicAdapterError::InvalidRequest)
                .and_then(|preview| {
                    WicCreateCommandSpec::from_preview(&preview, &app.wic_capability)
                }) {
                Ok(command) => runner.start(command, output_directory).await,
                Err(error) => Err(error),
            }
        }
        WicOperation::Write(request) => {
            let inspector = device_inspector.clone();
            let starting = tokio::spawn(async move {
                let result = runner.start_write(&inspector, request).await;
                (runner, result)
            });
            *operation = Some(WicCliOperation {
                id,
                starting: Some(starting),
                runner: None,
                cancellation: None,
            });
            return;
        }
    };
    match start {
        Ok(()) => {
            *operation = Some(WicCliOperation {
                id,
                starting: None,
                runner: Some(runner),
                cancellation: None,
            });
        }
        Err(error) => {
            let _ = update(
                app,
                Action::FailWicSession {
                    id,
                    message: error.to_string(),
                    exit_code: None,
                    finished_at: SystemTime::now(),
                },
            );
        }
    }
}

fn begin_wic_cancellation(
    app: &mut App,
    operation: &mut Option<WicCliOperation>,
    id: WicSessionId,
) {
    let Some(active) = operation.as_mut().filter(|active| active.id == id) else {
        let _ = update(
            app,
            Action::RejectWicSessionCancellation {
                id,
                message: "the CLI does not own this Wic process".into(),
            },
        );
        return;
    };
    if active.cancellation.is_some() {
        let _ = update(
            app,
            Action::RejectWicSessionCancellation {
                id,
                message: "Wic cancellation is already in progress".into(),
            },
        );
        return;
    }
    if let Some(starting) = active.starting.take() {
        starting.abort();
        let _ = update(
            app,
            Action::CancelWicSession {
                id,
                exit_code: None,
                finished_at: SystemTime::now(),
            },
        );
        *operation = None;
        return;
    }
    let Some(mut runner) = active.runner.take() else {
        let _ = update(
            app,
            Action::RejectWicSessionCancellation {
                id,
                message: "the Wic runner is unavailable".into(),
            },
        );
        return;
    };
    active.cancellation = Some(tokio::spawn(async move {
        let result = runner.cancel().await;
        (runner, result)
    }));
}

async fn poll_wic_job(app: &mut App, operation: &mut Option<WicCliOperation>) {
    let Some(active) = operation.as_mut() else {
        return;
    };
    if active
        .starting
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.starting.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(()))) => active.runner = Some(runner),
            Ok((_, Err(error))) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::FailWicSession {
                        id,
                        message: error.to_string(),
                        exit_code: None,
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseWicSession {
                        id,
                        message: format!("Wic startup task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
        }
    }
    if active.starting.is_some() {
        return;
    }
    if active
        .cancellation
        .as_ref()
        .is_some_and(tokio::task::JoinHandle::is_finished)
    {
        let handle = active.cancellation.take().expect("checked above");
        match handle.await {
            Ok((runner, Ok(_))) => active.runner = Some(runner),
            Ok((runner, Err(error))) => {
                active.runner = Some(runner);
                let _ = update(
                    app,
                    Action::RejectWicSessionCancellation {
                        id: active.id,
                        message: error.to_string(),
                    },
                );
            }
            Err(error) => {
                let id = active.id;
                let _ = update(
                    app,
                    Action::LoseWicSession {
                        id,
                        message: format!("Wic cancellation task was lost: {error}"),
                        finished_at: SystemTime::now(),
                    },
                );
                *operation = None;
                return;
            }
        }
    }
    if active.cancellation.is_some() {
        return;
    }
    let Some(runner) = active.runner.as_mut() else {
        return;
    };
    match tokio::time::timeout(Duration::from_millis(1), runner.next_event()).await {
        Ok(Ok(event)) => {
            let terminal = matches!(
                event,
                WicRunnerEvent::Completed { .. }
                    | WicRunnerEvent::Failed { .. }
                    | WicRunnerEvent::Cancelled { .. }
                    | WicRunnerEvent::Lost { .. }
            );
            for action in wic_actions_for_runner_event(active.id, event, SystemTime::now()) {
                let _ = update(app, action);
            }
            if terminal {
                *operation = None;
            }
        }
        Ok(Err(error)) => {
            let id = active.id;
            for action in wic_actions_for_runner_event(
                id,
                WicRunnerEvent::Lost {
                    message: error.to_string(),
                },
                SystemTime::now(),
            ) {
                let _ = update(app, action);
            }
            *operation = None;
        }
        Err(_) => {}
    }
}

#[derive(Debug, Clone)]
enum SignatureOperationRequest {
    Dump(SignatureTarget),
    Compare(SignatureComparisonRequest),
}

struct SignatureBackgroundOperation {
    request: SignatureOperationRequest,
    cancellation: SignatureCancellation,
    handle: tokio::task::JoinHandle<BackendEvent>,
}

fn begin_signature_operation(
    app: &mut App,
    adapter: &SignatureAdapter,
    operation: &mut Option<SignatureBackgroundOperation>,
    effect: Effect,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::Notify("A signature operation is already running.".into()),
        );
        return;
    }
    let cancellation = SignatureCancellation::default();
    let worker_cancellation = cancellation.clone();
    let adapter = adapter.clone();
    let (request, handle) = match effect {
        Effect::GetSignatureDump(target) => {
            let worker_target = target.clone();
            let handle = tokio::spawn(async move {
                match adapter
                    .dump_with_cancellation(worker_target.clone(), worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::SignatureDumpFailed {
                        target: worker_target,
                        message: error.to_string(),
                    },
                }
            });
            (SignatureOperationRequest::Dump(target), handle)
        }
        Effect::CompareSignatures(request) => {
            let worker_request = request.clone();
            let handle = tokio::spawn(async move {
                match adapter
                    .compare_with_cancellation(worker_request.clone(), worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::SignatureComparisonFailed {
                        request: worker_request,
                        message: error.to_string(),
                    },
                }
            });
            (SignatureOperationRequest::Compare(request), handle)
        }
        _ => return,
    };
    *operation = Some(SignatureBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

async fn poll_signature_operation(
    app: &mut App,
    operation: &mut Option<SignatureBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let event = match operation.handle.await {
        Ok(event) => event,
        Err(error) => match operation.request {
            SignatureOperationRequest::Dump(target) => BackendEvent::SignatureDumpFailed {
                target,
                message: format!("signature background task was lost: {error}"),
            },
            SignatureOperationRequest::Compare(request) => {
                BackendEvent::SignatureComparisonFailed {
                    request,
                    message: format!("signature background task was lost: {error}"),
                }
            }
        },
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
}

#[derive(Debug, Clone)]
enum PackageOperationRequest {
    Inventory(PackageInventoryRequest),
    Detail(PackageDetailRequest),
}

struct PackageBackgroundOperation {
    request: PackageOperationRequest,
    cancellation: PackageDataCancellation,
    handle: tokio::task::JoinHandle<BackendEvent>,
}

fn begin_package_operation(
    app: &mut App,
    adapter: &PackageDataAdapter,
    operation: &mut Option<PackageBackgroundOperation>,
    effect: Effect,
) {
    if operation.is_some() {
        let _ = update(
            app,
            Action::Notify("A package-data operation is already running.".into()),
        );
        return;
    }
    let cancellation = PackageDataCancellation::default();
    let worker_cancellation = cancellation.clone();
    let adapter = adapter.clone();
    let (request, handle) = match effect {
        Effect::GetPackageInventory(request) => {
            let worker_request = request;
            let handle = tokio::spawn(async move {
                match adapter
                    .inventory_with_cancellation(worker_request, worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::PackageInventoryFailed {
                        request: worker_request,
                        message: error.to_string(),
                    },
                }
            });
            (PackageOperationRequest::Inventory(request), handle)
        }
        Effect::GetPackageDetail(request) => {
            let worker_request = request.clone();
            let handle = tokio::spawn(async move {
                match adapter
                    .detail_with_cancellation(worker_request.clone(), worker_cancellation)
                    .await
                {
                    Ok(response) => response.into(),
                    Err(error) => BackendEvent::PackageDetailFailed {
                        request: worker_request,
                        message: error.to_string(),
                    },
                }
            });
            (PackageOperationRequest::Detail(request), handle)
        }
        _ => return,
    };
    *operation = Some(PackageBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

async fn poll_package_operation(app: &mut App, operation: &mut Option<PackageBackgroundOperation>) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let event = match operation.handle.await {
        Ok(event) => event,
        Err(error) => match operation.request {
            PackageOperationRequest::Inventory(request) => BackendEvent::PackageInventoryFailed {
                request,
                message: format!("package-data background task was lost: {error}"),
            },
            PackageOperationRequest::Detail(request) => BackendEvent::PackageDetailFailed {
                request,
                message: format!("package-data background task was lost: {error}"),
            },
        },
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
}

struct ImageArtifactBackgroundOperation {
    request: ImageArtifactRequest,
    cancellation: ImageArtifactCancellation,
    handle: tokio::task::JoinHandle<BackendEvent>,
}

struct WicCapabilityBackgroundOperation {
    image_generation: u64,
    handle: tokio::task::JoinHandle<WicCapability>,
}

struct WicDeviceBackgroundOperation {
    request: WicDeviceInventoryRequest,
    handle: tokio::task::JoinHandle<Result<WicDeviceInventoryResponse, WicAdapterError>>,
}

fn begin_wic_device_operation(
    inspector: &WicDeviceInspector,
    operation: &mut Option<WicDeviceBackgroundOperation>,
    effect: Effect,
) {
    let Effect::GetWicDevices(request) = effect else {
        return;
    };
    if let Some(stale) = operation.take() {
        stale.handle.abort();
    }
    let worker_request = request.clone();
    let inspector = inspector.clone();
    let handle = tokio::spawn(async move { inspector.discover(worker_request).await });
    *operation = Some(WicDeviceBackgroundOperation { request, handle });
}

async fn poll_wic_device_operation(
    app: &mut App,
    operation: &mut Option<WicDeviceBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let action = match operation.handle.await {
        Ok(Ok(response)) => Action::WicDeviceInventoryLoaded {
            request: response.request,
            devices: response.devices,
            limitations: response.limitations,
        },
        Ok(Err(error)) => Action::WicDeviceInventoryFailed {
            request: operation.request,
            message: error.to_string(),
        },
        Err(error) => Action::WicDeviceInventoryFailed {
            request: operation.request,
            message: format!("Wic device discovery task was lost: {error}"),
        },
    };
    let _ = update(app, action);
}

fn configure_wic_capability_inspector(
    app: &App,
    inspector: WicCapabilityInspector,
) -> WicCapabilityInspector {
    let configured_kickstarts = ["WKS_FILE", "WKS_FILES"]
        .into_iter()
        .filter_map(|name| app.workspace.variables.get(name))
        .flat_map(|value| value.split_ascii_whitespace())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .collect();
    let mut canned_roots = app
        .workspace
        .variables
        .get("WKS_SEARCH_PATH")
        .map_or_else(Vec::new, |value| env::split_paths(value).collect());
    if let Some(directory) = app.workspace.variables.get("WKS_FILES_DIR") {
        let directory = PathBuf::from(directory);
        if directory.is_absolute() {
            canned_roots.push(directory);
        }
    }
    canned_roots.retain(|path| path.is_absolute());
    canned_roots.sort();
    canned_roots.dedup();
    inspector.with_sources(configured_kickstarts, canned_roots)
}

fn wic_capability_inspector(app: &App) -> WicCapabilityInspector {
    configure_wic_capability_inspector(app, WicCapabilityInspector::default())
}

fn image_artifact_generation(app: &App) -> Option<u64> {
    match &app.image_artifacts {
        ImageArtifactInventoryState::Loading { request }
        | ImageArtifactInventoryState::AvailableEmpty { request, .. }
        | ImageArtifactInventoryState::Available { request, .. }
        | ImageArtifactInventoryState::Partial { request, .. }
        | ImageArtifactInventoryState::Failed { request, .. } => Some(request.generation),
        ImageArtifactInventoryState::NotLoaded => None,
    }
}

fn begin_wic_capability_operation(
    app: &mut App,
    inspector: &WicCapabilityInspector,
    operation: &mut Option<WicCapabilityBackgroundOperation>,
    effect: Effect,
) {
    if !matches!(effect, Effect::InspectWicCapability) || operation.is_some() {
        return;
    }
    let Some(inventory) = app.image_artifacts.inventory() else {
        let _ = update(
            app,
            Action::WicCapabilityLoaded(WicCapability::Failed {
                message: "image artifact inventory is unavailable".into(),
            }),
        );
        return;
    };
    let Some(image_generation) = image_artifact_generation(app) else {
        return;
    };
    let mut image_targets = inventory
        .artifacts
        .iter()
        .map(|artifact| artifact.identity.image.clone())
        .collect::<Vec<_>>();
    image_targets.sort();
    image_targets.dedup();
    let inspector = inspector.clone();
    let handle = tokio::spawn(async move { inspector.inspect(image_targets).await });
    *operation = Some(WicCapabilityBackgroundOperation {
        image_generation,
        handle,
    });
}

async fn poll_wic_capability_operation(
    app: &mut App,
    inspector: &WicCapabilityInspector,
    operation: &mut Option<WicCapabilityBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(active) = operation.take() else {
        return;
    };
    let active_generation = active.image_generation;
    let capability = match active.handle.await {
        Ok(capability) => capability,
        Err(error) => WicCapability::Failed {
            message: format!("Wic capability background task was lost: {error}"),
        },
    };
    if image_artifact_generation(app) == Some(active_generation) {
        let _ = update(app, Action::WicCapabilityLoaded(capability));
    } else if app.image_artifacts.inventory().is_some() {
        begin_wic_capability_operation(app, inspector, operation, Effect::InspectWicCapability);
    }
}

fn execute_qemu_capability_effect(
    app: &mut App,
    inspector: &QemuCapabilityInspector,
    effect: Effect,
) {
    if !matches!(effect, Effect::InspectQemuCapability) {
        return;
    }
    let capability = app.image_artifacts.inventory().map_or_else(
        || QemuCapability::Failed {
            message: "image artifact inventory is unavailable".into(),
        },
        |inventory| inspector.inspect(&inventory.artifacts),
    );
    let _ = update(app, Action::QemuCapabilityLoaded(capability));
}

fn begin_image_artifact_operation(
    app: &mut App,
    adapter: Option<&ImageArtifactAdapter>,
    operation: &mut Option<ImageArtifactBackgroundOperation>,
    effect: Effect,
) {
    let Effect::GetImageArtifacts(request) = effect else {
        return;
    };
    if operation.is_some() {
        app.notification = Some("An image artifact operation is already running.".into());
        return;
    }
    let Some(adapter) = adapter.cloned() else {
        let _ = update(
            app,
            Action::ImageArtifactInventoryFailed {
                request,
                message: "DEPLOY_DIR_IMAGE is unavailable from the active Yocto workspace".into(),
            },
        );
        let _ = update(
            app,
            Action::QemuCapabilityLoaded(QemuCapability::Failed {
                message: "image artifact inventory is unavailable".into(),
            }),
        );
        let _ = update(
            app,
            Action::WicCapabilityLoaded(WicCapability::Failed {
                message: "image artifact inventory is unavailable".into(),
            }),
        );
        return;
    };
    let cancellation = ImageArtifactCancellation::default();
    let worker_cancellation = cancellation.clone();
    let worker_request = request.clone();
    let handle = tokio::spawn(async move {
        match adapter
            .scan_with_cancellation(worker_request.clone(), worker_cancellation)
            .await
        {
            Ok(response) => response.into(),
            Err(error) => BackendEvent::ImageArtifactsFailed {
                request: worker_request,
                message: error.to_string(),
            },
        }
    });
    *operation = Some(ImageArtifactBackgroundOperation {
        request,
        cancellation,
        handle,
    });
}

async fn poll_image_artifact_operation(
    app: &mut App,
    operation: &mut Option<ImageArtifactBackgroundOperation>,
    qemu_inspector: &QemuCapabilityInspector,
    wic_inspector: &WicCapabilityInspector,
    wic_operation: &mut Option<WicCapabilityBackgroundOperation>,
) {
    if !operation
        .as_ref()
        .is_some_and(|operation| operation.handle.is_finished())
    {
        return;
    }
    let Some(operation) = operation.take() else {
        return;
    };
    let event = match operation.handle.await {
        Ok(event) => event,
        Err(error) => BackendEvent::ImageArtifactsFailed {
            request: operation.request,
            message: format!("image artifact background task was lost: {error}"),
        },
    };
    let failed_message = match &event {
        BackendEvent::ImageArtifactsFailed { message, .. } => Some(message.clone()),
        _ => None,
    };
    if let Some(action) = model_action_from_backend_event(event) {
        let _ = update(app, action);
    }
    if let Some(message) = failed_message {
        let _ = update(
            app,
            Action::QemuCapabilityLoaded(QemuCapability::Failed {
                message: format!("image artifact inventory failed: {message}"),
            }),
        );
        let _ = update(
            app,
            Action::WicCapabilityLoaded(WicCapability::Failed {
                message: format!("image artifact inventory failed: {message}"),
            }),
        );
    } else if let Some(effect) = update(app, Action::InspectQemuCapability) {
        execute_qemu_capability_effect(app, qemu_inspector, effect);
        if let Some(effect) = update(app, Action::InspectWicCapability) {
            begin_wic_capability_operation(app, wic_inspector, wic_operation, effect);
        }
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

async fn complete_devtool_update(app: &mut App, build_dir: &Path, identity: RecipeIdentity) {
    let status = DevtoolInspector::default()
        .inspect(build_dir, identity)
        .await;
    apply_completed_devtool_update_status(app, status);
}

fn apply_completed_devtool_update_status(app: &mut App, status: yoctui_model::DevtoolStatus) {
    let notification = if let Some(error) = &status.error {
        format!(
            "Devtool update-recipe completed, but authoritative status refresh failed: {error:?}"
        )
    } else {
        match &status.workspace {
            DevtoolWorkspace::Present { .. } => {
                "Devtool update-recipe completed and workspace status was refreshed.".into()
            }
            DevtoolWorkspace::MissingDirectory { source_path } => format!(
                "Devtool update-recipe completed, but the reported workspace source is missing: {}",
                source_path.display()
            ),
            DevtoolWorkspace::NotMember => {
                "Devtool update-recipe completed, but the recipe is no longer reported in the workspace."
                    .into()
            }
        }
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}

async fn complete_devtool_finish(app: &mut App, build_dir: &Path, identity: RecipeIdentity) {
    let status = DevtoolInspector::default()
        .inspect(build_dir, identity)
        .await;
    apply_completed_devtool_finish_status(app, status);
}

fn apply_completed_devtool_finish_status(app: &mut App, status: yoctui_model::DevtoolStatus) {
    let notification = if let Some(error) = &status.error {
        format!("Devtool finish completed, but authoritative status refresh failed: {error:?}")
    } else if let DevtoolWorkspace::MissingDirectory { source_path } = &status.workspace {
        format!(
            "Devtool finish completed, but the refreshed workspace source is missing: {}",
            source_path.display()
        )
    } else {
        "Devtool finish completed and workspace status was refreshed.".into()
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}

async fn complete_devtool_deploy(app: &mut App, build_dir: &Path, identity: RecipeIdentity) {
    let status = DevtoolInspector::default()
        .inspect(build_dir, identity)
        .await;
    apply_completed_devtool_deploy_status(app, status);
}

fn apply_completed_devtool_deploy_status(app: &mut App, status: yoctui_model::DevtoolStatus) {
    let notification = if let Some(error) = &status.error {
        format!(
            "Devtool deploy-target completed, but authoritative status refresh failed: {error:?}"
        )
    } else if let DevtoolWorkspace::MissingDirectory { source_path } = &status.workspace {
        format!(
            "Devtool deploy-target completed, but the refreshed workspace source is missing: {}",
            source_path.display()
        )
    } else {
        "Devtool deploy-target completed and workspace status was refreshed.".into()
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
}

async fn complete_devtool_reset(app: &mut App, build_dir: &Path, identity: RecipeIdentity) {
    let status = DevtoolInspector::default()
        .inspect(build_dir, identity)
        .await;
    apply_completed_devtool_reset_status(app, status);
}

fn apply_completed_devtool_reset_status(app: &mut App, status: yoctui_model::DevtoolStatus) {
    let notification = if let Some(error) = &status.error {
        format!("Devtool reset completed, but authoritative status refresh failed: {error:?}")
    } else {
        match &status.workspace {
            DevtoolWorkspace::NotMember => {
                "Devtool reset completed; the recipe is no longer in the workspace.".into()
            }
            DevtoolWorkspace::MissingDirectory { source_path } => format!(
                "Devtool reset completed, but the missing workspace is still reported: {}",
                source_path.display()
            ),
            DevtoolWorkspace::Present { source_path, .. } => format!(
                "Devtool reset completed, but the workspace is still reported at {}",
                source_path.display()
            ),
        }
    };
    let _ = update(app, Action::DevtoolStatusLoaded(status));
    app.notification = Some(notification);
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

async fn load_dependency_graph(app: &mut App, backend: &mut dyn BitBakeBackend, recipe: String) {
    match backend.get_dependency_graph(recipe.clone()).await {
        Ok(response) => {
            let action = if response.limitations.is_empty() {
                Action::DependencyGraphLoaded(response.graph)
            } else {
                Action::DependencyGraphPartial {
                    graph: response.graph,
                    limitations: response.limitations,
                }
            };
            let _ = update(app, action);
        }
        Err(error) => {
            let _ = update(
                app,
                Action::DependencyGraphFailed {
                    root: yoctui_model::DependencyNodeId::recipe(recipe),
                    message: error.to_string(),
                },
            );
        }
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
        build_dir_configured,
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
    let mut app = if build_dir_configured {
        App::new(log_entries, log_bytes)
    } else {
        App::new_unconfigured(log_entries, log_bytes)
    };
    app.backend = backend_kind.to_string();
    app.color_enabled = color;
    app.theme = theme;
    app.animation_speed = animation_speed;
    app.reduced_motion = reduced_motion;
    if build_dir_configured {
        app.screen = session.last_screen.unwrap_or(Screen::Dashboard);
    } else {
        app.screen = Screen::BuildEnvironment;
        app.focus = yoctui_model::FocusTarget::Navigator;
    }
    app.logs.filter = session.log_filter;
    app.logs.recipe_filter = session.log_recipe_filter.clone();
    app.logs.task_filter = session.log_task_filter.clone();
    app.logs.build_filter = session.log_build_filter.clone();
    app.logs.wrap = session.log_wrap.unwrap_or(false);
    app.logs.follow = session.log_follow.unwrap_or(true);
    let session_build_dir = build_dir.clone();
    let mut backend: Box<dyn BitBakeBackend> = if build_dir_configured {
        select_backend_with_timeout(backend_kind.clone(), build_dir, Some(cancellation_timeout))
            .await?
    } else {
        Box::new(ProcessBackend::new(PathBuf::from("/")))
    };
    if build_dir_configured {
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
    } else {
        app.notification =
            Some("Configure and verify a BitBake environment in Settings before building.".into());
    }
    if !targets.is_empty() {
        app.build.target = targets.first().cloned()
    }
    let mut build_jobs = BuildJobCoordinator::default();
    let mut devtool_jobs = DevtoolJobCoordinator::default();
    let mut devtool_runner = None;
    let mut pending_devtool_modify = None;
    let mut pending_devtool_update = None;
    let mut pending_devtool_finish = None;
    let mut pending_devtool_deploy = None;
    let mut pending_devtool_reset = None;
    let signature_adapter = SignatureAdapter::new(session_build_dir.clone());
    let mut signature_operation = None;
    let package_adapter = PackageDataAdapter::new(session_build_dir.clone());
    let mut package_operation = None;
    let image_artifact_adapter = app
        .workspace
        .variables
        .get("DEPLOY_DIR_IMAGE")
        .map(PathBuf::from)
        .map(ImageArtifactAdapter::new);
    let mut image_artifact_operation = None;
    let sdk_artifact_adapter = app
        .workspace
        .variables
        .get("SDK_DEPLOY")
        .map(PathBuf::from)
        .map(SdkArtifactAdapter::new);
    let sdk_tool_adapter = match sdk_tool_adapter_for_workspace(&app, &session_build_dir) {
        Ok(adapter) => Some(adapter),
        Err(message) => {
            let _ = update(
                &mut app,
                Action::SdkToolCapabilityLoaded(SdkToolCapability::Failed { message }),
            );
            None
        }
    };
    let mut sdk_artifact_operation = None;
    let mut sdk_capability_operation = None;
    let mut sdk_operation = None;
    let mut pending_sdk_build = None;
    let qemu_inspector = QemuCapabilityInspector::default();
    let mut qemu_operation = None;
    let wic_inspector = wic_capability_inspector(&app);
    let mut wic_capability_operation = None;
    let wic_device_inspector = WicDeviceInspector::default();
    let mut wic_device_operation = None;
    let mut wic_operation = None;
    let initialized_paths = initialized_path_directories();
    let mut test_coordinator = TestCliCoordinator::new(
        session_build_dir.clone(),
        initialized_paths.clone(),
        ptest_capability(&app),
    );
    let mut pending_test_build = None;
    let mut security_coordinator =
        SecurityCliCoordinator::new(session_build_dir.clone(), initialized_paths.clone());
    let mut pending_security_build = None;
    let mut qa_coordinator =
        QaCliCoordinator::new(session_build_dir.clone(), initialized_paths.clone());
    let mut pending_qa_build = None;
    let maintenance_build_dir = if app.build_environment.connected() {
        session_build_dir.clone()
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"))
    };
    let mut maintenance_coordinator =
        MaintenanceCliCoordinator::new(&app, &maintenance_build_dir, initialized_paths)
            .map_err(anyhow::Error::msg)?;
    if app.screen == Screen::Packages
        && let Some(effect @ Effect::GetPackageInventory(_)) =
            update(&mut app, Action::BeginPackageInventory)
    {
        begin_package_operation(&mut app, &package_adapter, &mut package_operation, effect);
    }
    if app.screen == Screen::Images
        && let Some(effect @ Effect::GetImageArtifacts(_)) =
            update(&mut app, Action::BeginImageArtifactInventory)
    {
        begin_image_artifact_operation(
            &mut app,
            image_artifact_adapter.as_ref(),
            &mut image_artifact_operation,
            effect,
        );
    }
    if sdk_tool_adapter.is_some() {
        begin_sdk_capability_operation(
            &mut app,
            sdk_tool_adapter.as_ref(),
            &mut sdk_capability_operation,
            Effect::InspectSdkTools,
        );
    }
    if app.screen == Screen::Testing
        && let Some(effect) = update(&mut app, Action::InspectTestCapability)
    {
        let _ = test_coordinator.handle_effect(&mut app, effect).await;
    }
    if app.screen == Screen::Security
        && let Some(effect) = update(
            &mut app,
            Action::Security(SecurityAction::InspectCapability),
        )
    {
        let _ = security_coordinator.handle_effect(&mut app, effect).await;
    }
    if app.screen == Screen::Qa
        && let Some(effect) = update(&mut app, Action::Qa(QaAction::InspectCapability))
    {
        let _ = qa_coordinator.handle_effect(&mut app, effect).await;
    }
    if app.screen == Screen::Maintenance
        && let Some(effect) = update(
            &mut app,
            Action::Maintenance(yoctui_model::MaintenanceAction::InspectCapability),
        )
    {
        let _ = maintenance_coordinator
            .handle_effect(&mut app, effect)
            .await;
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
        poll_signature_operation(&mut app, &mut signature_operation).await;
        poll_package_operation(&mut app, &mut package_operation).await;
        poll_image_artifact_operation(
            &mut app,
            &mut image_artifact_operation,
            &qemu_inspector,
            &wic_inspector,
            &mut wic_capability_operation,
        )
        .await;
        poll_sdk_artifact_operation(&mut app, &mut sdk_artifact_operation).await;
        poll_sdk_capability_operation(&mut app, &mut sdk_capability_operation).await;
        if let Some(completed) = poll_sdk_job(&mut app, &mut sdk_operation).await
            && matches!(completed, SdkOperation::Publish(_))
            && let Some(effect) = update(&mut app, Action::RefreshSdkArtifactInventory)
        {
            begin_sdk_artifact_operation(
                &mut app,
                sdk_artifact_adapter.as_ref(),
                &mut sdk_artifact_operation,
                effect,
            );
        }
        poll_wic_capability_operation(&mut app, &wic_inspector, &mut wic_capability_operation)
            .await;
        poll_wic_device_operation(&mut app, &mut wic_device_operation).await;
        poll_qemu_job(&mut app, &mut qemu_operation).await;
        poll_wic_job(&mut app, &mut wic_operation).await;
        test_coordinator.poll(&mut app).await;
        security_coordinator.poll(&mut app).await;
        qa_coordinator.poll(&mut app).await;
        maintenance_coordinator.poll(&mut app).await;
        if (matches!(
            app.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        ) || wic_operation.is_some()
            || sdk_operation.is_some()
            || test_coordinator.session.is_some()
            || test_coordinator.result.is_some()
            || security_coordinator.mapper.is_some()
            || qa_coordinator.layer.is_some()
            || maintenance_coordinator.operation_active())
            && Instant::now() >= next_telemetry_sample
        {
            let telemetry = telemetry_sampler.sample(&session_build_dir);
            let _ = update(&mut app, Action::HostTelemetryUpdated(telemetry));
            next_telemetry_sample = Instant::now() + Duration::from_secs(1);
        }
        let _ = update(&mut app, Action::Tick);
        terminal.draw(|f| render(f, &app))?;
        if event::poll(refresh)? {
            let terminal_event = event::read()?;
            if let Event::Paste(text) = terminal_event {
                let popup_action = match app.active_dialog() {
                    Some(Dialog::BuildEnvironmentEditor { editing: true, .. }) => {
                        Some(Action::AppendBuildEnvironmentEditor as fn(char) -> Action)
                    }
                    Some(Dialog::BuildEnvironmentCloneEditor { editing: true, .. }) => {
                        Some(Action::AppendBuildEnvironmentCloneEditor as fn(char) -> Action)
                    }
                    Some(Dialog::ConfigEdit { editing: true, .. }) => {
                        Some(Action::AppendConfigEdit as fn(char) -> Action)
                    }
                    Some(Dialog::BbmaskEdit { editing: true, .. }) => {
                        Some(Action::AppendBbmask as fn(char) -> Action)
                    }
                    Some(Dialog::BuildTarget { editing: true, .. }) => {
                        Some(Action::AppendBuildTarget as fn(char) -> Action)
                    }
                    Some(Dialog::WicCreateTomlEditor { editing: true, .. }) => {
                        Some(Action::AppendWicCreateTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::SdkPublishTomlEditor { editing: true, .. }) => {
                        Some(Action::AppendSdkPublishTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::SdkNativeTomlEditor { editing: true, .. }) => {
                        Some(Action::AppendSdkNativeTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::TestLaunchTomlEditor { editing: true, .. }) => {
                        Some(Action::AppendTestLaunchTomlEditor as fn(char) -> Action)
                    }
                    Some(Dialog::TestJunitTomlEditor { editor, .. }) if editor.editing => {
                        Some(Action::AppendTestJunitTomlEditor as fn(char) -> Action)
                    }
                    _ => None,
                };
                if let Some(action) = popup_action {
                    for character in text.chars().filter(|character| !character.is_control()) {
                        let _ = update(&mut app, action(character));
                    }
                } else if app.screen == Screen::BuildEnvironment
                    && app
                        .build_environment_draft
                        .as_ref()
                        .is_some_and(|draft| draft.editing)
                {
                    for character in text.chars() {
                        let _ = update(&mut app, Action::AppendBuildEnvironmentField(character));
                    }
                }
                continue;
            }
            if let Event::Key(k) = terminal_event {
                let Some(input) = input_from_key(k) else {
                    continue;
                };
                if app.command_palette_open {
                    let effect = match input {
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
                    if let Some(effect @ Effect::GetImageArtifacts(_)) = effect {
                        begin_image_artifact_operation(
                            &mut app,
                            image_artifact_adapter.as_ref(),
                            &mut image_artifact_operation,
                            effect,
                        );
                    } else if let Some(
                        effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                    ) = effect
                    {
                        begin_package_operation(
                            &mut app,
                            &package_adapter,
                            &mut package_operation,
                            effect,
                        );
                    } else if let Some(effect @ Effect::InspectSdkTools) = effect {
                        begin_sdk_capability_operation(
                            &mut app,
                            sdk_tool_adapter.as_ref(),
                            &mut sdk_capability_operation,
                            effect,
                        );
                    } else if let Some(
                        effect @ (Effect::InspectTestCapability
                        | Effect::InspectResultToolCapability),
                    ) = effect
                    {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    } else if let Some(effect @ Effect::Security(_)) = effect {
                        let _ = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    } else if let Some(effect @ Effect::Qa(_)) = effect {
                        let _ = route_independent_qa_effect(
                            &guard,
                            &mut app,
                            &mut qa_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    } else if let Some(effect @ Effect::Maintenance(_)) = effect {
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if let Some(Dialog::BuildEnvironmentCloneEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleBuildEnvironmentCloneEditor),
                            Input::Enter => Some(Action::ReviewBuildEnvironmentClone),
                            Input::Backspace => Some(Action::BackspaceBuildEnvironmentCloneEditor),
                            Input::Char(character) => {
                                Some(Action::AppendBuildEnvironmentCloneEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleBuildEnvironmentCloneEditor),
                            Input::Enter => Some(Action::ReviewBuildEnvironmentClone),
                            Input::Esc | Input::Char('q') => {
                                Some(Action::CancelBuildEnvironmentClone)
                            }
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::BuildEnvironmentCloneReview(_))
                ) {
                    let action = match input {
                        Input::Enter => Some(Action::ConfirmBuildEnvironmentClone),
                        Input::Esc => Some(Action::CancelBuildEnvironmentClone),
                        _ => None,
                    };
                    if let Some(action) = action
                        && let Some(Effect::CloneBuildEnvironment(plan)) = update(&mut app, action)
                    {
                        match BuildEnvironmentAdapter::default()
                            .clone_poky(plan.request.clone())
                            .await
                        {
                            Ok(_) => {
                                let profile = yoctui_model::BuildEnvironmentProfile {
                                    source_dir: plan.request.destination.clone(),
                                    build_dir: plan.build_dir,
                                    init_script: plan.request.destination.join("oe-init-build-env"),
                                };
                                let _ =
                                    update(&mut app, Action::ConfigureBuildEnvironment(profile));
                                app.notification = Some(
                                    "Poky cloned. Press V to initialize and verify BitBake.".into(),
                                );
                            }
                            Err(error) => {
                                app.notification = Some(format!("Poky clone failed: {error}"))
                            }
                        }
                    }
                } else if let Some(Dialog::BuildEnvironmentEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleBuildEnvironmentEditor),
                            Input::Enter => Some(Action::ApplyBuildEnvironmentEditor),
                            Input::Backspace => Some(Action::BackspaceBuildEnvironmentEditor),
                            Input::Char(character) => {
                                Some(Action::AppendBuildEnvironmentEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleBuildEnvironmentEditor),
                            Input::Char('q') | Input::Esc => {
                                Some(Action::CloseBuildEnvironmentEditor)
                            }
                            Input::Enter => Some(Action::ApplyBuildEnvironmentEditor),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ThemePicker { .. })) {
                    let action = match input {
                        Input::Up | Input::Char('k') => Some(Action::SelectTheme { delta: -1 }),
                        Input::Down | Input::Char('j') => Some(Action::SelectTheme { delta: 1 }),
                        Input::Enter => Some(Action::ApplySelectedTheme),
                        Input::Esc => Some(Action::CloseThemePicker),
                        _ => None,
                    };
                    if let Some(action) = action
                        && let Some(Effect::PersistSettings) = update(&mut app, action)
                    {
                        let result = persist_settings(session_path.as_deref(), &mut session, &app);
                        let persistence_action = match result {
                            Ok(()) => Action::SettingsPersisted,
                            Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                        };
                        let _ = update(&mut app, persistence_action);
                    }
                } else if let Some(Dialog::Maintenance(dialog)) = app.active_dialog().cloned() {
                    let effect = maintenance_dialog_action(&dialog, input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if let Some(Dialog::Security(dialog)) = app.active_dialog().cloned() {
                    let effect = security_dialog_action(&dialog, input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let routed = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect.clone(),
                            editor.as_deref(),
                        )
                        .await;
                        if !routed {
                            match effect {
                                Effect::Security(SecurityEffect::StartBuild { id, request }) => {
                                    if begin_security_build(
                                        &mut backend,
                                        &mut app,
                                        &mut build_jobs,
                                        id,
                                        request,
                                    )
                                    .await
                                    {
                                        pending_security_build = Some(id);
                                    }
                                }
                                Effect::Security(SecurityEffect::CancelSession(id))
                                    if pending_security_build == Some(id) =>
                                {
                                    if let Some(action) = build_jobs.request_cancellation() {
                                        let _ = update(&mut app, action);
                                    }
                                    if let Err(error) = backend.cancel_build().await {
                                        let _ = update(
                                            &mut app,
                                            Action::Security(SecurityAction::RejectCancellation {
                                                id,
                                                message: error.to_string(),
                                            }),
                                        );
                                        for action in build_jobs.cancellation_failed(
                                            error.to_string(),
                                            SystemTime::now(),
                                        ) {
                                            let _ = update(&mut app, action);
                                        }
                                    }
                                }
                                Effect::Security(SecurityEffect::CancelSession(id)) => {
                                    let _ = update(
                                        &mut app,
                                        Action::Security(SecurityAction::RejectCancellation {
                                            id,
                                            message: "the CLI does not own this Security operation"
                                                .into(),
                                        }),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                } else if let Some(Dialog::Qa(dialog)) = app.active_dialog().cloned() {
                    let effect = qa_dialog_action(&dialog, input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let routed = route_independent_qa_effect(
                            &guard,
                            &mut app,
                            &mut qa_coordinator,
                            effect.clone(),
                            editor.as_deref(),
                        )
                        .await;
                        if !routed {
                            match effect {
                                Effect::Qa(QaEffect::StartBuild { session, request }) => {
                                    if begin_qa_build(
                                        &mut backend,
                                        &mut app,
                                        &mut build_jobs,
                                        session,
                                        request,
                                    )
                                    .await
                                    {
                                        pending_qa_build = Some(session);
                                    }
                                }
                                Effect::Qa(QaEffect::CancelBuild { session, .. })
                                    if pending_qa_build == Some(session) =>
                                {
                                    if let Some(action) = build_jobs.request_cancellation() {
                                        let _ = update(&mut app, action);
                                    }
                                    if let Err(error) = backend.cancel_build().await {
                                        let _ = update(
                                            &mut app,
                                            Action::Qa(QaAction::RejectCancellation {
                                                session,
                                                message: error.to_string(),
                                            }),
                                        );
                                        for action in build_jobs.cancellation_failed(
                                            error.to_string(),
                                            SystemTime::now(),
                                        ) {
                                            let _ = update(&mut app, action);
                                        }
                                    }
                                }
                                Effect::Qa(QaEffect::CancelBuild { session, .. }) => {
                                    let _ = update(
                                        &mut app,
                                        Action::Qa(QaAction::RejectCancellation {
                                            session,
                                            message: "the CLI does not own this QA managed build"
                                                .into(),
                                        }),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::SdkBuildConfirmation(_))) {
                    let effect = sdk_build_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::Start(request)) = effect {
                        let tracked = sdk_build_is_populate(&request);
                        if begin_build(&mut backend, &mut app, &mut build_jobs, request.clone())
                            .await
                            && tracked
                        {
                            pending_sdk_build = Some(request);
                        }
                    }
                } else if let Some(Dialog::SdkPublishTomlEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleSdkPublishTomlEditor),
                            Input::Enter => Some(Action::PreviewSdkPublish),
                            Input::Backspace => Some(Action::BackspaceSdkPublishTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendSdkPublishTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleSdkPublishTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelSdkPublish),
                            Input::Enter => Some(Action::PreviewSdkPublish),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::SdkPublish(_))) {
                    let _ = sdk_publish_dialog_action(input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::SdkPublishConfirmation(_))) {
                    let effect = sdk_publish_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::StartSdkSession { id, operation }) = effect {
                        begin_sdk_job(
                            &mut app,
                            &mut sdk_operation,
                            sdk_tool_adapter.as_ref(),
                            cancellation_timeout,
                            SDK_TOOL_OPERATION_TIMEOUT,
                            id,
                            operation,
                        );
                    }
                } else if let Some(Dialog::SdkNativeTomlEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleSdkNativeTomlEditor),
                            Input::Enter => Some(Action::PreviewSdkNative),
                            Input::Backspace => Some(Action::BackspaceSdkNativeTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendSdkNativeTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleSdkNativeTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelSdkNative),
                            Input::Enter => Some(Action::PreviewSdkNative),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if let Some(Dialog::SdkNative(dialog)) = app.active_dialog() {
                    let editing = dialog.editing;
                    let _ = sdk_native_dialog_action(editing, input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::SdkNativeConfirmation(_))) {
                    let effect = sdk_native_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::StartSdkSession { id, operation }) = effect {
                        begin_sdk_job(
                            &mut app,
                            &mut sdk_operation,
                            sdk_tool_adapter.as_ref(),
                            cancellation_timeout,
                            SDK_TOOL_OPERATION_TIMEOUT,
                            id,
                            operation,
                        );
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::SdkCancellationConfirmation(_))
                ) {
                    let effect = sdk_cancellation_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::CancelSdkSession(id)) = effect {
                        begin_sdk_cancellation(&mut app, &mut sdk_operation, id);
                    }
                } else if let Some(Dialog::TestLaunchTomlEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleTestLaunchTomlEditor),
                            Input::Enter => Some(Action::PreviewTestLaunch),
                            Input::Backspace => Some(Action::BackspaceTestLaunchTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendTestLaunchTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleTestLaunchTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelTestLaunch),
                            Input::Enter => Some(Action::PreviewTestLaunch),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog() {
                    let editing = dialog.editing;
                    let _ = test_launch_dialog_action(editing, input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::TestLaunchConfirmation(_))) {
                    let effect = test_launch_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    match effect {
                        Some(effect @ Effect::StartTestSession { .. }) => {
                            let _ = test_coordinator.handle_effect(&mut app, effect).await;
                        }
                        Some(Effect::StartTestBuildSession { id, request }) => {
                            if begin_test_build(
                                &mut backend,
                                &mut app,
                                &mut build_jobs,
                                id,
                                request,
                            )
                            .await
                            {
                                pending_test_build = Some(id);
                            }
                        }
                        _ => {}
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::TestCancellationConfirmation(_))
                ) {
                    let effect = test_cancellation_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect @ Effect::CancelTestSession(id)) = effect
                        && !test_coordinator.handle_effect(&mut app, effect).await
                        && pending_test_build == Some(id)
                    {
                        if let Some(action) = build_jobs.request_cancellation() {
                            let _ = update(&mut app, action);
                        }
                        if let Err(error) = backend.cancel_build().await {
                            let _ = update(
                                &mut app,
                                Action::RejectTestSessionCancellation {
                                    id,
                                    message: error.to_string(),
                                },
                            );
                            for action in
                                build_jobs.cancellation_failed(error.to_string(), SystemTime::now())
                            {
                                let _ = update(&mut app, action);
                            }
                        }
                    }
                } else if let Some(Dialog::TestResultImportTomlEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleTestResultImportTomlEditor),
                            Input::Enter => Some(Action::ConfirmTestResultImport),
                            Input::Backspace => Some(Action::BackspaceTestResultImportTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendTestResultImportTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleTestResultImportTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelTestResultImport),
                            Input::Enter => Some(Action::ConfirmTestResultImport),
                            _ => None,
                        }
                    };
                    if let Some(effect) = action.and_then(|action| update(&mut app, action)) {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::TestResultImport(_))) {
                    let effect = test_result_import_dialog_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if let Some(Dialog::TestComparisonTomlEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleTestComparisonTomlEditor),
                            Input::Enter => Some(Action::PreviewTestComparison),
                            Input::Backspace => Some(Action::BackspaceTestComparisonTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendTestComparisonTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleTestComparisonTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelTestComparison),
                            Input::Enter => Some(Action::PreviewTestComparison),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::TestComparison(_))) {
                    let _ = test_comparison_dialog_action(input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::TestComparisonConfirmation(_))
                ) {
                    let effect = test_comparison_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if let Some(Dialog::TestJunitTomlEditor { editor, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editor.editing {
                        match input {
                            Input::Esc => Some(Action::ToggleTestJunitTomlEditor),
                            Input::Enter => Some(Action::PreviewTestJunitExport),
                            Input::Backspace => Some(Action::BackspaceTestJunitTomlEditor),
                            Input::Left => Some(Action::MoveTestJunitTomlEditorLeft),
                            Input::Right => Some(Action::MoveTestJunitTomlEditorRight),
                            Input::Up => Some(Action::MoveTestJunitTomlEditorUp),
                            Input::Down => Some(Action::MoveTestJunitTomlEditorDown),
                            Input::Home => Some(Action::MoveTestJunitTomlEditorHome),
                            Input::End => Some(Action::MoveTestJunitTomlEditorEnd),
                            Input::CtrlC => Some(Action::CopyTestJunitTomlEditor),
                            Input::CtrlV => Some(Action::PasteTestJunitTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendTestJunitTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleTestJunitTomlEditor),
                            Input::Char('e') => Some(Action::SelectTestJunitDestination),
                            Input::Left | Input::Char('h') => {
                                Some(Action::MoveTestJunitTomlEditorLeft)
                            }
                            Input::Right | Input::Char('l') => {
                                Some(Action::MoveTestJunitTomlEditorRight)
                            }
                            Input::Up | Input::Char('k') => Some(Action::MoveTestJunitTomlEditorUp),
                            Input::Down | Input::Char('j') => {
                                Some(Action::MoveTestJunitTomlEditorDown)
                            }
                            Input::Home => Some(Action::MoveTestJunitTomlEditorHome),
                            Input::End => Some(Action::MoveTestJunitTomlEditorEnd),
                            Input::CtrlC => Some(Action::CopyTestJunitTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelTestJunitExport),
                            Input::Enter => Some(Action::PreviewTestJunitExport),
                            _ => None,
                        }
                    };
                    if let Some(effect) = action.and_then(|action| update(&mut app, action)) {
                        match effect {
                            Effect::CopyToClipboard(content) => {
                                copy_to_clipboard(&mut app, content).await;
                            }
                            effect => {
                                let _ = test_coordinator.handle_effect(&mut app, effect).await;
                            }
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::TestJunitExport(_))) {
                    let effect =
                        test_junit_dialog_action(input).and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::TestJunitExportConfirmation(_))
                ) {
                    let effect = test_junit_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect) = effect {
                        let _ = test_coordinator.handle_effect(&mut app, effect).await;
                    }
                } else if let Some(Dialog::WicCreateTomlEditor { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleWicCreateTomlEditor),
                            Input::Enter => Some(Action::PreviewWicCreate),
                            Input::Backspace => Some(Action::BackspaceWicCreateTomlEditor),
                            Input::Char(character) => {
                                Some(Action::AppendWicCreateTomlEditor(character))
                            }
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleWicCreateTomlEditor),
                            Input::Char('q') | Input::Esc => Some(Action::CancelWicCreate),
                            Input::Enter => Some(Action::PreviewWicCreate),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::WicCreate(_))) {
                    let editing = app.active_dialog().is_some_and(
                        |dialog| matches!(dialog, Dialog::WicCreate(state) if state.editing),
                    );
                    let _ = wic_create_dialog_action(editing, input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::WicCreateConfirmation(_))) {
                    let effect = wic_create_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::StartWicSession { id, operation }) = effect {
                        begin_wic_job(
                            &mut app,
                            &mut wic_operation,
                            &wic_device_inspector,
                            &session_build_dir,
                            cancellation_timeout,
                            id,
                            operation,
                        )
                        .await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::WicDevicePicker(_))) {
                    let _ =
                        wic_device_picker_action(input).and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::WicWritePhrase(_))) {
                    let _ =
                        wic_write_phrase_action(input).and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::WicWriteConfirmation(_))) {
                    let effect = wic_write_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::StartWicSession { id, operation }) = effect {
                        begin_wic_job(
                            &mut app,
                            &mut wic_operation,
                            &wic_device_inspector,
                            &session_build_dir,
                            cancellation_timeout,
                            id,
                            operation,
                        )
                        .await;
                    }
                } else if let Some(Dialog::WicCancellationConfirmation {
                    id,
                    incomplete_device_warning,
                }) = app.active_dialog().cloned()
                {
                    let effect =
                        wic_cancellation_confirmation_action(id, incomplete_device_warning, input)
                            .and_then(|action| update(&mut app, action));
                    if let Some(Effect::CancelWicSession(id)) = effect {
                        begin_wic_cancellation(&mut app, &mut wic_operation, id);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))) {
                    let editing = app.active_dialog().is_some_and(
                        |dialog| matches!(dialog, Dialog::QemuLaunch(state) if state.editing),
                    );
                    let _ = qemu_launch_dialog_action(editing, input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(app.active_dialog(), Some(Dialog::QemuLaunchConfirmation(_))) {
                    let effect = qemu_launch_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::StartQemuSession { id, request }) = effect {
                        begin_qemu_job(
                            &mut app,
                            &mut qemu_operation,
                            &session_build_dir,
                            cancellation_timeout,
                            id,
                            request,
                        )
                        .await;
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::QemuCancellationConfirmation(_))
                ) {
                    let effect = qemu_cancellation_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::CancelQemuSession(id)) = effect {
                        begin_qemu_cancellation(&mut app, &mut qemu_operation, id);
                    }
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
                } else if app.screen == Screen::Signatures
                    && app.active_dialog().is_none()
                    && app.notification.is_none()
                {
                    let effect = signature_workspace_action(input)
                        .and_then(|action| update(&mut app, action));
                    match effect {
                        Some(
                            effect @ (Effect::GetSignatureDump(_) | Effect::CompareSignatures(_)),
                        ) => begin_signature_operation(
                            &mut app,
                            &signature_adapter,
                            &mut signature_operation,
                            effect,
                        ),
                        Some(Effect::CancelSignatureOperation) => {
                            if let Some(operation) = signature_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("Signature cancellation requested.".into());
                                }
                            } else {
                                app.notification =
                                    Some("No signature operation is running.".into());
                            }
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {
                            if matches!(input, Input::Char('q') | Input::CtrlC) {
                                let _ = update(&mut app, Action::Quit);
                            } else if input == Input::Char('?') {
                                let _ = update(&mut app, Action::Open(Screen::Help));
                            }
                        }
                    }
                } else if matches!(
                    app.focus,
                    yoctui_model::FocusTarget::Navigator | yoctui_model::FocusTarget::Inspector
                ) {
                    if let Some(action) = focus_action(app.focus, input) {
                        let effect = update(&mut app, action);
                        if let Some(
                            effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                        ) = effect
                        {
                            begin_package_operation(
                                &mut app,
                                &package_adapter,
                                &mut package_operation,
                                effect,
                            );
                        } else if let Some(effect @ Effect::GetImageArtifacts(_)) = effect {
                            begin_image_artifact_operation(
                                &mut app,
                                image_artifact_adapter.as_ref(),
                                &mut image_artifact_operation,
                                effect,
                            );
                        } else if let Some(effect @ Effect::InspectSdkTools) = effect {
                            begin_sdk_capability_operation(
                                &mut app,
                                sdk_tool_adapter.as_ref(),
                                &mut sdk_capability_operation,
                                effect,
                            );
                        } else if let Some(effect @ Effect::Security(_)) = effect {
                            let _ = route_independent_security_effect(
                                &guard,
                                &mut app,
                                &mut security_coordinator,
                                effect,
                                editor.as_deref(),
                            )
                            .await;
                        } else if let Some(effect @ Effect::Maintenance(_)) = effect {
                            let _ = route_independent_maintenance_effect(
                                &guard,
                                &mut app,
                                &mut maintenance_coordinator,
                                effect,
                                editor.as_deref(),
                            )
                            .await;
                        }
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
                        Input::Up => {
                            update(&mut app, Action::SelectLayerBrowserEntry { delta: -1 })
                        }
                        Input::Down => {
                            update(&mut app, Action::SelectLayerBrowserEntry { delta: 1 })
                        }
                        Input::Enter => update(&mut app, Action::LayerBrowserEnter),
                        Input::Right | Input::Char('l') => {
                            update(&mut app, Action::LayerBrowserExpand)
                        }
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
                    let effect = devtool_reset_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::DevtoolReset(plan)) = effect {
                        let operation = plan.operation();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            operation,
                        )
                        .await
                        {
                            pending_devtool_reset = Some(plan.identity);
                        }
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolUpdateConfirmation(_))
                ) {
                    let effect = devtool_update_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::DevtoolUpdateRecipe(identity)) = effect {
                        let recipe = identity.name.clone();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            DevtoolOperation::UpdateRecipe { recipe },
                        )
                        .await
                        {
                            pending_devtool_update = Some(identity);
                        }
                    }
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolFinishConfirmation(_))
                ) {
                    let effect = devtool_finish_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::DevtoolFinish(plan)) = effect {
                        let request = plan.request();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            request.into(),
                        )
                        .await
                        {
                            pending_devtool_finish = Some(plan.identity);
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::DevtoolFinishPicker(_))) {
                    let _ = devtool_finish_picker_action(input)
                        .and_then(|action| update(&mut app, action));
                } else if matches!(
                    app.active_dialog(),
                    Some(Dialog::DevtoolDeployConfirmation(_))
                ) {
                    let effect = devtool_deploy_confirmation_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::DevtoolDeploy(plan)) = effect {
                        let request = plan.request();
                        if begin_devtool_job(
                            &mut app,
                            &mut devtool_jobs,
                            &mut devtool_runner,
                            &session_build_dir,
                            cancellation_timeout,
                            request.into(),
                        )
                        .await
                        {
                            pending_devtool_deploy = Some(plan.identity);
                        }
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::DevtoolDeploy(_))) {
                    let _ = devtool_deploy_dialog_action(input)
                        .and_then(|action| update(&mut app, action));
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
                } else if let Some(Dialog::BbmaskEdit { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleBbmaskEdit),
                            Input::Enter => Some(Action::PreviewBbmaskEdit),
                            Input::Backspace => Some(Action::BackspaceBbmask),
                            Input::Char(character) => Some(Action::AppendBbmask(character)),
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleBbmaskEdit),
                            Input::Char('q') | Input::Esc => Some(Action::CancelBbmaskEdit),
                            Input::Enter => Some(Action::PreviewBbmaskEdit),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
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
                } else if matches!(app.active_dialog(), Some(Dialog::SignatureTaskPicker(_))) {
                    let effect = signature_task_picker_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(effect @ Effect::GetSignatureDump(_)) = effect {
                        begin_signature_operation(
                            &mut app,
                            &signature_adapter,
                            &mut signature_operation,
                            effect,
                        );
                    }
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
                    let effect = config_source_picker_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::OpenInEditor(path)) = effect {
                        open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigScopePicker(_))) {
                    let effect = config_scope_picker_action(input)
                        .and_then(|action| update(&mut app, action));
                    if let Some(Effect::GetVariable(identity)) = effect {
                        load_config_variable(&mut app, backend.as_mut(), identity).await;
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigComparison(_))) {
                    if let Some(action) = config_compare_dialog_action(input) {
                        let _ = update(&mut app, action);
                    }
                } else if let Some(Dialog::ConfigEdit { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleConfigEdit),
                            Input::Enter => Some(Action::PreviewConfigEdit),
                            Input::Backspace => Some(Action::BackspaceConfigEdit),
                            Input::Char(character) => Some(Action::AppendConfigEdit(character)),
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleConfigEdit),
                            Input::Char('q') | Input::Esc => Some(Action::CancelConfigEdit),
                            Input::Enter => Some(Action::PreviewConfigEdit),
                            _ => None,
                        }
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if matches!(app.active_dialog(), Some(Dialog::ConfigEditConfirmation(_))) {
                    if let Some(action) = config_edit_confirmation_action(input)
                        && let Some(Effect::WriteConfigAssignment(request)) =
                            update(&mut app, action)
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
                } else if let Some(Dialog::BuildTarget { editing, .. }) =
                    app.active_dialog().cloned()
                {
                    let action = if editing {
                        match input {
                            Input::Esc => Some(Action::ToggleBuildTargetEdit),
                            Input::Enter => Some(Action::ConfirmBuildTarget),
                            Input::Backspace => Some(Action::BackspaceBuildTarget),
                            Input::Char(character) => Some(Action::AppendBuildTarget(character)),
                            _ => None,
                        }
                    } else {
                        match input {
                            Input::Char('i') => Some(Action::ToggleBuildTargetEdit),
                            Input::Char('q') | Input::Esc => Some(Action::CancelBuildTargetEdit),
                            Input::Enter => Some(Action::ConfirmBuildTarget),
                            _ => None,
                        }
                    };
                    let effect = action.and_then(|action| update(&mut app, action));
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
                } else if app.screen == Screen::Packages
                    && package_workspace_action(app.package_searching, input).is_some()
                {
                    let action = package_workspace_action(app.package_searching, input)
                        .expect("Packages action was checked");
                    match update(&mut app, action) {
                        Some(
                            effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
                        ) => begin_package_operation(
                            &mut app,
                            &package_adapter,
                            &mut package_operation,
                            effect,
                        ),
                        Some(Effect::CancelPackageOperation) => {
                            if let Some(operation) = package_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("Package-data cancellation requested.".into());
                                }
                            } else {
                                app.notification =
                                    Some("No package-data operation is running.".into());
                            }
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Images
                    && images_workspace_action(app.image_artifact_searching, input).is_some()
                {
                    let action = images_workspace_action(app.image_artifact_searching, input)
                        .expect("Images action was checked");
                    match update(&mut app, action) {
                        Some(effect @ Effect::GetImageArtifacts(_)) => {
                            begin_image_artifact_operation(
                                &mut app,
                                image_artifact_adapter.as_ref(),
                                &mut image_artifact_operation,
                                effect,
                            )
                        }
                        Some(effect @ Effect::GetWicDevices(_)) => begin_wic_device_operation(
                            &wic_device_inspector,
                            &mut wic_device_operation,
                            effect,
                        ),
                        Some(Effect::CancelImageArtifactOperation) => {
                            if let Some(operation) = image_artifact_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("Image artifact cancellation requested.".into());
                                }
                            } else {
                                app.notification =
                                    Some("No image artifact operation is running.".into());
                            }
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Sdk
                    && sdk_workspace_action(app.sdk_artifact_searching, input).is_some()
                {
                    let action = sdk_workspace_action(app.sdk_artifact_searching, input)
                        .expect("SDK action was checked");
                    match update(&mut app, action) {
                        Some(effect @ Effect::GetSdkArtifacts(_)) => begin_sdk_artifact_operation(
                            &mut app,
                            sdk_artifact_adapter.as_ref(),
                            &mut sdk_artifact_operation,
                            effect,
                        ),
                        Some(Effect::CancelSdkArtifactOperation) => {
                            if let Some(operation) = sdk_artifact_operation.as_ref() {
                                if operation.cancellation.cancel() {
                                    app.notification =
                                        Some("SDK artifact cancellation requested.".into());
                                }
                            } else {
                                app.notification = Some("No SDK artifact scan is running.".into());
                            }
                        }
                        Some(effect @ Effect::InspectSdkTools) => begin_sdk_capability_operation(
                            &mut app,
                            sdk_tool_adapter.as_ref(),
                            &mut sdk_capability_operation,
                            effect,
                        ),
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Testing
                    && testing_screen_action(&app, input).is_some()
                {
                    let action =
                        testing_screen_action(&app, input).expect("Testing action was checked");
                    match update(&mut app, action) {
                        Some(effect @ Effect::ImportTestResults(_))
                        | Some(effect @ Effect::CompareTestResults(_))
                        | Some(effect @ Effect::InspectTestJunitDestination { .. })
                        | Some(effect @ Effect::ExportTestJunit(_))
                        | Some(effect @ Effect::InspectTestCapability)
                        | Some(effect @ Effect::InspectResultToolCapability) => {
                            let _ = test_coordinator.handle_effect(&mut app, effect).await;
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
                    }
                } else if app.screen == Screen::Security
                    && security_workspace_action(
                        app.security.view,
                        app.security.drilled,
                        app.security.searching,
                        input,
                    )
                    .is_some()
                {
                    let action = security_workspace_action(
                        app.security.view,
                        app.security.drilled,
                        app.security.searching,
                        input,
                    )
                    .expect("Security action was checked");
                    if let Some(effect) = update(&mut app, action) {
                        let _ = route_independent_security_effect(
                            &guard,
                            &mut app,
                            &mut security_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if app.screen == Screen::Qa
                    && qa_workspace_action(app.qa.view, app.qa.drilled, app.qa.searching, input)
                        .is_some()
                {
                    let action =
                        qa_workspace_action(app.qa.view, app.qa.drilled, app.qa.searching, input)
                            .expect("QA action was checked");
                    if let Some(effect) = update(&mut app, action) {
                        let _ = route_independent_qa_effect(
                            &guard,
                            &mut app,
                            &mut qa_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if app.screen == Screen::Maintenance
                    && maintenance_workspace_action(
                        app.maintenance.view,
                        maintenance_row_count(&app),
                        input,
                    )
                    .is_some()
                {
                    let action = maintenance_workspace_action(
                        app.maintenance.view,
                        maintenance_row_count(&app),
                        input,
                    )
                    .expect("Maintenance action was checked");
                    if let Some(effect) = update(&mut app, action) {
                        let _ = route_independent_maintenance_effect(
                            &guard,
                            &mut app,
                            &mut maintenance_coordinator,
                            effect,
                            editor.as_deref(),
                        )
                        .await;
                    }
                } else if app.screen == Screen::BuildEnvironment
                    && app
                        .build_environment_draft
                        .as_ref()
                        .is_some_and(|draft| draft.editing)
                {
                    let action = match input {
                        Input::Up => Some(Action::SelectBuildEnvironmentField { delta: -1 }),
                        Input::Down => Some(Action::SelectBuildEnvironmentField { delta: 1 }),
                        Input::Enter => Some(Action::ApplyBuildEnvironmentProfile),
                        Input::Esc => Some(Action::CancelBuildEnvironmentEdit),
                        Input::Backspace => Some(Action::BackspaceBuildEnvironmentField),
                        Input::Char(c) => Some(Action::AppendBuildEnvironmentField(c)),
                        _ => None,
                    };
                    if let Some(action) = action {
                        let _ = update(&mut app, action);
                    }
                } else if app.screen == Screen::BuildEnvironment
                    && build_environment_action(input).is_some()
                {
                    let action = build_environment_action(input)
                        .expect("build environment action was checked");
                    if let Some(Effect::VerifyBuildEnvironment {
                        profile,
                        generation,
                    }) = update(&mut app, action)
                    {
                        match BuildEnvironmentAdapter::default().initialize(profile).await {
                            Ok(response) => {
                                let profile = response.profile.clone();
                                let _ = update(
                                    &mut app,
                                    Action::BuildEnvironmentVerified { generation },
                                );
                                let _ = backend.shutdown().await;
                                match select_backend_with_environment(
                                    backend_kind.clone(),
                                    profile.build_dir.clone(),
                                    Some(cancellation_timeout),
                                    Some(response.environment),
                                )
                                .await
                                {
                                    Ok(mut connected) => {
                                        match connected.inspect_workspace().await {
                                            Ok(workspace) => {
                                                let _ = update(
                                                    &mut app,
                                                    Action::WorkspaceLoaded(workspace),
                                                );
                                                backend = connected;
                                            }
                                            Err(error) => {
                                                app.notification = Some(format!(
                                                    "BitBake verification failed: {error}"
                                                ))
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        app.notification =
                                            Some(format!("Could not start BitBake: {error}"))
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = update(
                                    &mut app,
                                    Action::BuildEnvironmentVerificationFailed {
                                        generation,
                                        message: error.to_string(),
                                    },
                                );
                            }
                        }
                    }
                } else if app.screen == Screen::Settings && settings_action(input).is_some() {
                    if app.settings_selection == 0 && matches!(input, Input::Enter | Input::Right) {
                        let _ = update(&mut app, Action::OpenThemePicker);
                        continue;
                    }
                    let action = settings_action(input).expect("settings action was checked");
                    match update(&mut app, action) {
                        Some(Effect::PersistSettings) => {
                            let result =
                                persist_settings(session_path.as_deref(), &mut session, &app);
                            let persistence_action = match result {
                                Ok(()) => Action::SettingsPersisted,
                                Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                            };
                            let _ = update(&mut app, persistence_action);
                        }
                        Some(Effect::VerifyBuildEnvironment {
                            profile,
                            generation,
                        }) => match BuildEnvironmentAdapter::default().initialize(profile).await {
                            Ok(response) => {
                                let profile = response.profile.clone();
                                let _ = update(
                                    &mut app,
                                    Action::BuildEnvironmentVerified { generation },
                                );
                                let _ = backend.shutdown().await;
                                match select_backend_with_environment(
                                    backend_kind.clone(),
                                    profile.build_dir,
                                    Some(cancellation_timeout),
                                    Some(response.environment),
                                )
                                .await
                                {
                                    Ok(mut connected) => {
                                        match connected.inspect_workspace().await {
                                            Ok(workspace) => {
                                                let _ = update(
                                                    &mut app,
                                                    Action::WorkspaceLoaded(workspace),
                                                );
                                                backend = connected;
                                            }
                                            Err(error) => {
                                                app.notification = Some(format!(
                                                    "BitBake verification failed: {error}"
                                                ))
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        app.notification =
                                            Some(format!("Could not start BitBake: {error}"))
                                    }
                                }
                            }
                            Err(error) => {
                                let _ = update(
                                    &mut app,
                                    Action::BuildEnvironmentVerificationFailed {
                                        generation,
                                        message: error.to_string(),
                                    },
                                );
                            }
                        },
                        _ => {}
                    }
                } else if app.screen == Screen::Tasks
                    && tasks_action(app.task_filter_editing, input).is_some()
                {
                    let action = tasks_action(app.task_filter_editing, input)
                        .expect("Tasks action was checked");
                    let _ = update(&mut app, action);
                } else if app.screen == Screen::Logs
                    && logs_action(app.logs.searching, input).is_some()
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
                } else if app.screen == Screen::Dependencies
                    && dependency_workspace_action(input).is_some()
                {
                    let action =
                        dependency_workspace_action(input).expect("Dependency action was checked");
                    match update(&mut app, action) {
                        Some(Effect::GetDependencies(recipe)) => {
                            load_dependency_graph(&mut app, backend.as_mut(), recipe).await;
                        }
                        Some(Effect::OpenInEditor(path)) => {
                            open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                        }
                        _ => {}
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
                } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('Z') {
                    let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
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
                        load_dependency_graph(&mut app, backend.as_mut(), recipe).await;
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
                } else if app.screen == yoctui_model::Screen::Configuration && input == Input::Enter
                {
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
                    if let Some(Effect::CopyToClipboard(content)) =
                        config_copy_effect(&mut app, input)
                    {
                        copy_to_clipboard(&mut app, content).await;
                    }
                } else if app.screen == yoctui_model::Screen::Configuration
                    && input == Input::Char('o')
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
                                for action in build_jobs
                                    .cancellation_failed(error.to_string(), SystemTime::now())
                                {
                                    let _ = update(&mut app, action);
                                }
                            }
                        }
                    } else {
                        if let Some(effect) = update(&mut app, action)
                            && !route_independent_security_effect(
                                &guard,
                                &mut app,
                                &mut security_coordinator,
                                effect.clone(),
                                editor.as_deref(),
                            )
                            .await
                            && !route_independent_qa_effect(
                                &guard,
                                &mut app,
                                &mut qa_coordinator,
                                effect.clone(),
                                editor.as_deref(),
                            )
                            .await
                            && !route_independent_maintenance_effect(
                                &guard,
                                &mut app,
                                &mut maintenance_coordinator,
                                effect.clone(),
                                editor.as_deref(),
                            )
                            .await
                        {
                            let _ = test_coordinator.handle_effect(&mut app, effect).await;
                        }
                    }
                }
            }
        }
        let completed_devtool =
            poll_devtool_job(&mut app, &mut devtool_jobs, &mut devtool_runner).await;
        match completed_devtool {
            Some(DevtoolOperation::Modify { recipe })
                if pending_devtool_modify
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_modify.take() {
                    complete_devtool_modify(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::UpdateRecipe { recipe })
                if pending_devtool_update
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_update.take() {
                    complete_devtool_update(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::Finish { recipe, .. })
                if pending_devtool_finish
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_finish.take() {
                    complete_devtool_finish(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::DeployTarget { recipe, .. })
                if pending_devtool_deploy
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_deploy.take() {
                    complete_devtool_deploy(&mut app, &session_build_dir, identity).await;
                }
            }
            Some(DevtoolOperation::Reset { recipe })
                if pending_devtool_reset
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = pending_devtool_reset.take() {
                    complete_devtool_reset(&mut app, &session_build_dir, identity).await;
                }
            }
            _ if devtool_jobs.active_operation().is_none() => {
                pending_devtool_modify = None;
                pending_devtool_update = None;
                pending_devtool_finish = None;
                pending_devtool_deploy = None;
                pending_devtool_reset = None;
            }
            _ => {}
        }
        match tokio::time::timeout(Duration::from_millis(1), backend.next_event()).await {
            Ok(Ok(event)) => {
                let test_terminal = matches!(
                    event,
                    BackendEvent::BuildCompleted { .. }
                        | BackendEvent::CommandFailed { .. }
                        | BackendEvent::Disconnected
                );
                let security_terminal = test_terminal;
                let qa_terminal = test_terminal;
                if let Some(id) = pending_test_build
                    && let Some(action) = test_build_action_for_event(&app, id, &event)
                {
                    let _ = update(&mut app, action);
                }
                let security_followup = pending_security_build
                    .and_then(|id| security_build_action_for_event(&app, id, &event))
                    .and_then(|action| update(&mut app, action));
                let qa_followup = pending_qa_build
                    .and_then(|id| qa_build_action_for_event(&app, id, &event))
                    .and_then(|action| update(&mut app, action));
                let sdk_refresh =
                    sdk_refresh_after_build_event(&mut app, &mut pending_sdk_build, &event);
                for action in build_jobs.actions_for_backend_event(event, SystemTime::now()) {
                    let _ = update(&mut app, action);
                }
                if test_terminal {
                    pending_test_build = None;
                }
                if security_terminal {
                    pending_security_build = None;
                }
                if qa_terminal {
                    pending_qa_build = None;
                }
                if let Some(effect) = security_followup {
                    let _ = security_coordinator.handle_effect(&mut app, effect).await;
                }
                if let Some(effect) = qa_followup {
                    let _ = qa_coordinator.handle_effect(&mut app, effect).await;
                }
                if let Some(effect) = sdk_refresh {
                    begin_sdk_artifact_operation(
                        &mut app,
                        sdk_artifact_adapter.as_ref(),
                        &mut sdk_artifact_operation,
                        effect,
                    );
                }
            }
            Ok(Err(error)) => {
                if let Some(id) = pending_test_build.take() {
                    let _ = update(
                        &mut app,
                        Action::LoseTestSession {
                            id,
                            message: error.to_string(),
                            finished_at: SystemTime::now(),
                        },
                    );
                }
                if let Some(id) = pending_security_build.take() {
                    let _ = update(
                        &mut app,
                        Action::Security(SecurityAction::LoseSession {
                            id,
                            message: error.to_string(),
                            finished_at: SystemTime::now(),
                        }),
                    );
                }
                if let Some(id) = pending_qa_build.take() {
                    let _ = update(
                        &mut app,
                        Action::Qa(QaAction::LoseSession {
                            session: id,
                            message: error.to_string(),
                            finished_at: SystemTime::now(),
                        }),
                    );
                }
                pending_sdk_build = None;
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
    if let Some(operation) = signature_operation.take() {
        operation.cancellation.cancel();
        let _ = operation.handle.await;
    }
    if let Some(operation) = image_artifact_operation.take() {
        operation.cancellation.cancel();
        let _ = operation.handle.await;
    }
    if let Some(operation) = sdk_artifact_operation.take() {
        operation.cancellation.cancel();
        let _ = operation.handle.await;
    }
    if let Some(operation) = sdk_capability_operation.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(mut operation) = sdk_operation.take() {
        if let Some(handle) = operation.starting.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(handle) = operation.timeout_wait.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(handle) = operation.cancellation.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(mut runner) = operation.runner.take() {
            let _ = runner.cancel().await;
        }
    }
    if let Some(mut operation) = qemu_operation.take() {
        if let Some(handle) = operation.cancellation.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(mut runner) = operation.runner.take() {
            let _ = runner.cancel().await;
        }
    }
    if let Some(operation) = wic_capability_operation.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(mut operation) = wic_operation.take() {
        if let Some(handle) = operation.cancellation.take() {
            handle.abort();
            let _ = handle.await;
        } else if let Some(mut runner) = operation.runner.take() {
            let _ = runner.cancel().await;
        }
    }
    if let Some(mut operation) = test_coordinator.session.take() {
        let _ = operation.runner.cancel().await;
    }
    if let Some(mut operation) = test_coordinator.result.take() {
        let _ = operation.runner.cancel().await;
    }
    if let Some(operation) = test_coordinator.import.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(operation) = security_coordinator.capability.take() {
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(operation) = security_coordinator.report.take() {
        operation.cancellation.cancel();
        operation.handle.abort();
        let _ = operation.handle.await;
    }
    if let Some(mut operation) = security_coordinator.mapper.take() {
        let _ = operation.runner.cancel(operation.id).await;
    }
    maintenance_coordinator.shutdown().await;
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
        KeyCode::Char('v') if key.modifiers.contains(KeyModifiers::CONTROL) => Some(Input::CtrlV),
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
        KeyCode::Home => Some(Input::Home),
        KeyCode::End => Some(Input::End),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn write_test_executable(path: &Path, body: &str) {
        use std::{
            os::unix::fs::PermissionsExt,
            sync::atomic::{AtomicU64, Ordering},
        };

        static NEXT_EXECUTABLE: AtomicU64 = AtomicU64::new(1);

        let temporary = path.with_extension(format!(
            "yoctui-fixture-write-{}-{}",
            std::process::id(),
            NEXT_EXECUTABLE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&temporary, body).unwrap();
        let mut permissions = fs::metadata(&temporary).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&temporary, permissions).unwrap();
        fs::rename(temporary, path).unwrap();
    }

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
    fn no_build_directory_resolves_to_unconfigured_startup() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-no-build-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        let config_path = directory.join("config.toml");
        fs::write(&config_path, "").unwrap();
        let cli =
            Cli::try_parse_from(["yoctui", "--config", config_path.to_str().unwrap()]).unwrap();
        let mut session = Session::default();
        session
            .recent_build_dirs
            .push(std::env::current_dir().unwrap());
        let resolved = resolve_config(&cli, &session).unwrap();
        assert!(!resolved.build_dir_configured);
        assert_eq!(resolved.build_dir, PathBuf::from("/"));
        fs::remove_file(config_path).unwrap();
        fs::remove_dir(directory).unwrap();
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
            theme: Some(Theme::DarkPro),
            ..Session::default()
        };
        let mut app = App::new(10, 1_000);
        app.theme = Theme::WhiteClassic;

        assert!(persist_settings(Some(&path), &mut session, &app).is_err());
        assert_eq!(session.theme, Some(Theme::DarkPro));

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
    fn popup_editor_keys_decode_home_end_and_paste() {
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
            Some(Input::Home)
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
            Some(Input::End)
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)),
            Some(Input::CtrlV)
        );
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
    fn dependency_workspace_terminal_keys_decode_to_typed_routes() {
        for (code, expected) in [
            (KeyCode::Up, Action::SelectDependencyGraphNode { delta: -1 }),
            (
                KeyCode::Down,
                Action::SelectDependencyGraphNode { delta: 1 },
            ),
            (KeyCode::Enter, Action::OpenSelectedDependencyRecipe),
            (KeyCode::Char('o'), Action::OpenSelectedDependencyProvider),
            (KeyCode::Char('L'), Action::OpenSelectedDependencyTaskLog),
            (KeyCode::Char('r'), Action::RefreshDependencyGraph),
        ] {
            let input = input_from_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
            assert_eq!(dependency_workspace_action(input), Some(expected));
        }
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

    #[tokio::test]
    async fn devtool_publish_update_refreshes_original_identity_and_retains_failure() {
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        let original = yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: yoctui_model::DevtoolGitState::NotRepository,
            error: None,
        };
        let operation = DevtoolOperation::UpdateRecipe {
            recipe: identity.name.clone(),
        };
        let command = DevtoolCommandSpec::with_executable("/bin/false".into(), &operation).unwrap();
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dashboard;
        app.devtool_statuses
            .insert(identity.clone(), original.clone());
        let mut coordinator = DevtoolJobCoordinator::default();
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
        tokio::time::timeout(Duration::from_secs(2), async {
            while runner.is_some() {
                assert!(
                    poll_devtool_job(&mut app, &mut coordinator, &mut runner)
                        .await
                        .is_none()
                );
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
        assert_eq!(app.devtool_statuses.get(&identity), Some(&original));

        let refreshed = yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: yoctui_model::DevtoolGitState::Available {
                branch: Some("devtool".into()),
                head: Some("abc123".into()),
                modified: 0,
                untracked: 0,
                conflicted: 0,
            },
            error: None,
        };
        apply_completed_devtool_update_status(&mut app, refreshed.clone());
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status was refreshed"))
        );
        apply_completed_devtool_update_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity: identity.clone(),
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                    exit_code: Some(7),
                    message: "refresh failed".into(),
                }),
            },
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status refresh failed"))
        );
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
    }

    #[tokio::test]
    async fn devtool_publish_finish_refreshes_original_identity_and_retains_job_context() {
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dashboard;
        let id = yoctui_model::BackgroundJobId((1_u64 << 63) + 100);
        let _ = update(
            &mut app,
            Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
                id,
                kind: yoctui_model::BackgroundJobKind::Devtool,
                title: "Devtool finish busybox".into(),
                context: yoctui_model::BackgroundJobContext {
                    recipe: Some("busybox".into()),
                    path: Some(PathBuf::from("/layers/meta-demo")),
                    ..yoctui_model::BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::UNIX_EPOCH,
            }),
        );
        let _ = update(
            &mut app,
            Action::StartBackgroundJob {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::RunBackgroundJob { id });
        let _ = update(
            &mut app,
            Action::SucceedBackgroundJob {
                id,
                result: yoctui_model::BackgroundJobResult {
                    summary: "Devtool completed successfully".into(),
                    artifacts: vec![],
                },
                finished_at: SystemTime::UNIX_EPOCH,
            },
        );
        let refreshed = yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: None,
        };
        apply_completed_devtool_finish_status(&mut app, refreshed.clone());
        assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
        assert_eq!(app.screen, Screen::Dashboard);
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status was refreshed"))
        );
        apply_completed_devtool_finish_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity: identity.clone(),
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                    exit_code: Some(5),
                    message: "refresh failed".into(),
                }),
            },
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status refresh failed"))
        );
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Succeeded
        );

        let operation = DevtoolOperation::Finish {
            recipe: "busybox".into(),
            destination: PathBuf::from("/layers/meta-demo"),
        };
        let command = DevtoolCommandSpec::with_executable("/bin/false".into(), &operation).unwrap();
        let mut coordinator = DevtoolJobCoordinator::default();
        for action in coordinator
            .queue(operation, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = update(&mut app, action);
        }
        let failed_id = coordinator.active_job_id().unwrap();
        let mut started = DevtoolJobRunner::new(std::env::temp_dir());
        started.start(command).await.unwrap();
        let mut runner = Some(started);
        tokio::time::timeout(Duration::from_secs(2), async {
            while runner.is_some() {
                assert!(
                    poll_devtool_job(&mut app, &mut coordinator, &mut runner)
                        .await
                        .is_none()
                );
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
    }

    #[tokio::test]
    async fn devtool_target_deploy_refreshes_original_identity_and_retains_failures() {
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        let original = yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: yoctui_model::DevtoolGitState::NotRepository,
            error: None,
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dashboard;
        app.devtool_statuses
            .insert(identity.clone(), original.clone());
        let operation = DevtoolOperation::DeployTarget {
            recipe: identity.name.clone(),
            target: "qemuarm".into(),
        };
        let command = DevtoolCommandSpec::with_executable("/bin/false".into(), &operation).unwrap();
        let mut coordinator = DevtoolJobCoordinator::default();
        for action in coordinator
            .queue(operation, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = update(&mut app, action);
        }
        let failed_id = coordinator.active_job_id().unwrap();
        let mut started = DevtoolJobRunner::new(std::env::temp_dir());
        started.start(command).await.unwrap();
        let mut runner = Some(started);
        tokio::time::timeout(Duration::from_secs(2), async {
            while runner.is_some() {
                assert!(
                    poll_devtool_job(&mut app, &mut coordinator, &mut runner)
                        .await
                        .is_none()
                );
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
        assert_eq!(app.devtool_statuses.get(&identity), Some(&original));

        let refreshed = yoctui_model::DevtoolStatus {
            git: yoctui_model::DevtoolGitState::Available {
                branch: Some("devtool".into()),
                head: Some("abc123".into()),
                modified: 0,
                untracked: 0,
                conflicted: 0,
            },
            ..original
        };
        apply_completed_devtool_deploy_status(&mut app, refreshed.clone());
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status was refreshed"))
        );
        apply_completed_devtool_deploy_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity,
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                    exit_code: Some(9),
                    message: "refresh failed".into(),
                }),
            },
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status refresh failed"))
        );
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
    }

    #[tokio::test]
    async fn devtool_target_reset_refreshes_expected_removal_and_retains_failures() {
        let identity = RecipeIdentity {
            name: "busybox".into(),
            file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
        };
        let original = yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: yoctui_model::DevtoolGitState::NotRepository,
            error: None,
        };
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Dashboard;
        app.devtool_statuses
            .insert(identity.clone(), original.clone());
        let operation = DevtoolOperation::Reset {
            recipe: identity.name.clone(),
        };
        let command = DevtoolCommandSpec::with_executable("/bin/false".into(), &operation).unwrap();
        let mut coordinator = DevtoolJobCoordinator::default();
        for action in coordinator
            .queue(operation, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = update(&mut app, action);
        }
        let failed_id = coordinator.active_job_id().unwrap();
        let mut started = DevtoolJobRunner::new(std::env::temp_dir());
        started.start(command).await.unwrap();
        let mut runner = Some(started);
        tokio::time::timeout(Duration::from_secs(2), async {
            while runner.is_some() {
                assert!(
                    poll_devtool_job(&mut app, &mut coordinator, &mut runner)
                        .await
                        .is_none()
                );
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
        assert_eq!(app.devtool_statuses.get(&identity), Some(&original));

        let refreshed = yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: None,
        };
        apply_completed_devtool_reset_status(&mut app, refreshed.clone());
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("no longer in the workspace"))
        );
        apply_completed_devtool_reset_status(
            &mut app,
            yoctui_model::DevtoolStatus {
                identity,
                capability: yoctui_model::DevtoolCapability::Available,
                workspace: DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                    exit_code: Some(4),
                    message: "refresh failed".into(),
                }),
            },
        );
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("status refresh failed"))
        );
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            yoctui_model::BackgroundJobStatus::Failed
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn signature_workspace_background_operation_reports_success_failure_and_cancellation() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "yoctui-signature-workspace-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let stamps = directory.join("tmp/stamps/qemux86_64/busybox");
        fs::create_dir_all(&stamps).unwrap();
        fs::write(
            stamps.join("1.0.do_compile.sigdata.aaa"),
            "fixture artifact",
        )
        .unwrap();
        let dump = directory.join("bitbake-dumpsig");
        let diff = directory.join("bitbake-diffsigs");
        let write_tool = |path: &Path, body: &str| {
            fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
            let mut permissions = fs::metadata(path).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(path, permissions).unwrap();
        };
        write_tool(
            &dump,
            "printf '%s\\n' 'basehash_ignore_vars: []' 'taskhash_ignore_tasks: []' 'Task dependencies: []' 'basehash: base-aaa' 'Variable CC value is gcc' 'Tasks this task depends on: []' 'Computed base hash is base-aaa and from file base-aaa' 'Computed task hash is aaa'",
        );
        write_tool(&diff, "exit 0");
        let adapter =
            SignatureAdapter::with_programs(directory.clone(), dump.clone(), diff.clone());
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let mut app = App::new(10, 1_000);
        let effect = update(&mut app, Action::BeginSignatureDump(target.clone())).unwrap();
        let mut operation = None;
        begin_signature_operation(&mut app, &adapter, &mut operation, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_signature_operation(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.signature_dump,
            yoctui_model::SignatureDumpState::Available { .. }
        ));

        write_tool(&dump, "printf 'bad signature\\n' >&2\nexit 7");
        let effect = update(&mut app, Action::RefreshSignatureDump).unwrap();
        begin_signature_operation(&mut app, &adapter, &mut operation, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_signature_operation(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.signature_dump,
            yoctui_model::SignatureDumpState::Failed { ref message, .. }
                if message.contains("bad signature")
        ));

        app.notification = None;
        write_tool(&dump, "sleep 30");
        let effect = update(&mut app, Action::BeginSignatureDump(target)).unwrap();
        begin_signature_operation(&mut app, &adapter, &mut operation, effect);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            operation
                .as_ref()
                .is_some_and(|operation| operation.cancellation.cancel())
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_signature_operation(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.signature_dump,
            yoctui_model::SignatureDumpState::Failed { ref message, .. }
                if message.contains("cancelled")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn pkgdata_workspace_background_operation_reports_inventory_detail_and_cancellation() {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "yoctui-pkgdata-workspace-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let build_dir = directory.join("build");
        let pkgdata_dir = build_dir.join("tmp/pkgdata");
        fs::create_dir_all(&pkgdata_dir).unwrap();
        let tool = directory.join("oe-pkgdata-util");
        let write_tool = |body: &str| {
            fs::write(&tool, format!("#!/bin/sh\n{body}\n")).unwrap();
            let mut permissions = fs::metadata(&tool).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(&tool, permissions).unwrap();
        };
        write_tool(
            r#"case "$3" in
list-pkgs) printf 'busybox\nlibc6\n' ;;
package-info) printf 'busybox 1.37.0-r0 busybox 1.37.0-r0 1024 "GPL-2.0-only"\nlibc6 2.40-r0 glibc 2.40-r0 4096 "GPL-2.0-or-later"\n' ;;
list-pkg-files) printf 'busybox:\n\t/bin/busybox\n' ;;
read-value) printf 'busybox libc6\nlibc6\n' ;;
*) exit 9 ;;
esac"#,
        );
        let adapter = PackageDataAdapter::with_paths(build_dir, tool.clone(), pkgdata_dir);
        let mut app = App::new(10, 1_000);
        app.screen = Screen::Packages;
        let effect = update(&mut app, Action::BeginPackageInventory).unwrap();
        let mut operation = None;
        begin_package_operation(&mut app, &adapter, &mut operation, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_package_operation(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.package_inventory,
            yoctui_model::PackageInventoryState::Partial { .. }
        ));
        assert_eq!(
            app.package_selection,
            Some(yoctui_model::PackageIdentity::new("busybox"))
        );

        let effect = update(&mut app, Action::BeginSelectedPackageDetail).unwrap();
        begin_package_operation(&mut app, &adapter, &mut operation, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_package_operation(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.selected_package_detail(),
            Some(yoctui_model::PackageDetailState::Available { .. })
        ));

        write_tool("sleep 30");
        let effect = update(&mut app, Action::RefreshPackageInventory).unwrap();
        begin_package_operation(&mut app, &adapter, &mut operation, effect);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            operation
                .as_ref()
                .is_some_and(|operation| operation.cancellation.cancel())
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_package_operation(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.package_inventory,
            yoctui_model::PackageInventoryState::Failed { ref message, .. }
                if message.contains("cancelled")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn images_workspace_background_operation_reports_success_failure_and_cancellation() {
        let directory = std::env::temp_dir().join(format!(
            "yoctui-images-workspace-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let deploy = directory.join("qemux86-64");
        fs::create_dir_all(&deploy).unwrap();
        fs::write(
            deploy.join("core-image-minimal-qemux86-64.rootfs.ext4"),
            b"image",
        )
        .unwrap();
        let adapter = ImageArtifactAdapter::new(deploy);
        let qemu_inspector =
            QemuCapabilityInspector::with_executable(directory.join("missing-runqemu"));
        let wic_inspector = WicCapabilityInspector::with_executable(directory.join("missing-wic"));
        let mut wic_capability_operation = None;
        let mut app = App::new(10, 1_000);
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        let effect = update(&mut app, Action::BeginImageArtifactInventory).unwrap();
        let mut operation = None;
        begin_image_artifact_operation(&mut app, Some(&adapter), &mut operation, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_image_artifact_operation(
                    &mut app,
                    &mut operation,
                    &qemu_inspector,
                    &wic_inspector,
                    &mut wic_capability_operation,
                )
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.image_artifacts,
            yoctui_model::ImageArtifactInventoryState::Available { .. }
        ));

        let effect = update(&mut app, Action::RefreshImageArtifactInventory).unwrap();
        begin_image_artifact_operation(&mut app, None, &mut operation, effect);
        assert!(matches!(
            app.image_artifacts,
            yoctui_model::ImageArtifactInventoryState::Failed { ref message, .. }
                if message.contains("DEPLOY_DIR_IMAGE")
        ));

        let effect = update(&mut app, Action::RefreshImageArtifactInventory).unwrap();
        begin_image_artifact_operation(&mut app, Some(&adapter), &mut operation, effect);
        assert!(
            operation
                .as_ref()
                .is_some_and(|operation| operation.cancellation.cancel())
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_image_artifact_operation(
                    &mut app,
                    &mut operation,
                    &qemu_inspector,
                    &wic_inspector,
                    &mut wic_capability_operation,
                )
                .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.image_artifacts,
            yoctui_model::ImageArtifactInventoryState::Failed { ref message, .. }
                if message.contains("cancelled")
        ));
        if let Some(operation) = wic_capability_operation {
            operation.handle.abort();
        }
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    fn qemu_workspace_fixture(name: &str, body: &str) -> (PathBuf, PathBuf, App) {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "yoctui-qemu-workspace-{}-{name}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let build_dir = directory.join("build");
        let deploy = directory.join("qemux86-64");
        fs::create_dir_all(&build_dir).unwrap();
        fs::create_dir_all(&deploy).unwrap();
        let executable = directory.join("runqemu");
        fs::write(&executable, format!("#!/bin/sh\n{body}\n")).unwrap();
        let mut permissions = fs::metadata(&executable).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&executable, permissions).unwrap();
        let image_path = deploy.join("core-image-minimal.wic");
        fs::write(&image_path, b"wic").unwrap();
        let identity = yoctui_model::ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: image_path,
        };
        let artifact = yoctui_model::ImageArtifact {
            identity: identity.clone(),
            kind: yoctui_model::ImageArtifactKind::Wic,
            size_bytes: yoctui_model::ImageArtifactField::Available(3),
            modified_unix_seconds: yoctui_model::ImageArtifactField::Unavailable,
            checksums: yoctui_model::ImageArtifactField::Unavailable,
            manifests: yoctui_model::ImageArtifactField::Unavailable,
            licenses: yoctui_model::ImageArtifactField::Unavailable,
            spdx: yoctui_model::ImageArtifactField::Unavailable,
            wic_files: yoctui_model::ImageArtifactField::Unavailable,
        };
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.image_artifact_selection = Some(identity);
        app.image_artifacts = yoctui_model::ImageArtifactInventoryState::Available {
            request: ImageArtifactRequest {
                generation: 1,
                machine: "qemux86-64".into(),
            },
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: yoctui_model::ImageArtifactField::Available(deploy),
                artifacts: vec![artifact],
            },
        };
        execute_qemu_capability_effect(
            &mut app,
            &QemuCapabilityInspector::with_executable(executable),
            Effect::InspectQemuCapability,
        );
        (directory, build_dir, app)
    }

    fn qemu_workspace_start_effect(app: &mut App) -> (QemuSessionId, QemuLaunchRequest) {
        let _ = update(app, Action::BeginSelectedQemuLaunch);
        let _ = update(app, Action::PreviewQemuLaunch);
        let Some(Effect::StartQemuSession { id, request }) = update(app, Action::ConfirmQemuLaunch)
        else {
            panic!("expected QEMU start effect");
        };
        (id, request)
    }

    async fn poll_qemu_until(
        app: &mut App,
        operation: &mut Option<QemuCliOperation>,
        condition: impl Fn(&App, &Option<QemuCliOperation>) -> bool,
    ) {
        let result = tokio::time::timeout(Duration::from_secs(30), async {
            while !condition(app, operation) {
                poll_qemu_job(app, operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await;
        assert!(
            result.is_ok(),
            "timed out waiting for QEMU CLI state; operation active: {}",
            operation.is_some()
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_workspace_cli_refreshes_capability_and_runs_exact_request_across_navigation() {
        let (directory, build_dir, mut app) =
            qemu_workspace_fixture("success", "printf '%s\\n' \"$@\"; exit 0");
        assert!(matches!(
            app.qemu_capability,
            QemuCapability::Available { .. }
        ));
        assert_eq!(
            qemu_launch_dialog_action(false, Input::Char('Q')),
            None,
            "modal Q input must not leak to the Images workspace"
        );
        let (id, request) = qemu_workspace_start_effect(&mut app);
        let mut operation = None;
        begin_qemu_job(
            &mut app,
            &mut operation,
            &build_dir,
            Duration::from_millis(100),
            id,
            request.clone(),
        )
        .await;
        app.screen = Screen::Logs;
        poll_qemu_until(&mut app, &mut operation, |_, operation| operation.is_none()).await;
        let session = app.qemu_session(id).unwrap();
        let job = app.background_jobs.get(session.background_job_id).unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert_eq!(app.screen, Screen::Logs);
        assert!(
            job.output
                .iter()
                .any(|entry| entry.message == request.image.path.display().to_string())
        );
        assert!(
            job.output
                .iter()
                .any(|entry| entry.message == "qemumemory=1024")
        );

        let effect = update(&mut app, Action::RefreshImageArtifactInventory).unwrap();
        let mut scan = None;
        begin_image_artifact_operation(&mut app, None, &mut scan, effect);
        assert!(matches!(app.qemu_capability, QemuCapability::Failed { .. }));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn qemu_workspace_cli_reports_nonzero_rejection_forced_cancel_and_loss() {
        let (failed_directory, build_dir, mut failed) =
            qemu_workspace_fixture("failure", "printf 'failed\\n' >&2; exit 9");
        let (failed_id, failed_request) = qemu_workspace_start_effect(&mut failed);
        let mut failed_operation = None;
        begin_qemu_job(
            &mut failed,
            &mut failed_operation,
            &build_dir,
            Duration::from_millis(100),
            failed_id,
            failed_request,
        )
        .await;
        poll_qemu_until(&mut failed, &mut failed_operation, |_, operation| {
            operation.is_none()
        })
        .await;
        let failed_job = failed
            .background_jobs
            .get(failed.qemu_session(failed_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(failed_job.status, yoctui_model::BackgroundJobStatus::Failed);
        assert_eq!(failed.qemu_session(failed_id).unwrap().exit_code, Some(9));
        fs::remove_dir_all(failed_directory).unwrap();

        let (cancel_directory, cancel_build_dir, mut cancelled) = qemu_workspace_fixture(
            "cancel",
            "trap '' TERM; printf 'ready\\n'; while :; do :; done",
        );
        let (cancel_id, cancel_request) = qemu_workspace_start_effect(&mut cancelled);
        let mut cancel_operation = None;
        begin_qemu_job(
            &mut cancelled,
            &mut cancel_operation,
            &cancel_build_dir,
            Duration::from_millis(100),
            cancel_id,
            cancel_request,
        )
        .await;
        poll_qemu_until(&mut cancelled, &mut cancel_operation, |cancelled, _| {
            cancelled
                .qemu_session(cancel_id)
                .and_then(|session| cancelled.background_jobs.get(session.background_job_id))
                .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"))
        })
        .await;
        let _ = update(
            &mut cancelled,
            Action::BeginQemuSessionCancellation { id: cancel_id },
        );
        let Some(Effect::CancelQemuSession(effect_id)) =
            update(&mut cancelled, Action::ConfirmQemuSessionCancellation)
        else {
            panic!("cancel effect");
        };
        begin_qemu_cancellation(&mut cancelled, &mut cancel_operation, effect_id);
        poll_qemu_until(&mut cancelled, &mut cancel_operation, |_, operation| {
            operation.is_none()
        })
        .await;
        let cancel_job = cancelled
            .background_jobs
            .get(cancelled.qemu_session(cancel_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(
            cancel_job.status,
            yoctui_model::BackgroundJobStatus::Cancelled
        );
        assert!(
            cancel_job
                .output
                .iter()
                .any(|entry| entry.message.contains("forced termination"))
        );

        let (reject_directory, _, mut rejected) = qemu_workspace_fixture("reject", "sleep 30");
        let (reject_id, _) = qemu_workspace_start_effect(&mut rejected);
        let _ = update(
            &mut rejected,
            Action::QemuSessionStarting {
                id: reject_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut rejected, Action::QemuSessionRunning { id: reject_id });
        let _ = update(
            &mut rejected,
            Action::BeginQemuSessionCancellation { id: reject_id },
        );
        let Some(Effect::CancelQemuSession(reject_effect_id)) =
            update(&mut rejected, Action::ConfirmQemuSessionCancellation)
        else {
            panic!("reject effect");
        };
        let mut no_operation = None;
        begin_qemu_cancellation(&mut rejected, &mut no_operation, reject_effect_id);
        let reject_job = rejected
            .background_jobs
            .get(rejected.qemu_session(reject_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(
            reject_job.status,
            yoctui_model::BackgroundJobStatus::Running
        );
        fs::remove_dir_all(reject_directory).unwrap();

        let (lost_directory, lost_build_dir, mut lost) = qemu_workspace_fixture("lost", "sleep 30");
        let (lost_id, lost_request) = qemu_workspace_start_effect(&mut lost);
        let mut lost_operation = None;
        begin_qemu_job(
            &mut lost,
            &mut lost_operation,
            &lost_build_dir,
            Duration::from_millis(100),
            lost_id,
            lost_request,
        )
        .await;
        poll_qemu_job(&mut lost, &mut lost_operation).await;
        poll_qemu_job(&mut lost, &mut lost_operation).await;
        let active = lost_operation.as_mut().unwrap();
        drop(active.runner.take());
        active.cancellation = Some(tokio::spawn(async {
            panic!("synthetic cancellation task loss");
        }));
        tokio::task::yield_now().await;
        poll_qemu_job(&mut lost, &mut lost_operation).await;
        let lost_job = lost
            .background_jobs
            .get(lost.qemu_session(lost_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(lost_job.status, yoctui_model::BackgroundJobStatus::Lost);
        fs::remove_dir_all(lost_directory).unwrap();
        fs::remove_dir_all(cancel_directory).unwrap();
    }

    #[cfg(unix)]
    async fn wic_workspace_fixture(name: &str, create_body: &str) -> (PathBuf, PathBuf, App) {
        let directory = std::env::temp_dir().join(format!(
            "yoctui-wic-workspace-{}-{name}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let build_dir = directory.join("build");
        let deploy = directory.join("deploy");
        fs::create_dir_all(&build_dir).unwrap();
        fs::create_dir_all(&deploy).unwrap();
        let executable = directory.join("wic");
        write_test_executable(
            &executable,
            &format!("#!/bin/sh\nif [ \"$1\" = \"list\" ]; then exit 0; fi\n{create_body}\n"),
        );
        let kickstart = directory.join("directdisk.wks");
        fs::write(
            &kickstart,
            "part / --source=rootfs --fstype=ext4 --size=64\n",
        )
        .unwrap();
        let image_path = deploy.join("core-image-minimal.ext4");
        fs::write(&image_path, b"rootfs").unwrap();
        let identity = yoctui_model::ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: image_path,
        };
        let artifact = yoctui_model::ImageArtifact {
            identity: identity.clone(),
            kind: yoctui_model::ImageArtifactKind::RootFilesystem,
            size_bytes: yoctui_model::ImageArtifactField::Available(6),
            modified_unix_seconds: yoctui_model::ImageArtifactField::Unavailable,
            checksums: yoctui_model::ImageArtifactField::Unavailable,
            manifests: yoctui_model::ImageArtifactField::Unavailable,
            licenses: yoctui_model::ImageArtifactField::Unavailable,
            spdx: yoctui_model::ImageArtifactField::Unavailable,
            wic_files: yoctui_model::ImageArtifactField::Unavailable,
        };
        let mut app = App::new(20, 20_000);
        app.screen = Screen::Images;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("WKS_FILE".into(), kickstart.display().to_string());
        app.image_artifact_selection = Some(identity);
        app.image_artifacts = ImageArtifactInventoryState::Available {
            request: ImageArtifactRequest {
                generation: 1,
                machine: "qemux86-64".into(),
            },
            inventory: yoctui_model::ImageArtifactInventory {
                machine: "qemux86-64".into(),
                deploy_directory: yoctui_model::ImageArtifactField::Available(deploy),
                artifacts: vec![artifact],
            },
        };
        let inspector = configure_wic_capability_inspector(
            &app,
            WicCapabilityInspector::with_executable(executable),
        );
        let effect = update(&mut app, Action::InspectWicCapability).unwrap();
        let mut capability_operation = None;
        begin_wic_capability_operation(&mut app, &inspector, &mut capability_operation, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while capability_operation.is_some() {
                poll_wic_capability_operation(&mut app, &inspector, &mut capability_operation)
                    .await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(
            matches!(app.wic_capability, WicCapability::Available { .. }),
            "{:?}",
            app.wic_capability
        );
        (directory, build_dir, app)
    }

    fn wic_workspace_start_effect(app: &mut App) -> (WicSessionId, WicOperation) {
        let _ = update(app, Action::BeginSelectedWicCreate);
        let _ = update(app, Action::PreviewWicCreate);
        let Some(Effect::StartWicSession { id, operation }) = update(app, Action::ConfirmWicCreate)
        else {
            panic!("expected Wic start effect");
        };
        (id, operation)
    }

    #[cfg(unix)]
    async fn wic_device_write_start_effect(
        app: &mut App,
        inspector: &WicDeviceInspector,
    ) -> (WicSessionId, WicOperation) {
        let effect = update(app, Action::BeginSelectedWicDeviceWrite)
            .expect("expected device discovery effect");
        let mut discovery = None;
        begin_wic_device_operation(inspector, &mut discovery, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while discovery.is_some() {
                poll_wic_device_operation(app, &mut discovery).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let _ = update(
            app,
            wic_device_picker_action(Input::Enter).expect("device picker action"),
        );
        for character in "WRITE /dev/sdz".chars() {
            let _ = update(
                app,
                wic_write_phrase_action(Input::Char(character)).expect("phrase action"),
            );
        }
        let _ = update(
            app,
            wic_write_phrase_action(Input::Enter).expect("phrase preview action"),
        );
        let effect = update(
            app,
            wic_write_confirmation_action(Input::Enter).expect("write confirmation action"),
        );
        let Some(Effect::StartWicSession { id, operation }) = effect else {
            panic!("expected Wic write start effect");
        };
        (id, operation)
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_device_write_cli_discovers_routes_and_revalidates_before_spawn() {
        let (directory, build_dir, mut app) =
            wic_workspace_fixture("device-write-cli", "printf 'write-started\\n'; exit 0").await;
        let image_path = directory.join("deploy/device-image.wic");
        fs::write(&image_path, b"image").unwrap();
        let image_path = fs::canonicalize(image_path).unwrap();
        let image = yoctui_model::WicOutput {
            identity: yoctui_model::WicOutputIdentity {
                path: image_path,
                size_bytes: 5,
                modified_unix_seconds: 7,
            },
            kind: yoctui_model::WicOutputKind::Wic,
        };
        app.wic_output_selection = Some(image.identity.clone());
        app.wic_outputs = yoctui_model::WicOutputInventoryState::Available {
            request: yoctui_model::WicOutputInventoryRequest {
                generation: 1,
                output_directory: directory.join("deploy"),
            },
            outputs: vec![image],
        };

        let lsblk = directory.join("lsblk");
        let inventory = r#"{"blockdevices":[{"path":"/dev/sda","type":"disk","maj:min":"8:0","size":8192,"model":"root","serial":"root-serial","tran":null,"rm":false,"ro":false,"mountpoints":[],"children":[{"path":"/dev/sda1","type":"part","maj:min":"8:1","size":4096,"model":null,"serial":null,"tran":null,"rm":false,"ro":false,"mountpoints":["/"],"children":[]}]},{"path":"/dev/sdz","type":"disk","maj:min":"8:240","size":16384,"model":"Protected USB","serial":"SERIAL-123","tran":"usb","rm":true,"ro":false,"mountpoints":[],"children":[]}]}"#;
        write_test_executable(&lsblk, &format!("#!/bin/sh\nprintf '%s' '{}'\n", inventory));
        let inspector = WicDeviceInspector::with_program(fs::canonicalize(&lsblk).unwrap())
            .without_device_node_validation_for_tests();

        let Some(effect) = update(&mut app, Action::BeginSelectedWicDeviceWrite) else {
            panic!("expected device discovery effect");
        };
        let Effect::GetWicDevices(request) = &effect else {
            panic!("expected device discovery effect");
        };
        let request = request.clone();
        let mut discovery = None;
        begin_wic_device_operation(&inspector, &mut discovery, effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while discovery.is_some() {
                poll_wic_device_operation(&mut app, &mut discovery).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let yoctui_model::WicDeviceInventoryState::Partial {
            request: discovered,
            devices,
            limitations,
        } = &app.wic_devices
        else {
            panic!("fake discovery must retain a partial inventory");
        };
        assert_eq!(discovered, &request);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].identity.path, Path::new("/dev/sdz"));
        assert!(
            limitations
                .iter()
                .any(|limitation| limitation.contains("/dev/sda")
                    && limitation.contains("root filesystem"))
        );

        let mut failed_app = app.clone();
        let _ = update(&mut failed_app, Action::CancelWicDevicePicker);
        let Some(failed_effect) = update(&mut failed_app, Action::BeginSelectedWicDeviceWrite)
        else {
            panic!("expected replacement discovery");
        };
        let Effect::GetWicDevices(failed_request) = &failed_effect else {
            panic!("expected replacement discovery");
        };
        let failed_request = failed_request.clone();
        let missing = WicDeviceInspector::with_program(directory.join("missing-lsblk"));
        begin_wic_device_operation(&missing, &mut discovery, failed_effect);
        tokio::time::timeout(Duration::from_secs(2), async {
            while discovery.is_some() {
                poll_wic_device_operation(&mut failed_app, &mut discovery).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            &failed_app.wic_devices,
            yoctui_model::WicDeviceInventoryState::Failed { request, message }
                if request == &failed_request && message.contains("unavailable")
        ));

        assert_eq!(
            wic_device_picker_action(Input::Char('D')),
            None,
            "modal input must not leak to the Images workspace"
        );
        let picker_action = wic_device_picker_action(Input::Enter).unwrap();
        let _ = update(&mut app, picker_action);
        for character in "WRITE /dev/sdz".chars() {
            let action = wic_write_phrase_action(Input::Char(character)).unwrap();
            let _ = update(&mut app, action);
        }
        let preview_action = wic_write_phrase_action(Input::Enter).unwrap();
        let _ = update(&mut app, preview_action);
        let confirm_action = wic_write_confirmation_action(Input::Enter).unwrap();
        let Some(Effect::StartWicSession { id, operation }) = update(&mut app, confirm_action)
        else {
            panic!("expected confirmed write effect");
        };
        assert!(matches!(&operation, WicOperation::Write(request)
            if request.image.path == directory.join("deploy/device-image.wic")
                && request.image.size_bytes == 5
                && request.device.path == Path::new("/dev/sdz")));
        let cancellation = wic_cancellation_confirmation_action(id, true, Input::Enter);
        assert_eq!(
            cancellation,
            Some(Action::ConfirmWicSessionCancellation {
                id,
                acknowledge_incomplete_device: true,
            })
        );

        let mut running = None;
        begin_wic_job(
            &mut app,
            &mut running,
            &inspector,
            &build_dir,
            Duration::from_millis(100),
            id,
            operation,
        )
        .await;
        assert!(running.is_some());
        tokio::time::timeout(Duration::from_secs(2), async {
            while running.is_some() {
                poll_wic_job(&mut app, &mut running).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let session = app.wic_session(id).unwrap();
        let job = app.background_jobs.get(session.background_job_id).unwrap();
        assert!(
            job.output
                .iter()
                .any(|entry| entry.message == "write-started")
        );
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert_eq!(session.exit_code, Some(0));

        let wic = match &app.wic_capability {
            WicCapability::Available { executable, .. } => executable.clone(),
            capability => panic!("unexpected Wic capability: {capability:?}"),
        };
        write_test_executable(
            &wic,
            "#!/bin/sh\ntrap 'exit 0' TERM\nprintf 'ready\\n'\nwhile :; do sleep 1; done\n",
        );
        let (cancel_id, cancel_request) = wic_device_write_start_effect(&mut app, &inspector).await;
        let mut cancel_operation = None;
        begin_wic_job(
            &mut app,
            &mut cancel_operation,
            &inspector,
            &build_dir,
            Duration::from_secs(2),
            cancel_id,
            cancel_request,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                poll_wic_job(&mut app, &mut cancel_operation).await;
                let ready = app
                    .wic_session(cancel_id)
                    .and_then(|session| app.background_jobs.get(session.background_job_id))
                    .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"));
                if ready {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
        let Some(Dialog::WicCancellationConfirmation {
            id: dialog_id,
            incomplete_device_warning: true,
        }) = app.active_dialog().cloned()
        else {
            panic!("write cancellation must retain its incomplete-device warning");
        };
        let effect = wic_cancellation_confirmation_action(dialog_id, true, Input::Enter)
            .and_then(|action| update(&mut app, action));
        let Some(Effect::CancelWicSession(effect_id)) = effect else {
            panic!("expected write cancellation effect");
        };
        begin_wic_cancellation(&mut app, &mut cancel_operation, effect_id);
        tokio::time::timeout(Duration::from_secs(4), async {
            while cancel_operation.is_some() {
                poll_wic_job(&mut app, &mut cancel_operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let cancelled = app.wic_session(cancel_id).unwrap();
        assert_eq!(
            app.background_jobs
                .get(cancelled.background_job_id)
                .unwrap()
                .status,
            yoctui_model::BackgroundJobStatus::Cancelled
        );

        write_test_executable(&wic, "#!/bin/sh\nprintf 'write-error\\n' >&2\nexit 9\n");
        let (failed_id, failed_request) = wic_device_write_start_effect(&mut app, &inspector).await;
        let mut failed_operation = None;
        begin_wic_job(
            &mut app,
            &mut failed_operation,
            &inspector,
            &build_dir,
            Duration::from_millis(100),
            failed_id,
            failed_request,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            while failed_operation.is_some() {
                poll_wic_job(&mut app, &mut failed_operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let failed = app.wic_session(failed_id).unwrap();
        let failed_job = app.background_jobs.get(failed.background_job_id).unwrap();
        assert_eq!(failed_job.status, yoctui_model::BackgroundJobStatus::Failed);
        assert_eq!(failed.exit_code, Some(9));
        assert!(
            failed_job
                .output
                .iter()
                .any(|entry| entry.message == "write-error")
        );

        write_test_executable(&wic, "#!/bin/sh\nsleep 30\n");
        let (lost_id, lost_request) = wic_device_write_start_effect(&mut app, &inspector).await;
        let mut lost_operation = None;
        begin_wic_job(
            &mut app,
            &mut lost_operation,
            &inspector,
            &build_dir,
            Duration::from_millis(100),
            lost_id,
            lost_request,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                poll_wic_job(&mut app, &mut lost_operation).await;
                let running = app
                    .wic_session(lost_id)
                    .and_then(|session| app.background_jobs.get(session.background_job_id))
                    .is_some_and(|job| job.status == yoctui_model::BackgroundJobStatus::Running);
                if running {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let lost_handle = tokio::spawn(async {
            std::future::pending::<(WicJobRunner, Result<bool, WicAdapterError>)>().await
        });
        lost_handle.abort();
        lost_operation.as_mut().unwrap().cancellation = Some(lost_handle);
        tokio::task::yield_now().await;
        poll_wic_job(&mut app, &mut lost_operation).await;
        let lost = app.wic_session(lost_id).unwrap();
        assert_eq!(
            app.background_jobs
                .get(lost.background_job_id)
                .unwrap()
                .status,
            yoctui_model::BackgroundJobStatus::Lost
        );

        let (stale_id, stale_request) = wic_device_write_start_effect(&mut app, &inspector).await;
        let changed_inventory = inventory.replace("SERIAL-123", "SERIAL-CHANGED");
        write_test_executable(
            &lsblk,
            &format!("#!/bin/sh\nprintf '%s' '{}'\n", changed_inventory),
        );
        let mut stale_operation = None;
        begin_wic_job(
            &mut app,
            &mut stale_operation,
            &inspector,
            &build_dir,
            Duration::from_millis(100),
            stale_id,
            stale_request,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            while stale_operation.is_some() {
                poll_wic_job(&mut app, &mut stale_operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let stale = app.wic_session(stale_id).unwrap();
        assert!(
            stale
                .error_detail
                .as_deref()
                .is_some_and(|message| message.contains("identity changed"))
        );

        let (reject_id, _) = wic_device_write_start_effect(&mut app, &inspector).await;
        let _ = update(
            &mut app,
            Action::WicSessionStarting {
                id: reject_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::WicSessionRunning { id: reject_id });
        let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
        let effect = wic_cancellation_confirmation_action(reject_id, true, Input::Enter)
            .and_then(|action| update(&mut app, action));
        let Some(Effect::CancelWicSession(effect_id)) = effect else {
            panic!("expected unowned write cancellation effect");
        };
        let mut unowned = None;
        begin_wic_cancellation(&mut app, &mut unowned, effect_id);
        let rejected = app.wic_session(reject_id).unwrap();
        assert_eq!(
            app.background_jobs
                .get(rejected.background_job_id)
                .unwrap()
                .status,
            yoctui_model::BackgroundJobStatus::Running
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_workspace_cli_discovers_runs_scans_and_persists_across_navigation() {
        let (directory, build_dir, mut app) = wic_workspace_fixture(
            "success",
            "printf '%s\\n' \"$@\"; printf 'wic' > \"$6/generated.wic\"; exit 0",
        )
        .await;
        let WicCapability::Available { kickstarts, .. } = &app.wic_capability else {
            panic!("available capability");
        };
        assert_eq!(
            kickstarts[0].identity.path.as_deref(),
            Some(directory.join("directdisk.wks").as_path())
        );
        assert_eq!(
            wic_create_dialog_action(false, Input::Char('Q')),
            None,
            "modal Q input must not leak to the Images workspace"
        );
        let (id, operation_request) = wic_workspace_start_effect(&mut app);
        let mut operation = None;
        begin_wic_job(
            &mut app,
            &mut operation,
            &WicDeviceInspector::default(),
            &build_dir,
            Duration::from_millis(100),
            id,
            operation_request,
        )
        .await;
        let duplicate = update(&mut app, Action::BeginSelectedWicCreate);
        assert!(duplicate.is_none());
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("already active"))
        );
        let _ = update(&mut app, Action::DismissNotification);
        app.screen = Screen::Logs;
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_wic_job(&mut app, &mut operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let session = app.wic_session(id).unwrap();
        let job = app.background_jobs.get(session.background_job_id).unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert_eq!(app.screen, Screen::Logs);
        assert!(
            job.output
                .iter()
                .any(|entry| entry.message == "core-image-minimal")
        );
        let outputs = app.wic_output_rows();
        assert_eq!(outputs.len(), 1);
        assert_eq!(
            outputs[0].identity.path,
            directory.join("deploy/generated.wic")
        );
        assert_eq!(app.wic_output_selection, Some(outputs[0].identity.clone()));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_workspace_cli_reports_failure_graceful_and_forced_cancellation() {
        let (failed_directory, failed_build, mut failed) =
            wic_workspace_fixture("failure", "printf 'failed\\n' >&2; exit 9").await;
        let (failed_id, failed_request) = wic_workspace_start_effect(&mut failed);
        let mut failed_operation = None;
        begin_wic_job(
            &mut failed,
            &mut failed_operation,
            &WicDeviceInspector::default(),
            &failed_build,
            Duration::from_millis(100),
            failed_id,
            failed_request,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(2), async {
            while failed_operation.is_some() {
                poll_wic_job(&mut failed, &mut failed_operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let job = failed
            .background_jobs
            .get(failed.wic_session(failed_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Failed);
        assert_eq!(failed.wic_session(failed_id).unwrap().exit_code, Some(9));
        fs::remove_dir_all(failed_directory).unwrap();

        for (name, body, expect_forced) in [
            (
                "graceful-cancel",
                "trap 'exit 0' TERM; printf 'ready\\n'; while :; do :; done",
                false,
            ),
            (
                "forced-cancel",
                "trap '' TERM; printf 'ready\\n'; while :; do sleep 1; done",
                true,
            ),
        ] {
            let (directory, build_dir, mut app) = wic_workspace_fixture(name, body).await;
            let (id, request) = wic_workspace_start_effect(&mut app);
            let mut operation = None;
            begin_wic_job(
                &mut app,
                &mut operation,
                &WicDeviceInspector::default(),
                &build_dir,
                Duration::from_millis(50),
                id,
                request,
            )
            .await;
            tokio::time::timeout(Duration::from_secs(2), async {
                loop {
                    poll_wic_job(&mut app, &mut operation).await;
                    let ready = app
                        .wic_session(id)
                        .and_then(|session| app.background_jobs.get(session.background_job_id))
                        .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"));
                    if ready {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            let _ = update(&mut app, Action::BeginActiveWicSessionCancellation);
            let Some(Dialog::WicCancellationConfirmation {
                id: dialog_id,
                incomplete_device_warning,
            }) = app.active_dialog().cloned()
            else {
                panic!("Wic cancellation dialog");
            };
            let effect = wic_cancellation_confirmation_action(
                dialog_id,
                incomplete_device_warning,
                Input::Enter,
            )
            .and_then(|action| update(&mut app, action));
            let Some(Effect::CancelWicSession(effect_id)) = effect else {
                panic!("Wic cancellation effect");
            };
            begin_wic_cancellation(&mut app, &mut operation, effect_id);
            tokio::time::timeout(Duration::from_secs(2), async {
                while operation.is_some() {
                    poll_wic_job(&mut app, &mut operation).await;
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap();
            let job = app
                .background_jobs
                .get(app.wic_session(id).unwrap().background_job_id)
                .unwrap();
            assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Cancelled);
            assert_eq!(
                job.output
                    .iter()
                    .any(|entry| entry.message.contains("forced termination")),
                expect_forced
            );
            fs::remove_dir_all(directory).unwrap();
        }
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn wic_workspace_cli_reports_rejection_and_unexpected_runner_loss() {
        let (reject_directory, _, mut rejected) = wic_workspace_fixture("reject", "sleep 30").await;
        let (reject_id, _) = wic_workspace_start_effect(&mut rejected);
        let _ = update(
            &mut rejected,
            Action::WicSessionStarting {
                id: reject_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut rejected, Action::WicSessionRunning { id: reject_id });
        let _ = update(&mut rejected, Action::BeginActiveWicSessionCancellation);
        let Some(Effect::CancelWicSession(effect_id)) = update(
            &mut rejected,
            Action::ConfirmWicSessionCancellation {
                id: reject_id,
                acknowledge_incomplete_device: false,
            },
        ) else {
            panic!("Wic rejection effect");
        };
        let mut no_operation = None;
        begin_wic_cancellation(&mut rejected, &mut no_operation, effect_id);
        let job = rejected
            .background_jobs
            .get(rejected.wic_session(reject_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Running);
        fs::remove_dir_all(reject_directory).unwrap();

        let (lost_directory, lost_build, mut lost) =
            wic_workspace_fixture("lost", "sleep 30").await;
        let (lost_id, lost_request) = wic_workspace_start_effect(&mut lost);
        let mut lost_operation = None;
        begin_wic_job(
            &mut lost,
            &mut lost_operation,
            &WicDeviceInspector::default(),
            &lost_build,
            Duration::from_millis(100),
            lost_id,
            lost_request,
        )
        .await;
        poll_wic_job(&mut lost, &mut lost_operation).await;
        poll_wic_job(&mut lost, &mut lost_operation).await;
        let lost_handle = tokio::spawn(async {
            std::future::pending::<(WicJobRunner, Result<bool, WicAdapterError>)>().await
        });
        lost_handle.abort();
        lost_operation.as_mut().unwrap().cancellation = Some(lost_handle);
        tokio::task::yield_now().await;
        poll_wic_job(&mut lost, &mut lost_operation).await;
        let job = lost
            .background_jobs
            .get(lost.wic_session(lost_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Lost);
        fs::remove_dir_all(lost_directory).unwrap();
    }

    #[cfg(unix)]
    fn sdk_workflow_fixture(
        name: &str,
        publish_body: &str,
        find_body: &str,
        run_body: &str,
    ) -> (PathBuf, PathBuf, SdkArtifactAdapter, SdkToolAdapter, App) {
        use std::os::unix::fs::PermissionsExt;

        let directory = std::env::temp_dir().join(format!(
            "yoctui-sdk-workflow-{}-{name}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let build = directory.join("build");
        let source = directory.join("source");
        let scripts = source.join("scripts");
        let deploy = directory.join("deploy-sdk");
        fs::create_dir_all(&build).unwrap();
        fs::create_dir_all(&scripts).unwrap();
        fs::create_dir_all(&deploy).unwrap();
        for (tool, body) in [
            ("oe-publish-sdk", publish_body),
            ("oe-find-native-sysroot", find_body),
            ("oe-run-native", run_body),
        ] {
            let path = scripts.join(tool);
            fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(path, permissions).unwrap();
        }
        fs::write(
            deploy.join("poky-glibc-x86_64-core-image-minimal-qemux86-64.sh"),
            b"installer",
        )
        .unwrap();

        let build = fs::canonicalize(build).unwrap();
        let source = fs::canonicalize(source).unwrap();
        let deploy = fs::canonicalize(deploy).unwrap();
        let mut app = App::new(100, 100_000);
        app.screen = Screen::Sdk;
        app.build.target = Some("core-image-minimal".into());
        app.workspace.build_dir = Some(build.clone());
        app.workspace.source_dir = Some(source.clone());
        app.workspace
            .variables
            .insert("SDK_DEPLOY".into(), deploy.display().to_string());
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        let artifact_adapter = SdkArtifactAdapter::new(deploy.clone());
        let tool_adapter = SdkToolAdapter::new(build.clone(), deploy, vec![build.clone(), source]);
        (directory, build, artifact_adapter, tool_adapter, app)
    }

    fn set_sdk_publish_destination(app: &mut App, destination: &Path) {
        let Some(Dialog::SdkPublishTomlEditor { content, .. }) = app.active_dialog_mut() else {
            panic!("SDK publication TOML editor");
        };
        *content = format!("destination = \"{}\"\n", destination.display());
    }

    fn set_sdk_native_draft(app: &mut App, draft: yoctui_model::SdkNativeDraft) {
        let Some(Dialog::SdkNativeTomlEditor { content, .. }) = app.active_dialog_mut() else {
            panic!("SDK native TOML editor");
        };
        let mode = match draft.mode {
            yoctui_model::SdkNativeMode::FindSysroot => "find-sysroot",
            yoctui_model::SdkNativeMode::RunNative => "run-native",
        };
        *content = format!(
            "mode = \"{mode}\"\nworkspace = \"{}\"\nrecipe = \"{}\"\ntool = \"{}\"\narguments = \"{}\"\n",
            draft.extracted_root,
            draft.recipe,
            draft.tool,
            draft.arguments.join(" ")
        );
    }

    async fn sdk_workflow_poll_scan(
        app: &mut App,
        operation: &mut Option<SdkArtifactBackgroundOperation>,
    ) {
        tokio::time::timeout(Duration::from_secs(2), async {
            while operation.is_some() {
                poll_sdk_artifact_operation(app, operation).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    async fn sdk_workflow_poll_job(
        app: &mut App,
        operation: &mut Option<SdkCliOperation>,
    ) -> Option<SdkOperation> {
        tokio::time::timeout(Duration::from_secs(3), async {
            let mut completed = None;
            while operation.is_some() {
                completed = poll_sdk_job(app, operation).await.or(completed);
                tokio::task::yield_now().await;
            }
            completed
        })
        .await
        .unwrap()
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_workflow_cli_inspects_capability_and_correlates_replaceable_scans() {
        use std::os::unix::fs::symlink;

        let (directory, _, artifact_adapter, tool_adapter, mut app) =
            sdk_workflow_fixture("scan", "exit 0", "exit 0", "exit 0");
        let mut capability = None;
        begin_sdk_capability_operation(
            &mut app,
            Some(&tool_adapter),
            &mut capability,
            Effect::InspectSdkTools,
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            while capability.is_some() {
                poll_sdk_capability_operation(&mut app, &mut capability).await;
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert!(matches!(
            app.sdk_tool_capability,
            SdkToolCapability::Available {
                publish: Some(_),
                find_sysroot: Some(_),
                run_native: Some(_)
            }
        ));

        let deploy = PathBuf::from(app.workspace.variables.get("SDK_DEPLOY").unwrap());
        symlink("/outside-sdk", deploy.join("ignored-link")).unwrap();
        let first_effect = update(&mut app, Action::BeginSdkArtifactInventory).unwrap();
        let Effect::GetSdkArtifacts(first_request) = first_effect.clone() else {
            panic!("expected SDK scan");
        };
        let mut scan = None;
        begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, first_effect);

        let replacement = SdkArtifactInventoryRequest {
            generation: first_request.generation + 1,
            ..first_request.clone()
        };
        app.sdk_artifacts = yoctui_model::SdkArtifactInventoryState::Loading {
            request: replacement.clone(),
        };
        begin_sdk_artifact_operation(
            &mut app,
            Some(&artifact_adapter),
            &mut scan,
            Effect::GetSdkArtifacts(replacement.clone()),
        );
        let ignored_before = app.background_jobs.ignored_transitions;
        let _ = update(
            &mut app,
            Action::SdkArtifactInventoryLoaded {
                request: first_request,
                artifacts: Vec::new(),
                limitations: Vec::new(),
            },
        );
        assert_eq!(app.background_jobs.ignored_transitions, ignored_before + 1);
        sdk_workflow_poll_scan(&mut app, &mut scan).await;
        assert!(matches!(
            &app.sdk_artifacts,
            yoctui_model::SdkArtifactInventoryState::Partial {
                request,
                artifacts,
                limitations
            } if request == &replacement
                && artifacts.iter().any(|artifact| artifact.kind == yoctui_model::SdkArtifactKind::Installer)
                && limitations.iter().any(|limitation| limitation.contains("symlink"))
        ));

        let failure_request = SdkArtifactInventoryRequest {
            generation: replacement.generation + 1,
            ..replacement
        };
        app.sdk_artifacts = yoctui_model::SdkArtifactInventoryState::Loading {
            request: failure_request.clone(),
        };
        begin_sdk_artifact_operation(
            &mut app,
            None,
            &mut scan,
            Effect::GetSdkArtifacts(failure_request.clone()),
        );
        assert!(matches!(
            &app.sdk_artifacts,
            yoctui_model::SdkArtifactInventoryState::Failed { request, message }
                if request == &failure_request && message.contains("SDK_DEPLOY")
        ));

        let cancel_effect = update(&mut app, Action::RefreshSdkArtifactInventory).unwrap();
        begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, cancel_effect);
        let Some(Effect::CancelSdkArtifactOperation) =
            update(&mut app, Action::BeginActiveSdkSessionCancellation)
        else {
            panic!("expected independently routed SDK scan cancellation");
        };
        assert!(scan.as_ref().unwrap().cancellation.cancel());
        sdk_workflow_poll_scan(&mut app, &mut scan).await;
        assert!(matches!(
            &app.sdk_artifacts,
            yoctui_model::SdkArtifactInventoryState::Failed { message, .. }
                if message.contains("cancelled")
        ));
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_workflow_cli_runs_publish_and_native_with_output_refresh_and_navigation() {
        let (directory, build, artifact_adapter, tool_adapter, mut app) = sdk_workflow_fixture(
            "success",
            "printf 'publish:%s\\n' \"$1\"; touch \"$2/published\"; exit 0",
            "printf 'sysroot:%s\\n' \"$1\"; exit 0",
            "printf 'child:%s args:%s\\n' \"$YOCTUI_SDK_CHILD_ONLY\" \"$*\"; exit 0",
        );
        let _ = update(
            &mut app,
            Action::SdkToolCapabilityLoaded(tool_adapter.capability()),
        );
        let effect = update(&mut app, Action::BeginSdkArtifactInventory).unwrap();
        let mut scan = None;
        begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, effect);
        sdk_workflow_poll_scan(&mut app, &mut scan).await;
        let open = sdk_workspace_action(false, Input::Char('o'))
            .and_then(|action| update(&mut app, action));
        assert!(
            matches!(open, Some(Effect::OpenInEditor(path)) if path.to_string_lossy().ends_with(".sh"))
        );

        let destination = directory.join("published");
        fs::create_dir(&destination).unwrap();
        let _ = update(&mut app, Action::BeginSelectedSdkPublish);
        set_sdk_publish_destination(&mut app, &destination);
        let _ = update(&mut app, Action::PreviewSdkPublish);
        let Some(Effect::StartSdkSession { id, operation }) =
            update(&mut app, Action::ConfirmSdkPublish)
        else {
            panic!("expected SDK publication");
        };
        let mut owned = None;
        begin_sdk_job(
            &mut app,
            &mut owned,
            Some(&tool_adapter),
            Duration::from_millis(100),
            Duration::from_secs(2),
            id,
            operation,
        );
        app.screen = Screen::Logs;
        let completed = sdk_workflow_poll_job(&mut app, &mut owned).await;
        assert!(matches!(completed, Some(SdkOperation::Publish(_))));
        let job = app
            .background_jobs
            .get(app.sdk_session(id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert!(
            job.output
                .iter()
                .any(|entry| entry.message.starts_with("publish:"))
        );
        assert_eq!(app.screen, Screen::Logs);
        let generation = app.sdk_artifact_generation;
        let refresh = update(&mut app, Action::RefreshSdkArtifactInventory).unwrap();
        begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, refresh);
        sdk_workflow_poll_scan(&mut app, &mut scan).await;
        assert!(app.sdk_artifact_generation > generation);

        let extracted = directory.join("extracted");
        fs::create_dir(&extracted).unwrap();
        fs::write(
            extracted.join("environment-setup-x86_64-pokysdk-linux"),
            "export YOCTUI_SDK_CHILD_ONLY=visible\n",
        )
        .unwrap();
        let executable = match &app.sdk_tool_capability {
            SdkToolCapability::Available {
                run_native: Some(path),
                ..
            } => path.clone(),
            capability => panic!("unexpected SDK capability: {capability:?}"),
        };
        let _ = update(&mut app, Action::BeginSdkNative);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::SdkNativeTomlEditor { .. })
        ));
        set_sdk_native_draft(
            &mut app,
            yoctui_model::SdkNativeDraft {
                mode: yoctui_model::SdkNativeMode::RunNative,
                extracted_root: extracted.display().to_string(),
                recipe: "busybox".into(),
                tool: "sh".into(),
                arguments: vec!["--version".into()],
            },
        );
        let _ = update(&mut app, Action::PreviewSdkNative);
        let Some(Effect::StartSdkSession {
            id: native_id,
            operation: native_operation,
        }) = update(&mut app, Action::ConfirmSdkNative)
        else {
            panic!("expected SDK native tool operation");
        };
        assert!(matches!(
            &native_operation,
            SdkOperation::Native(request)
                if request.executable == executable && request.extracted_root.as_ref() == Some(&extracted)
        ));
        begin_sdk_job(
            &mut app,
            &mut owned,
            Some(&tool_adapter),
            Duration::from_millis(100),
            Duration::from_secs(2),
            native_id,
            native_operation,
        );
        let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
        let native_job = app
            .background_jobs
            .get(app.sdk_session(native_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(
            native_job.status,
            yoctui_model::BackgroundJobStatus::Succeeded
        );
        assert!(
            native_job
                .output
                .iter()
                .any(|entry| entry.message == "child:visible args:busybox sh --version")
        );
        assert!(std::env::var_os("YOCTUI_SDK_CHILD_ONLY").is_none());

        let populate = BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("populate_sdk".into()),
            force: false,
        };
        let mut pending = Some(populate);
        let effect = sdk_refresh_after_build_event(
            &mut app,
            &mut pending,
            &BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        );
        assert!(matches!(effect, Some(Effect::GetSdkArtifacts(_))));
        assert!(pending.is_none());
        let mut test_pending = Some(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("testsdk".into()),
            force: false,
        });
        assert!(
            sdk_refresh_after_build_event(
                &mut app,
                &mut test_pending,
                &BackendEvent::BuildCompleted {
                    success: true,
                    exit_code: Some(0),
                },
            )
            .is_none()
        );
        assert!(test_pending.is_none());
        assert!(build.is_dir());
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn sdk_workflow_cli_preserves_failure_timeout_cancel_rejection_and_loss() {
        let (directory, _, artifact_adapter, tool_adapter, mut app) = sdk_workflow_fixture(
            "terminal",
            "printf 'publish failed\\n' >&2; exit 17",
            "exit 0",
            "trap '' TERM; printf 'ready\\n'; while :; do :; done",
        );
        let _ = update(
            &mut app,
            Action::SdkToolCapabilityLoaded(tool_adapter.capability()),
        );
        let effect = update(&mut app, Action::BeginSdkArtifactInventory).unwrap();
        let mut scan = None;
        begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, effect);
        sdk_workflow_poll_scan(&mut app, &mut scan).await;

        let destination = directory.join("failure-destination");
        fs::create_dir(&destination).unwrap();
        let _ = update(&mut app, Action::BeginSelectedSdkPublish);
        set_sdk_publish_destination(&mut app, &destination);
        let _ = update(&mut app, Action::PreviewSdkPublish);
        let Some(Effect::StartSdkSession { id, operation }) =
            update(&mut app, Action::ConfirmSdkPublish)
        else {
            panic!("expected failing SDK publication");
        };
        let mut owned = None;
        begin_sdk_job(
            &mut app,
            &mut owned,
            Some(&tool_adapter),
            Duration::from_millis(50),
            Duration::from_secs(2),
            id,
            operation,
        );
        let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
        let failed = app
            .background_jobs
            .get(app.sdk_session(id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(failed.status, yoctui_model::BackgroundJobStatus::Failed);
        assert_eq!(app.sdk_session(id).unwrap().exit_code, Some(17));
        assert!(
            failed
                .output
                .iter()
                .any(|entry| entry.message == "publish failed")
        );

        let _ = update(&mut app, Action::BeginSdkNative);
        set_sdk_native_draft(
            &mut app,
            yoctui_model::SdkNativeDraft {
                mode: yoctui_model::SdkNativeMode::RunNative,
                extracted_root: String::new(),
                recipe: "busybox".into(),
                tool: "sh".into(),
                arguments: Vec::new(),
            },
        );
        let _ = update(&mut app, Action::PreviewSdkNative);
        let Some(Effect::StartSdkSession {
            id: timeout_id,
            operation: timeout_operation,
        }) = update(&mut app, Action::ConfirmSdkNative)
        else {
            panic!("expected timed SDK native operation");
        };
        begin_sdk_job(
            &mut app,
            &mut owned,
            Some(&tool_adapter),
            Duration::from_millis(30),
            Duration::from_millis(30),
            timeout_id,
            timeout_operation,
        );
        let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
        let timed_out = app
            .background_jobs
            .get(app.sdk_session(timeout_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(timed_out.status, yoctui_model::BackgroundJobStatus::Failed);
        assert!(
            timed_out
                .error
                .as_ref()
                .and_then(|error| error.detail.as_deref())
                .is_some_and(|detail| detail.contains("timed out"))
        );

        let _ = update(&mut app, Action::BeginSdkNative);
        set_sdk_native_draft(
            &mut app,
            yoctui_model::SdkNativeDraft {
                mode: yoctui_model::SdkNativeMode::RunNative,
                extracted_root: String::new(),
                recipe: "busybox".into(),
                tool: "sh".into(),
                arguments: Vec::new(),
            },
        );
        let _ = update(&mut app, Action::PreviewSdkNative);
        let Some(Effect::StartSdkSession {
            id: cancel_id,
            operation: cancel_operation,
        }) = update(&mut app, Action::ConfirmSdkNative)
        else {
            panic!("expected cancellable SDK native operation");
        };
        begin_sdk_job(
            &mut app,
            &mut owned,
            Some(&tool_adapter),
            Duration::from_millis(30),
            Duration::from_secs(2),
            cancel_id,
            cancel_operation,
        );
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let _ = poll_sdk_job(&mut app, &mut owned).await;
                let ready = app
                    .sdk_session(cancel_id)
                    .and_then(|session| app.background_jobs.get(session.background_job_id))
                    .is_some_and(|job| job.output.iter().any(|entry| entry.message == "ready"));
                if ready {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
        let Some(Effect::CancelSdkSession(effect_id)) =
            update(&mut app, Action::ConfirmSdkSessionCancellation)
        else {
            panic!("expected SDK cancellation");
        };
        begin_sdk_cancellation(&mut app, &mut owned, effect_id);
        let _ = sdk_workflow_poll_job(&mut app, &mut owned).await;
        let cancelled = app
            .background_jobs
            .get(app.sdk_session(cancel_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(
            cancelled.status,
            yoctui_model::BackgroundJobStatus::Cancelled
        );
        assert!(
            cancelled
                .output
                .iter()
                .any(|entry| entry.message.contains("forced termination"))
        );

        let _ = update(&mut app, Action::BeginSdkNative);
        set_sdk_native_draft(
            &mut app,
            yoctui_model::SdkNativeDraft {
                mode: yoctui_model::SdkNativeMode::FindSysroot,
                extracted_root: String::new(),
                recipe: "busybox".into(),
                tool: String::new(),
                arguments: Vec::new(),
            },
        );
        let _ = update(&mut app, Action::PreviewSdkNative);
        let Some(Effect::StartSdkSession {
            id: rejected_id,
            operation: rejected_operation,
        }) = update(&mut app, Action::ConfirmSdkNative)
        else {
            panic!("expected rejected SDK operation");
        };
        let _ = update(
            &mut app,
            Action::SdkSessionStarting {
                id: rejected_id,
                started_at: SystemTime::UNIX_EPOCH,
            },
        );
        let _ = update(&mut app, Action::SdkSessionRunning { id: rejected_id });
        let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
        let Some(Effect::CancelSdkSession(rejected_effect_id)) =
            update(&mut app, Action::ConfirmSdkSessionCancellation)
        else {
            panic!("expected rejected cancellation effect");
        };
        begin_sdk_cancellation(&mut app, &mut None, rejected_effect_id);
        let rejected = app
            .background_jobs
            .get(app.sdk_session(rejected_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(rejected.status, yoctui_model::BackgroundJobStatus::Running);

        let lost_handle = tokio::spawn(async {
            std::future::pending::<(SdkToolJobRunner, Result<(), SdkToolAdapterError>)>().await
        });
        lost_handle.abort();
        let mut lost_operation = Some(SdkCliOperation {
            id: rejected_id,
            operation: rejected_operation,
            starting: Some(lost_handle),
            runner: None,
            timeout_wait: None,
            cancellation: None,
        });
        tokio::task::yield_now().await;
        poll_sdk_job(&mut app, &mut lost_operation).await;
        let lost = app
            .background_jobs
            .get(app.sdk_session(rejected_id).unwrap().background_job_id)
            .unwrap();
        assert_eq!(lost.status, yoctui_model::BackgroundJobStatus::Lost);
        fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(unix)]
    fn test_workflow_cli_executable(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;
        fs::write(path, body).unwrap();
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_workflow_cli_discovers_and_runs_selftest_across_navigation() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-testing-cli-runner-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        let bin = directory.join("bin");
        let build = directory.join("build");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&build).unwrap();
        test_workflow_cli_executable(
            &bin.join("oe-selftest"),
            "#!/bin/sh\nprintf 'selftest output\\n'\nexit 0\n",
        );
        test_workflow_cli_executable(&bin.join("bitbake-selftest"), "#!/bin/sh\nexit 0\n");
        test_workflow_cli_executable(&bin.join("resulttool"), "#!/bin/sh\nexit 0\n");

        let mut app = App::new(100, 100_000);
        app.screen = Screen::Testing;
        app.workspace
            .variables
            .insert("MACHINE".into(), "qemux86-64".into());
        app.workspace
            .variables
            .insert("DISTRO".into(), "poky".into());
        app.build.target = Some("core-image-minimal".into());
        let mut coordinator =
            TestCliCoordinator::new(build, vec![bin], yoctui_model::PtestCapability::Configured);
        assert!(
            coordinator
                .handle_effect(&mut app, Effect::InspectTestCapability)
                .await
        );
        assert!(matches!(
            app.test_capability.oe_selftest,
            yoctui_model::TestExecutableCapability::Available(_)
        ));
        assert!(matches!(
            app.result_tool_capability,
            yoctui_model::ResultToolCapability::Available(_)
        ));

        let _ = update(&mut app, Action::BeginSelectedTestLaunch);
        let _ = update(&mut app, Action::PreviewTestLaunch);
        let effect = update(&mut app, Action::ConfirmTestLaunch).unwrap();
        let Effect::StartTestSession { id, .. } = effect.clone() else {
            panic!("expected exact selftest effect");
        };
        assert!(coordinator.handle_effect(&mut app, effect).await);
        let _ = update(&mut app, Action::Open(Screen::Dashboard));
        for _ in 0..100 {
            coordinator.poll(&mut app).await;
            if app
                .test_session(id)
                .is_some_and(|session| session.outcome.is_some())
            {
                break;
            }
            tokio::task::yield_now().await;
        }
        let session = app.test_session(id).unwrap();
        assert_eq!(
            session.outcome,
            Some(yoctui_model::TestSessionOutcome::Succeeded)
        );
        let job = app
            .background_jobs
            .get(session.background_job_id.unwrap())
            .unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert!(
            job.output
                .iter()
                .any(|entry| entry.message == "selftest output")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn test_workflow_cli_correlates_managed_bitbake_success_and_cancellation() {
        fn queued_test_build() -> (App, BuildJobCoordinator, TestSessionId, BuildRequest) {
            let mut app = App::new(100, 100_000);
            app.screen = Screen::Testing;
            app.test_family_selection = yoctui_model::TestFamily::TestImage;
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            app.workspace
                .variables
                .insert("DISTRO".into(), "poky".into());
            app.build.target = Some("core-image-minimal".into());
            let _ = update(&mut app, Action::BeginSelectedTestLaunch);
            let _ = update(&mut app, Action::PreviewTestLaunch);
            let Some(Effect::StartTestBuildSession { id, request }) =
                update(&mut app, Action::ConfirmTestLaunch)
            else {
                panic!("expected managed Testing build");
            };
            let mut coordinator = BuildJobCoordinator::default();
            for action in coordinator
                .queue_build(&request, SystemTime::UNIX_EPOCH)
                .unwrap()
            {
                let _ = update(&mut app, action);
            }
            let background_job_id = coordinator.active_job_id().unwrap();
            let _ = update(
                &mut app,
                Action::AttachTestBuildSession {
                    id,
                    background_job_id,
                },
            );
            (app, coordinator, id, request)
        }

        let (mut succeeded, mut success_jobs, success_id, request) = queued_test_build();
        assert_eq!(request.task.as_deref(), Some("testimage"));
        for event in [
            BackendEvent::BuildStarted,
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        ] {
            if let Some(action) = test_build_action_for_event(&succeeded, success_id, &event) {
                let _ = update(&mut succeeded, action);
            }
            for action in success_jobs.job_actions_for_event(&event, SystemTime::UNIX_EPOCH) {
                let _ = update(&mut succeeded, action);
            }
        }
        assert_eq!(
            succeeded.test_session(success_id).unwrap().outcome,
            Some(yoctui_model::TestSessionOutcome::Succeeded)
        );

        let (mut cancelled, mut cancel_jobs, cancel_id, _) = queued_test_build();
        if let Some(action) =
            test_build_action_for_event(&cancelled, cancel_id, &BackendEvent::BuildStarted)
        {
            let _ = update(&mut cancelled, action);
        }
        for action in
            cancel_jobs.job_actions_for_event(&BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH)
        {
            let _ = update(&mut cancelled, action);
        }
        let _ = update(&mut cancelled, Action::BeginActiveTestSessionCancellation);
        assert!(matches!(
            update(&mut cancelled, Action::ConfirmTestSessionCancellation),
            Some(Effect::CancelTestSession(id)) if id == cancel_id
        ));
        let event = BackendEvent::BuildCompleted {
            success: false,
            exit_code: Some(130),
        };
        let action = test_build_action_for_event(&cancelled, cancel_id, &event).unwrap();
        let _ = update(&mut cancelled, action);
        for action in cancel_jobs.job_actions_for_event(&event, SystemTime::UNIX_EPOCH) {
            let _ = update(&mut cancelled, action);
        }
        assert_eq!(
            cancelled.test_session(cancel_id).unwrap().outcome,
            Some(yoctui_model::TestSessionOutcome::Cancelled)
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_workflow_cli_imports_compares_and_exports_exact_results() {
        let directory =
            std::env::temp_dir().join(format!("yoctui-testing-cli-results-{}", std::process::id()));
        let _ = fs::remove_dir_all(&directory);
        let bin = directory.join("bin");
        let build = directory.join("build");
        let results = directory.join("results");
        let export = directory.join("export");
        fs::create_dir_all(&bin).unwrap();
        fs::create_dir_all(&build).unwrap();
        fs::create_dir_all(&results).unwrap();
        fs::create_dir_all(&export).unwrap();
        test_workflow_cli_executable(
            &bin.join("resulttool"),
            "#!/bin/sh\nif [ \"$1\" = junit ]; then : > \"$4\"; fi\nexit 0\n",
        );
        for tool in ["oe-selftest", "bitbake-selftest"] {
            test_workflow_cli_executable(&bin.join(tool), "#!/bin/sh\nexit 0\n");
        }
        let result_json = |status: &str| {
            format!(
                r#"{{"runtime":{{"configuration":{{"TEST_TYPE":"runtime","MACHINE":"qemux86-64","IMAGE_BASENAME":"core-image-minimal"}},"result":{{"runtime.Case.test_one":{{"status":"{status}"}}}}}}}}"#
            )
        };
        let baseline_path = results.join("baseline").join("testresults.json");
        let candidate_path = results.join("candidate").join("testresults.json");
        fs::create_dir_all(baseline_path.parent().unwrap()).unwrap();
        fs::create_dir_all(candidate_path.parent().unwrap()).unwrap();
        fs::write(&baseline_path, result_json("PASSED")).unwrap();
        fs::write(&candidate_path, result_json("FAILED")).unwrap();

        let mut app = App::new(100, 100_000);
        app.screen = Screen::Testing;
        let mut coordinator =
            TestCliCoordinator::new(build, vec![bin], yoctui_model::PtestCapability::Configured);
        let _ = coordinator
            .handle_effect(&mut app, Effect::InspectResultToolCapability)
            .await;
        let request =
            yoctui_model::TestResultImportRequest::new(1, vec![baseline_path, candidate_path])
                .unwrap();
        app.test_results = yoctui_model::TestResultInventoryState::Loading {
            request: request.clone(),
        };
        assert!(
            coordinator
                .handle_effect(&mut app, Effect::ImportTestResults(request))
                .await
        );
        for _ in 0..100 {
            coordinator.poll(&mut app).await;
            if app.test_results.records().len() == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert_eq!(app.test_results.records().len(), 2);
        let baseline = app.test_results.records()[0].clone();
        let candidate = app.test_results.records()[1].clone();
        let request = yoctui_model::TestComparisonRequest::new(
            1,
            baseline.identity.clone(),
            candidate.identity.clone(),
        )
        .unwrap();
        app.test_comparison = yoctui_model::TestComparisonState::Loading {
            request: request.clone(),
        };
        assert!(
            coordinator
                .handle_effect(&mut app, Effect::CompareTestResults(request))
                .await
        );
        for _ in 0..100 {
            coordinator.poll(&mut app).await;
            if matches!(
                app.test_comparison,
                yoctui_model::TestComparisonState::Available { .. }
                    | yoctui_model::TestComparisonState::Partial { .. }
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(matches!(
            app.test_comparison,
            yoctui_model::TestComparisonState::Available { .. }
        ));

        let destination = export.join("results.xml");
        let inspection = coordinator
            .result_adapter
            .inspect_junit_destination(destination);
        let request =
            yoctui_model::TestJunitExportRequest::new(1, candidate.identity, &inspection).unwrap();
        app.test_junit_export = yoctui_model::TestJunitExportState::Running(request.clone());
        assert!(
            coordinator
                .handle_effect(&mut app, Effect::ExportTestJunit(request))
                .await
        );
        for _ in 0..100 {
            coordinator.poll(&mut app).await;
            if matches!(
                app.test_junit_export,
                yoctui_model::TestJunitExportState::Succeeded(_)
            ) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        assert!(matches!(
            app.test_junit_export,
            yoctui_model::TestJunitExportState::Succeeded(_)
        ));
        assert!(export.join("results.xml").is_file());
        fs::remove_dir_all(directory).unwrap();
    }

    struct SecurityCliFixture {
        root: PathBuf,
        build: PathBuf,
        reports: PathBuf,
        bin: PathBuf,
        provider: PathBuf,
    }

    impl SecurityCliFixture {
        fn new(mapper_body: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT_SECURITY_FIXTURE: AtomicU64 = AtomicU64::new(1);
            let root = std::env::temp_dir().join(format!(
                "yoctui-security-cli-{}-{}",
                std::process::id(),
                NEXT_SECURITY_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            let build = root.join("build");
            let reports = root.join("reports");
            let bin = root.join("bin");
            let layer = root.join("layer");
            for directory in [&build, &reports, &bin, &layer] {
                fs::create_dir_all(directory).unwrap();
            }
            let provider = layer.join("busybox.bb");
            fs::write(&provider, "SUMMARY = \"busybox\"\n").unwrap();
            let mapper = bin.join("cve-check-map-pkgs");
            fs::write(&mapper, mapper_body).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = fs::metadata(&mapper).unwrap().permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&mapper, permissions).unwrap();
            }
            Self {
                root,
                build,
                reports,
                bin,
                provider,
            }
        }

        fn app(&self) -> App {
            let mut app = App::new(20, 8_000);
            app.screen = Screen::Security;
            app.build.target = Some("core-image-minimal".into());
            app.workspace.build_dir = Some(self.build.clone());
            app.workspace.release = Some("6.0".into());
            app.workspace
                .variables
                .insert("MACHINE".into(), "qemux86-64".into());
            app.workspace
                .variables
                .insert("DISTRO".into(), "poky".into());
            app.workspace.variables.insert(
                "DEPLOY_DIR_IMAGE".into(),
                self.reports.display().to_string(),
            );
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "busybox".into(),
                file: Some(self.provider.clone()),
                ..yoctui_model::Recipe::default()
            });
            app.recipe_metadata.insert(
                "busybox".into(),
                yoctui_model::RecipeMetadata {
                    recipe: "busybox".into(),
                    tasks: Some(vec!["do_cve_check".into(), "do_create_recipe_sbom".into()]),
                    ..yoctui_model::RecipeMetadata::default()
                },
            );
            app
        }

        fn write_cve(&self) -> PathBuf {
            let path = self.reports.join("busybox.cve.json");
            fs::write(
                &path,
                br#"{
                  "version": "1",
                  "packages": [{
                    "name": "busybox",
                    "version": "1.36",
                    "products": [{
                      "product": "busybox",
                      "cves": [{
                        "id": "CVE-2026-0001",
                        "status": "Unpatched",
                        "severity": "HIGH",
                        "mapping": {"source": "cve-check"}
                      }]
                    }]
                  }]
                }"#,
            )
            .unwrap();
            path
        }
    }

    impl Drop for SecurityCliFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    struct SecurityBuildBackend {
        started: std::sync::Arc<std::sync::Mutex<Vec<BuildRequest>>>,
        fail_start: bool,
    }

    #[async_trait::async_trait]
    impl BitBakeBackend for SecurityBuildBackend {
        async fn inspect_workspace(
            &mut self,
        ) -> std::result::Result<yoctui_model::Workspace, yoctui_bitbake::BackendError> {
            Ok(yoctui_model::Workspace::default())
        }

        async fn list_recipes(
            &mut self,
            _filter: Option<String>,
        ) -> std::result::Result<Vec<yoctui_model::Recipe>, yoctui_bitbake::BackendError> {
            Ok(Vec::new())
        }

        async fn list_layers(
            &mut self,
        ) -> std::result::Result<Vec<yoctui_model::Layer>, yoctui_bitbake::BackendError> {
            Ok(Vec::new())
        }

        async fn get_variable(
            &mut self,
            _name: String,
            _recipe: Option<String>,
        ) -> std::result::Result<VariableValue, yoctui_bitbake::BackendError> {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn get_dependencies(
            &mut self,
            _recipe: String,
        ) -> std::result::Result<yoctui_bitbake::RecipeDependencies, yoctui_bitbake::BackendError>
        {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn get_dependency_graph(
            &mut self,
            _recipe: String,
        ) -> std::result::Result<
            yoctui_bitbake::DependencyGraphResponse,
            yoctui_bitbake::BackendError,
        > {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn get_signature_dump(
            &mut self,
            _target: SignatureTarget,
        ) -> std::result::Result<yoctui_bitbake::SignatureDumpResponse, yoctui_bitbake::BackendError>
        {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn compare_signatures(
            &mut self,
            _request: SignatureComparisonRequest,
        ) -> std::result::Result<
            yoctui_bitbake::SignatureComparisonResponse,
            yoctui_bitbake::BackendError,
        > {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn get_recipe_sources(
            &mut self,
            _recipe: String,
        ) -> std::result::Result<Vec<PathBuf>, yoctui_bitbake::BackendError> {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn get_recipe_metadata(
            &mut self,
            _recipe: String,
        ) -> std::result::Result<yoctui_model::RecipeMetadata, yoctui_bitbake::BackendError>
        {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn get_layer_relationships(
            &mut self,
        ) -> std::result::Result<Vec<yoctui_bitbake::LayerRelationship>, yoctui_bitbake::BackendError>
        {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn start_build(
            &mut self,
            request: BuildRequest,
        ) -> std::result::Result<(), yoctui_bitbake::BackendError> {
            if self.fail_start {
                Err(yoctui_bitbake::BackendError::Bridge(
                    "synthetic Security start failure".into(),
                ))
            } else {
                self.started.lock().unwrap().push(request);
                Ok(())
            }
        }

        async fn cancel_build(&mut self) -> std::result::Result<(), yoctui_bitbake::BackendError> {
            Ok(())
        }

        async fn next_event(
            &mut self,
        ) -> std::result::Result<BackendEvent, yoctui_bitbake::BackendError> {
            Err(yoctui_bitbake::BackendError::NotRunning)
        }

        async fn shutdown(&mut self) -> std::result::Result<(), yoctui_bitbake::BackendError> {
            Ok(())
        }
    }

    async fn poll_security_until(
        coordinator: &mut SecurityCliCoordinator,
        app: &mut App,
        complete: impl Fn(&App, &SecurityCliCoordinator) -> bool,
    ) {
        for _ in 0..200 {
            coordinator.poll(app).await;
            if complete(app, coordinator) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("Security CLI operation did not finish");
    }

    #[tokio::test]
    async fn security_workflow_cli_discovers_capability_imports_and_preserves_navigation() {
        let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
        let report = fixture.write_cve();
        let mut app = fixture.app();
        let mut coordinator =
            SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);

        let effect = update(
            &mut app,
            Action::Security(SecurityAction::InspectCapability),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        app.screen = Screen::Dashboard;
        poll_security_until(&mut coordinator, &mut app, |app, _| {
            matches!(
                app.security.capability,
                yoctui_model::SecurityCapability::Available(_)
            )
        })
        .await;
        let yoctui_model::SecurityCapability::Available(capability) = &app.security.capability
        else {
            unreachable!()
        };
        assert_eq!(capability.release.as_deref(), Some("6.0"));
        assert_eq!(capability.cve_task.as_deref(), Some("cve_check"));
        assert_eq!(
            capability.recipe_sbom_task.as_deref(),
            Some("create_recipe_sbom")
        );
        assert_eq!(
            capability.mapper.as_ref().unwrap().executable,
            fixture.bin.join("cve-check-map-pkgs")
        );

        let _ = update(&mut app, Action::Security(SecurityAction::BeginImport));
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::ConfirmImport(report.display().to_string())),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && app
                    .security
                    .inventory
                    .reports()
                    .is_some_and(|reports| !reports.is_empty())
        })
        .await;
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(
            app.security.visible_findings()[0].identity.cve,
            "CVE-2026-0001"
        );
    }

    #[tokio::test]
    async fn security_workflow_cli_runs_exact_mapper_refreshes_and_rejects_duplicate_start() {
        let fixture =
            SecurityCliFixture::new("#!/bin/sh\nprintf 'mapped busybox -> busybox\\n'\nexit 0\n");
        fixture.write_cve();
        let mut app = fixture.app();
        let mut coordinator =
            SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::InspectCapability),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_security_until(&mut coordinator, &mut app, |app, _| {
            matches!(
                app.security.capability,
                yoctui_model::SecurityCapability::Available(_)
            )
        })
        .await;

        let _ = update(&mut app, Action::Security(SecurityAction::BeginPackageMap));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
            other => panic!("unexpected mapper dialog: {other:?}"),
        };
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::ConfirmOperation(preview)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect.clone()).await);
        assert!(coordinator.handle_effect(&mut app, effect).await);
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("already owned"))
        );
        app.screen = Screen::Layers;
        poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.mapper.is_none()
                && coordinator.report.is_none()
                && app
                    .security
                    .sessions
                    .last()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let session = app.security.sessions.last().unwrap();
        assert_eq!(session.status, SecuritySessionStatus::Succeeded);
        assert!(
            session
                .output
                .iter()
                .any(|line| line.line.contains("mapped busybox"))
        );
        assert!(app.security.inventory.reports().is_some());
        assert_eq!(app.screen, Screen::Layers);
    }

    #[tokio::test]
    async fn security_workflow_cli_cancels_only_the_exact_mapper_session() {
        let fixture =
            SecurityCliFixture::new("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do :; done\n");
        fixture.write_cve();
        let mut app = fixture.app();
        let mut coordinator =
            SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::InspectCapability),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_security_until(&mut coordinator, &mut app, |app, _| {
            matches!(
                app.security.capability,
                yoctui_model::SecurityCapability::Available(_)
            )
        })
        .await;
        let _ = update(&mut app, Action::Security(SecurityAction::BeginPackageMap));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
            other => panic!("unexpected mapper dialog: {other:?}"),
        };
        let id = preview.id;
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::ConfirmOperation(preview)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::BeginCancellation),
        );
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::ConfirmCancellation(id)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        app.screen = Screen::Configuration;
        poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.mapper.is_none()
                && app
                    .security
                    .sessions
                    .last()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        assert_eq!(
            app.security.sessions.last().unwrap().status,
            SecuritySessionStatus::Cancelled
        );
        assert_eq!(app.screen, Screen::Configuration);
    }

    #[tokio::test]
    async fn security_workflow_cli_maps_report_failures_and_replaceable_generations() {
        let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
        let empty = fixture.root.join("empty");
        fs::create_dir(&empty).unwrap();
        let mut app = fixture.app();
        let mut coordinator =
            SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);

        let request = SecurityReportRequest::new(1, vec![empty.clone()]).unwrap();
        app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
            request: request.clone(),
        };
        coordinator.begin_report_scan(request);
        poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && matches!(
                    app.security.inventory,
                    yoctui_model::SecurityInventoryState::AvailableEmpty { .. }
                )
        })
        .await;

        for (generation, error) in [
            (2, SecurityReportAdapterError::Cancelled),
            (3, SecurityReportAdapterError::Timeout(30)),
            (
                4,
                SecurityReportAdapterError::WorkerLost("worker channel closed".into()),
            ),
            (
                5,
                SecurityReportAdapterError::PermissionDenied(empty.clone()),
            ),
        ] {
            let request = SecurityReportRequest::new(generation, vec![empty.clone()]).unwrap();
            app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
                request: request.clone(),
            };
            let cancellation = SecurityReportCancellation::default();
            coordinator.report = Some(SecurityReportCliOperation {
                request,
                cancellation,
                handle: tokio::spawn(async move { Err(error) }),
            });
            tokio::task::yield_now().await;
            coordinator.poll(&mut app).await;
            assert!(
                matches!(
                    (&app.security.inventory, generation),
                    (yoctui_model::SecurityInventoryState::Cancelled { .. }, 2)
                        | (yoctui_model::SecurityInventoryState::TimedOut { .. }, 3)
                        | (yoctui_model::SecurityInventoryState::Lost { .. }, 4)
                        | (yoctui_model::SecurityInventoryState::Failed { .. }, 5)
                ),
                "generation {generation}: {:?}",
                app.security.inventory
            );
        }

        let first = SecurityReportRequest::new(6, vec![empty.clone()]).unwrap();
        app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
            request: first.clone(),
        };
        coordinator.begin_report_scan(first);
        let replacement = SecurityReportRequest::new(7, vec![empty]).unwrap();
        app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
            request: replacement.clone(),
        };
        coordinator.begin_report_scan(replacement);
        poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && app
                    .security
                    .inventory
                    .request()
                    .is_some_and(|request| request.generation == 7)
        })
        .await;
        assert_eq!(app.security.inventory.request().unwrap().generation, 7);
    }

    #[tokio::test]
    async fn security_workflow_cli_revalidates_exact_report_provider_and_advisory_opens() {
        let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
        let report = fixture.write_cve();
        let marker = fixture.root.join("opened-url");
        let opener = fixture.bin.join("xdg-open");
        fs::write(
            &opener,
            format!("#!/bin/sh\nprintf opened > '{}'\n", marker.display()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&opener).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&opener, permissions).unwrap();
        }
        let mut app = fixture.app();
        let mut coordinator =
            SecurityCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        let request = SecurityReportRequest::new(1, vec![report.clone()]).unwrap();
        app.security.inventory = yoctui_model::SecurityInventoryState::Loading {
            request: request.clone(),
        };
        coordinator.begin_report_scan(request);
        poll_security_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && app
                    .security
                    .inventory
                    .reports()
                    .is_some_and(|reports| !reports.is_empty())
        })
        .await;
        assert!(
            coordinator
                .revalidate_open_path(&app, &report)
                .await
                .is_ok()
        );
        assert!(
            coordinator
                .revalidate_open_path(&app, &fixture.provider)
                .await
                .is_err(),
            "a provider is authorized only by the selected exact Security scope"
        );

        let input =
            security_capability_input(&app, fixture.build.clone(), vec![fixture.bin.clone()])
                .unwrap();
        let capability = SecurityCapabilityInspector::new(input).inspect().unwrap();
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::CapabilityLoaded(capability)),
        );
        assert!(
            coordinator
                .revalidate_open_path(&app, &fixture.provider)
                .await
                .is_ok()
        );
        fs::write(&report, "{}").unwrap();
        assert!(
            coordinator
                .revalidate_open_path(&app, &report)
                .await
                .is_err(),
            "changed report identity must fail closed"
        );

        open_security_url(
            &mut app,
            coordinator.url_opener(),
            "https://example.invalid/CVE-2026-0001".into(),
        )
        .await;
        assert_eq!(fs::read_to_string(marker).unwrap(), "opened");
        open_security_url(
            &mut app,
            coordinator.url_opener(),
            "http://example.invalid/not-allowed".into(),
        )
        .await;
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("invalid"))
        );
    }

    #[tokio::test]
    async fn security_workflow_cli_reuses_managed_build_coordinator_and_attaches_job() {
        let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
        let mut app = fixture.app();
        let input =
            security_capability_input(&app, fixture.build.clone(), vec![fixture.bin.clone()])
                .unwrap();
        let capability = SecurityCapabilityInspector::new(input).inspect().unwrap();
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::CapabilityLoaded(capability)),
        );
        let _ = update(&mut app, Action::Security(SecurityAction::BeginCveCheck));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
            other => panic!("unexpected build dialog: {other:?}"),
        };
        let effect = update(
            &mut app,
            Action::Security(SecurityAction::ConfirmOperation(preview)),
        )
        .unwrap();
        let Effect::Security(SecurityEffect::StartBuild { id, request }) = effect else {
            panic!("Security build did not produce its typed effect");
        };
        let started = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut backend: Box<dyn BitBakeBackend> = Box::new(SecurityBuildBackend {
            started: started.clone(),
            fail_start: false,
        });
        let mut jobs = BuildJobCoordinator::default();
        assert!(begin_security_build(&mut backend, &mut app, &mut jobs, id, request.clone()).await);
        assert_eq!(*started.lock().unwrap(), [request]);
        let background_job_id = jobs.active_job_id().unwrap();
        assert_eq!(
            app.security.sessions.last().unwrap().background_job_id,
            Some(background_job_id)
        );
        assert_eq!(
            app.background_jobs.get(background_job_id).unwrap().kind,
            yoctui_model::BackgroundJobKind::CveCheck
        );
    }

    #[test]
    fn security_workflow_cli_correlates_managed_build_terminal_outcomes() {
        let fixture = SecurityCliFixture::new("#!/bin/sh\nexit 0\n");
        let mut app = fixture.app();
        let input =
            security_capability_input(&app, fixture.build.clone(), vec![fixture.bin.clone()])
                .unwrap();
        let capability = SecurityCapabilityInspector::new(input).inspect().unwrap();
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::CapabilityLoaded(capability)),
        );
        let _ = update(&mut app, Action::Security(SecurityAction::BeginCveCheck));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Security(yoctui_model::SecurityDialog::Operation(preview))) => preview,
            other => panic!("unexpected build dialog: {other:?}"),
        };
        let _ = update(
            &mut app,
            Action::Security(SecurityAction::ConfirmOperation(preview)),
        );
        let id = app.security.sessions.last().unwrap().preview.id;

        let started =
            security_build_action_for_event(&app, id, &BackendEvent::BuildStarted).unwrap();
        let _ = update(&mut app, started);
        assert_eq!(
            app.security.sessions.last().unwrap().status,
            SecuritySessionStatus::Running
        );

        let mut succeeded = app.clone();
        succeeded.screen = Screen::Logs;
        let action = security_build_action_for_event(
            &succeeded,
            id,
            &BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        )
        .unwrap();
        assert!(matches!(
            update(&mut succeeded, action),
            Some(Effect::Security(SecurityEffect::ImportReports(_)))
        ));
        assert_eq!(
            succeeded.security.sessions.last().unwrap().status,
            SecuritySessionStatus::Succeeded
        );
        assert_eq!(succeeded.screen, Screen::Logs);

        let mut failed = app.clone();
        let action = security_build_action_for_event(
            &failed,
            id,
            &BackendEvent::BuildCompleted {
                success: false,
                exit_code: Some(1),
            },
        )
        .unwrap();
        let _ = update(&mut failed, action);
        assert_eq!(
            failed.security.sessions.last().unwrap().status,
            SecuritySessionStatus::Failed
        );

        let mut cancelled = app.clone();
        cancelled.security.sessions.last_mut().unwrap().status = SecuritySessionStatus::Cancelling;
        let action = security_build_action_for_event(
            &cancelled,
            id,
            &BackendEvent::BuildCompleted {
                success: false,
                exit_code: Some(130),
            },
        )
        .unwrap();
        let _ = update(&mut cancelled, action);
        assert_eq!(
            cancelled.security.sessions.last().unwrap().status,
            SecuritySessionStatus::Cancelled
        );

        let mut lost = app;
        let action =
            security_build_action_for_event(&lost, id, &BackendEvent::Disconnected).unwrap();
        let _ = update(&mut lost, action);
        assert_eq!(
            lost.security.sessions.last().unwrap().status,
            SecuritySessionStatus::Lost
        );
    }

    struct QaCliFixture {
        root: PathBuf,
        build: PathBuf,
        reports: PathBuf,
        bin: PathBuf,
        provider: PathBuf,
        layer: PathBuf,
        source: PathBuf,
    }

    impl QaCliFixture {
        fn new(layer_runner_body: &str) -> Self {
            use std::sync::atomic::{AtomicU64, Ordering};
            static NEXT_QA_FIXTURE: AtomicU64 = AtomicU64::new(1);
            let root = std::env::temp_dir().join(format!(
                "yoctui-qa-cli-{}-{}",
                std::process::id(),
                NEXT_QA_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            let build = root.join("build");
            let reports = build.join("qa-reports");
            let bin = root.join("bin");
            let layer = root.join("meta-demo");
            let recipes = layer.join("recipes-core/busybox");
            for directory in [&build, &reports, &bin, &layer, &recipes] {
                fs::create_dir_all(directory).unwrap();
            }
            let provider = recipes.join("busybox_1.0.bb");
            fs::write(&provider, "SUMMARY = \"BusyBox\"\n").unwrap();
            let source = layer.join("conf/layer.conf");
            fs::create_dir_all(source.parent().unwrap()).unwrap();
            fs::write(&source, "LAYERSERIES_COMPAT_meta-demo = \"scarthgap\"\n").unwrap();
            let runner = bin.join("yocto-check-layer");
            fs::write(&runner, layer_runner_body).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut permissions = fs::metadata(&runner).unwrap().permissions();
                permissions.set_mode(0o755);
                fs::set_permissions(&runner, permissions).unwrap();
            }
            Self {
                root: fs::canonicalize(root).unwrap(),
                build: fs::canonicalize(build).unwrap(),
                reports: fs::canonicalize(reports).unwrap(),
                bin: fs::canonicalize(bin).unwrap(),
                provider: fs::canonicalize(provider).unwrap(),
                layer: fs::canonicalize(layer).unwrap(),
                source: fs::canonicalize(source).unwrap(),
            }
        }

        fn app(&self) -> App {
            let mut app = App::new(20, 8_000);
            app.screen = Screen::Qa;
            app.workspace.build_dir = Some(self.build.clone());
            app.workspace.release = Some("6.0".into());
            app.workspace.variables.insert(
                "PACKAGE_QA_REPORT_ROOT".into(),
                self.reports.display().to_string(),
            );
            app.workspace.variables.insert(
                "YOCTO_CHECK_LAYER_REPORT_ROOT".into(),
                self.reports.display().to_string(),
            );
            app.workspace.recipes.push(yoctui_model::Recipe {
                name: "busybox".into(),
                file: Some(self.provider.clone()),
                ..yoctui_model::Recipe::default()
            });
            app.workspace.layers.push(yoctui_model::Layer {
                name: "meta-demo".into(),
                path: self.layer.clone(),
                priority: Some(6),
            });
            app.recipe_metadata.insert(
                "busybox".into(),
                yoctui_model::RecipeMetadata {
                    recipe: "busybox".into(),
                    tasks: Some(vec![
                        "do_checkuri".into(),
                        "do_patch_qa".into(),
                        "do_populate_lic".into(),
                        "do_package_qa".into(),
                    ]),
                    ..yoctui_model::RecipeMetadata::default()
                },
            );
            app
        }

        fn write_report(&self) -> PathBuf {
            let report = self.reports.join("recipe-qa.json");
            fs::write(
                &report,
                format!(
                    r#"{{"findings":[{{"status":"warning","severity":"warning","message":"license checksum needs review","rule":"license-checksum","source":{{"path":"{}","line":1}}}}]}}"#,
                    self.source.display()
                ),
            )
            .unwrap();
            fs::canonicalize(report).unwrap()
        }
    }

    impl Drop for QaCliFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    async fn poll_qa_until(
        coordinator: &mut QaCliCoordinator,
        app: &mut App,
        complete: impl Fn(&App, &QaCliCoordinator) -> bool,
    ) {
        for _ in 0..300 {
            coordinator.poll(app).await;
            if complete(app, coordinator) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("QA CLI operation did not finish");
    }

    async fn inspect_qa_capability(
        fixture: &QaCliFixture,
        app: &mut App,
        coordinator: &mut QaCliCoordinator,
    ) {
        let effect = update(app, Action::Qa(QaAction::InspectCapability)).unwrap();
        assert!(coordinator.handle_effect(app, effect).await);
        poll_qa_until(coordinator, app, |app, _| {
            app.qa.capability.snapshot().is_some()
        })
        .await;
        assert_eq!(
            app.qa
                .selected_check()
                .and_then(|check| check.task.as_deref()),
            None,
            "the first kernel-only check stays disabled for a non-kernel recipe"
        );
        assert_eq!(app.qa.scope.as_ref().unwrap().recipe.file, fixture.provider);
    }

    #[tokio::test]
    async fn qa_workflow_cli_discovers_capability_imports_reports_and_preserves_navigation() {
        let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
        let report = fixture.write_report();
        let mut app = fixture.app();
        let mut coordinator =
            QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
        app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());

        let _ = update(&mut app, Action::Qa(QaAction::BeginImport));
        let effect = update(
            &mut app,
            Action::Qa(QaAction::ConfirmImport(report.display().to_string())),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        app.screen = Screen::Layers;
        poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && app
                    .qa
                    .inventory
                    .reports()
                    .is_some_and(|reports| !reports.is_empty())
        })
        .await;
        assert_eq!(app.screen, Screen::Layers);
        assert_eq!(app.qa.visible_findings().len(), 1);
        assert_eq!(
            app.qa.visible_findings()[0].message,
            "license checksum needs review"
        );
    }

    #[tokio::test]
    async fn qa_workflow_cli_runs_exact_layer_check_refreshes_and_rejects_duplicate() {
        let fixture =
            QaCliFixture::new("#!/bin/sh\nprintf 'checking configured layer\\n'\nexit 0\n");
        fixture.write_report();
        let mut app = fixture.app();
        let mut coordinator =
            QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
        let effect = update(&mut app, Action::Qa(QaAction::CycleView)).unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_qa_until(&mut coordinator, &mut app, |app, _| {
            app.qa.layer_capability.snapshot().is_some()
        })
        .await;
        let selected = app.qa.selected_layer().unwrap();
        assert_eq!(selected.identity.root, fixture.layer);
        assert!(matches!(
            selected.run,
            yoctui_model::QaLayerRunCapability::Available { .. }
        ));

        let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedLayerCheck));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Qa(yoctui_model::QaDialog::LayerOperation(preview))) => preview,
            other => panic!("unexpected layer-QA dialog: {other:?}"),
        };
        let effect = update(
            &mut app,
            Action::Qa(QaAction::ConfirmLayerOperation(preview)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect.clone()).await);
        assert!(coordinator.handle_effect(&mut app, effect).await);
        assert!(
            app.notification
                .as_deref()
                .is_some_and(|message| message.contains("already owned"))
        );
        app.screen = Screen::Dashboard;
        poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.layer.is_none()
                && coordinator.report.is_none()
                && app
                    .qa
                    .layer_sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        let session = app.qa.layer_sessions.back().unwrap();
        assert_eq!(session.status, QaSessionStatus::Succeeded);
        assert!(
            session
                .output
                .iter()
                .any(|line| line.line.contains("checking configured layer"))
        );
        assert!(app.qa.inventory.reports().is_some());
        assert_eq!(app.screen, Screen::Dashboard);
    }

    #[tokio::test]
    async fn qa_workflow_cli_cancels_only_exact_native_layer_session() {
        let fixture =
            QaCliFixture::new("#!/bin/sh\ntrap 'exit 0' TERM\nwhile :; do sleep 1; done\n");
        let mut app = fixture.app();
        let mut coordinator =
            QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
        let effect = update(&mut app, Action::Qa(QaAction::CycleView)).unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_qa_until(&mut coordinator, &mut app, |app, _| {
            app.qa.layer_capability.snapshot().is_some()
        })
        .await;
        let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedLayerCheck));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Qa(yoctui_model::QaDialog::LayerOperation(preview))) => preview,
            other => panic!("unexpected layer-QA dialog: {other:?}"),
        };
        let effect = update(
            &mut app,
            Action::Qa(QaAction::ConfirmLayerOperation(preview)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        let actual_id = app.qa.layer_sessions.back().unwrap().id;
        let _ = update(&mut app, Action::Qa(QaAction::BeginLayerCancellation));
        let effect = update(
            &mut app,
            Action::Qa(QaAction::ConfirmLayerCancellation(actual_id)),
        )
        .unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        assert!(!coordinator.owns_layer(QaLayerSessionId(actual_id.0 + 1)));
        poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.layer.is_none()
                && app
                    .qa
                    .layer_sessions
                    .back()
                    .is_some_and(|session| session.status.is_terminal())
        })
        .await;
        assert_eq!(
            app.qa.layer_sessions.back().unwrap().status,
            QaSessionStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn qa_workflow_cli_revalidates_exact_report_provider_source_and_layer_opens() {
        let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
        let report = fixture.write_report();
        let mut app = fixture.app();
        let mut coordinator =
            QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
        app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());
        let request = QaReportRequest::new(1, vec![report.clone()]).unwrap();
        app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
            request: request.clone(),
        };
        coordinator.begin_report_scan(&app, request);
        poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && app
                    .qa
                    .inventory
                    .reports()
                    .is_some_and(|reports| !reports.is_empty())
        })
        .await;
        let identity = app.qa.inventory.reports().unwrap()[0].identity.clone();
        let source = app.qa.inventory.reports().unwrap()[0].findings[0]
            .source
            .clone()
            .unwrap();
        assert!(coordinator.revalidate_report(&app, &identity).is_ok());
        assert!(
            coordinator
                .revalidate_provider(
                    &app,
                    &RecipeIdentity {
                        name: "busybox".into(),
                        file: fixture.provider.clone(),
                    }
                )
                .is_ok()
        );
        assert!(coordinator.revalidate_source(&app, &source).is_ok());

        let effect = update(&mut app, Action::Qa(QaAction::CycleView)).unwrap();
        assert!(coordinator.handle_effect(&mut app, effect).await);
        poll_qa_until(&mut coordinator, &mut app, |app, _| {
            app.qa.layer_capability.snapshot().is_some()
        })
        .await;
        let layer = app.qa.selected_layer().unwrap().identity.clone();
        assert!(coordinator.revalidate_layer(&app, &layer).is_ok());
        fs::write(&report, "{}").unwrap();
        assert!(coordinator.revalidate_report(&app, &identity).is_err());
        fs::remove_file(&fixture.source).unwrap();
        assert!(coordinator.revalidate_source(&app, &source).is_err());
    }

    #[tokio::test]
    async fn qa_workflow_cli_reuses_managed_build_and_correlates_terminal_outcomes() {
        let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
        fixture.write_report();
        let mut app = fixture.app();
        let mut coordinator =
            QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
        app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());
        let _ = update(&mut app, Action::Qa(QaAction::BeginSelectedCheck));
        let preview = match app.active_dialog().cloned() {
            Some(Dialog::Qa(yoctui_model::QaDialog::Operation(preview))) => preview,
            other => panic!("unexpected QA build dialog: {other:?}"),
        };
        let effect = update(&mut app, Action::Qa(QaAction::ConfirmOperation(preview))).unwrap();
        let Effect::Qa(QaEffect::StartBuild { session, request }) = effect else {
            panic!("QA build did not produce its typed effect");
        };
        let started = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let mut backend: Box<dyn BitBakeBackend> = Box::new(SecurityBuildBackend {
            started: started.clone(),
            fail_start: false,
        });
        let mut jobs = BuildJobCoordinator::default();
        assert!(begin_qa_build(&mut backend, &mut app, &mut jobs, session, request.clone()).await);
        assert_eq!(*started.lock().unwrap(), [request]);
        assert_eq!(
            app.qa.sessions.back().unwrap().background_job_id,
            jobs.active_job_id()
        );
        let started_action =
            qa_build_action_for_event(&app, session, &BackendEvent::BuildStarted).unwrap();
        let _ = update(&mut app, started_action);
        assert_eq!(
            app.qa.sessions.back().unwrap().status,
            QaSessionStatus::Running
        );
        app.screen = Screen::Logs;
        let completed_action = qa_build_action_for_event(
            &app,
            session,
            &BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
        )
        .unwrap();
        let followup = update(&mut app, completed_action);
        assert!(matches!(
            followup,
            Some(Effect::Qa(QaEffect::ImportReports(_)))
        ));
        assert_eq!(app.screen, Screen::Logs);

        let mut failed = fixture.app();
        inspect_qa_capability(&fixture, &mut failed, &mut coordinator).await;
        failed.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());
        let _ = update(&mut failed, Action::Qa(QaAction::BeginSelectedCheck));
        let preview = match failed.active_dialog().cloned() {
            Some(Dialog::Qa(yoctui_model::QaDialog::Operation(preview))) => preview,
            other => panic!("unexpected QA build dialog: {other:?}"),
        };
        let _ = update(&mut failed, Action::Qa(QaAction::ConfirmOperation(preview)));
        let failed_id = failed.qa.sessions.back().unwrap().id;
        let action = qa_build_action_for_event(
            &failed,
            failed_id,
            &BackendEvent::BuildCompleted {
                success: false,
                exit_code: Some(1),
            },
        )
        .unwrap();
        let _ = update(&mut failed, action);
        assert_eq!(
            failed.qa.sessions.back().unwrap().status,
            QaSessionStatus::Failed
        );
    }

    #[tokio::test]
    async fn qa_workflow_cli_preserves_report_terminal_states_and_replaceable_generations() {
        let fixture = QaCliFixture::new("#!/bin/sh\nexit 0\n");
        let empty = fixture.build.join("empty-qa");
        fs::create_dir(&empty).unwrap();
        let empty = fs::canonicalize(empty).unwrap();
        let mut app = fixture.app();
        let mut coordinator =
            QaCliCoordinator::new(fixture.build.clone(), vec![fixture.bin.clone()]);
        inspect_qa_capability(&fixture, &mut app, &mut coordinator).await;
        app.qa.check_selection = Some(QaCheckId::new("recipe-package".into()).unwrap());

        for (generation, error) in [
            (1, QaReportAdapterError::Cancelled),
            (2, QaReportAdapterError::Timeout(30)),
            (
                3,
                QaReportAdapterError::WorkerLost("worker channel closed".into()),
            ),
            (4, QaReportAdapterError::PermissionDenied(empty.clone())),
        ] {
            let request = QaReportRequest::new(generation, vec![empty.clone()]).unwrap();
            app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
                request: request.clone(),
            };
            coordinator.report = Some(QaReportCliOperation {
                request,
                cancellation: QaReportCancellation::default(),
                handle: tokio::spawn(async move { Err(error) }),
            });
            tokio::task::yield_now().await;
            coordinator.poll(&mut app).await;
            assert!(
                matches!(
                    (&app.qa.inventory, generation),
                    (yoctui_model::QaReportInventoryState::Cancelled { .. }, 1)
                        | (yoctui_model::QaReportInventoryState::TimedOut { .. }, 2)
                        | (yoctui_model::QaReportInventoryState::Lost { .. }, 3)
                        | (yoctui_model::QaReportInventoryState::Failed { .. }, 4)
                ),
                "generation {generation}: {:?}",
                app.qa.inventory
            );
        }

        let first = QaReportRequest::new(5, vec![empty.clone()]).unwrap();
        app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
            request: first.clone(),
        };
        coordinator.begin_report_scan(&app, first);
        let replacement = QaReportRequest::new(6, vec![empty]).unwrap();
        app.qa.inventory = yoctui_model::QaReportInventoryState::Loading {
            request: replacement.clone(),
        };
        coordinator.begin_report_scan(&app, replacement);
        poll_qa_until(&mut coordinator, &mut app, |app, coordinator| {
            coordinator.report.is_none()
                && app
                    .qa
                    .inventory
                    .request()
                    .is_some_and(|request| request.generation == 6)
        })
        .await;
        assert!(matches!(
            app.qa.inventory,
            yoctui_model::QaReportInventoryState::AvailableEmpty { .. }
        ));
    }

    #[test]
    fn qa_workflow_cli_routes_every_workspace_and_modal_key_without_leakage() {
        use yoctui_app::Input;
        let keys = [
            Input::Tab,
            Input::Up,
            Input::Down,
            Input::Char('s'),
            Input::Char('/'),
            Input::Char('f'),
            Input::Char('r'),
            Input::Char('I'),
            Input::Char('R'),
            Input::Enter,
            Input::Char('o'),
            Input::Char('e'),
            Input::Char('l'),
            Input::Char('c'),
        ];
        for key in keys {
            assert!(
                qa_workspace_action(yoctui_model::QaView::RecipeKernel, false, false, key)
                    .is_some(),
                "unrouted QA workspace key: {key:?}"
            );
        }
        let dialog = yoctui_model::QaDialog::Import {
            input: String::new(),
        };
        assert!(qa_dialog_action(&dialog, Input::Char('x')).is_some());
        assert!(qa_dialog_action(&dialog, Input::Backspace).is_some());
        assert!(qa_dialog_action(&dialog, Input::Enter).is_some());
        assert!(qa_dialog_action(&dialog, Input::Esc).is_some());
        assert!(qa_dialog_action(&dialog, Input::Tab).is_none());
    }
}
