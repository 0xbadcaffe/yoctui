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
use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::Command as ProcessCommand,
    time::Duration,
};
#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};
use yoctui_app::{Input, key_action};
use yoctui_bitbake::{BackendEvent, BitBakeBackend, BridgeBackend, ProcessBackend};
use yoctui_model::{Action, App, AppError, BuildRequest, Effect, TaskId, TaskInfo, update};
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
#[derive(Clone, Debug, Deserialize, ValueEnum)]
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
    default_target: Option<String>,
    editor: Option<String>,
    color: Option<bool>,
}

#[derive(Debug)]
struct Config {
    backend: Backend,
    build_dir: PathBuf,
    log_entries: usize,
    log_bytes: usize,
    refresh: Duration,
    default_target: Option<String>,
    editor: Option<String>,
    log_level: String,
    color: bool,
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

fn resolve_config(cli: &Cli) -> Result<Config> {
    let file = read_file_config(config_path(cli).as_deref())?;
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
        .unwrap_or(Backend::Bridge);
    let build_dir = cli
        .build_dir
        .clone()
        .or_else(|| env::var_os("YOCTUI_BUILD_DIR").map(PathBuf::from))
        .or(file.build_dir)
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
    Ok(Config {
        backend,
        build_dir,
        log_entries,
        log_bytes,
        refresh: Duration::from_millis(file.refresh_ms.unwrap_or(100).max(16)),
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
    })
}
#[tokio::main]
async fn main() -> Result<()> {
    install_panic_hook();
    let cli = Cli::parse();
    let config = resolve_config(&cli)?;
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
        _ => config.default_target.clone().into_iter().collect(),
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
    tui(config, targets).await
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
    let value = result?;
    let value = value
        .as_deref()
        .with_context(|| format!("{name} is not available from the selected backend"))?;
    shutdown?;
    println!("{name}={value}");
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
                    let _ = update(&mut app, Action::BuildCompleted { success });
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
        BackendEvent::ParseProgress => Some(Action::ParseProgress),
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
        | BackendEvent::Log(_)
        | BackendEvent::BuildCompleted { .. } => None,
    }
}

async fn select_backend(backend: Backend, build_dir: PathBuf) -> Result<Box<dyn BitBakeBackend>> {
    match backend {
        Backend::Process => Ok(Box::new(ProcessBackend::new(build_dir))),
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

async fn tui(config: Config, targets: Vec<String>) -> Result<()> {
    let Config {
        backend: backend_kind,
        build_dir,
        log_entries,
        log_bytes,
        refresh,
        color: _color,
        editor,
        ..
    } = config;
    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(log_entries, log_bytes);
    app.backend = backend_kind.to_string();
    let mut backend = select_backend(backend_kind, build_dir).await?;
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
    #[cfg(unix)]
    let mut termination = termination_receiver()?;
    loop {
        #[cfg(unix)]
        if termination_requested(&mut termination) {
            break;
        }
        terminal.draw(|f| render(f, &app))?;
        if event::poll(refresh)?
            && let Event::Key(k) = event::read()?
        {
            let Some(input) = input_from_key(k) else {
                continue;
            };
            if app.recipe_task_confirmation.is_some() {
                let effect = match input {
                    Input::Enter => update(&mut app, Action::ConfirmRecipeTask),
                    Input::Esc => update(&mut app, Action::CancelRecipeTask),
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
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('b') {
                let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
            } else if input == Input::Char('b') {
                let _ = update(&mut app, Action::BeginBuildTargetEdit);
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
            } else if app.screen == yoctui_model::Screen::Recipes && input == Input::Char('S') {
                let _ = update(&mut app, Action::BeginSelectedRecipeCleanState);
            } else if app.screen == yoctui_model::Screen::Layers
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectLayer { delta });
            } else if app.screen == yoctui_model::Screen::Layers && input == Input::Char('o') {
                if let Some(Effect::OpenInEditor(path)) =
                    update(&mut app, Action::OpenSelectedLayer)
                {
                    open_in_editor(&guard, &mut app, path, editor.as_deref()).await;
                }
            } else if app.screen == yoctui_model::Screen::Configuration
                && matches!(input, Input::Up | Input::Down)
            {
                let delta = if input == Input::Up { -1 } else { 1 };
                let _ = update(&mut app, Action::SelectConfigVariable { delta });
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
                BackendEvent::BuildCompleted { success, .. } => {
                    let _ = update(&mut app, Action::BuildCompleted { success });
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
            "backend = 'process'\nlog_retention_entries = 42\nlog_retention_bytes = 1024\nrefresh_ms = 50\ndefault_target = 'core-image-minimal'\neditor = 'nano'",
        )
        .unwrap();
        assert!(matches!(config.backend, Some(Backend::Process)));
        assert_eq!(config.log_retention_entries, Some(42));
        assert_eq!(config.default_target.as_deref(), Some("core-image-minimal"));
        assert_eq!(config.editor.as_deref(), Some("nano"));
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
}
