use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratabake_app::{Input, key_action};
use ratabake_bitbake::{BitBakeBackend, BridgeBackend, ProcessBackend};
use ratabake_model::{Action, App, BuildRequest, update};
use ratabake_ui::render;
use ratatui::Terminal;
use serde::Deserialize;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    time::Duration,
};
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

#[derive(Debug, Default, Deserialize)]
struct FileConfig {
    backend: Option<Backend>,
    build_dir: Option<PathBuf>,
    log_retention_entries: Option<usize>,
    log_retention_bytes: Option<usize>,
    refresh_ms: Option<u64>,
    default_target: Option<String>,
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
            .map(|p| p.join("ratabake/config.toml"))
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
    let environment_backend = env::var("RATABAKE_BACKEND")
        .ok()
        .map(|value| {
            Backend::from_str(&value, true)
                .map_err(|_| anyhow::anyhow!("RATABAKE_BACKEND must be bridge or process"))
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
        .or_else(|| env::var_os("RATABAKE_BUILD_DIR").map(PathBuf::from))
        .or(file.build_dir)
        .unwrap_or(env::current_dir()?);
    let log_entries = env_usize("RATABAKE_LOG_RETENTION_ENTRIES")?
        .or(file.log_retention_entries)
        .unwrap_or(10_000);
    let log_bytes = env_usize("RATABAKE_LOG_RETENTION_BYTES")?
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
        default_target: env::var("RATABAKE_DEFAULT_TARGET")
            .ok()
            .or(file.default_target),
        log_level: cli
            .log_level
            .clone()
            .or_else(|| env::var("RATABAKE_LOG_LEVEL").ok())
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
    if matches!(config.backend, Backend::Bridge) {
        eprintln!(
            "Bridge backend requires an active BitBake Python environment; use --backend process for Knotty fallback."
        )
    }
    tui(
        build_dir,
        targets,
        config.log_entries,
        config.log_bytes,
        config.refresh,
        config.color,
    )
    .await
}
async fn doctor(build_dir: &Path) -> Result<()> {
    let initialized = std::env::var_os("BUILDDIR").is_some();
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
    let mut app = App::new(log_entries, log_bytes);
    let _ = backend.inspect_workspace().await?;
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
            ratabake_bitbake::BackendEvent::Log(l) => {
                println!("{}", l.message);
                update(&mut app, Action::Log(l));
            }
            ratabake_bitbake::BackendEvent::BuildCompleted { success } => {
                println!("build {}", if success { "completed" } else { "failed" });
                if success {
                    return Ok(());
                }
                return Err(anyhow::anyhow!("BitBake build failed"));
            }
            _ => {}
        }
    }
}

async fn select_backend(backend: Backend, build_dir: PathBuf) -> Result<Box<dyn BitBakeBackend>> {
    match backend {
        Backend::Process => Ok(Box::new(ProcessBackend::new(build_dir))),
        Backend::Bridge => {
            let script = env::var_os("RATABAKE_BRIDGE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                        .join("../..")
                        .join("bridge/ratabake_bridge.py")
                });
            let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
            BridgeBackend::spawn(&python, script, build_dir)
                .await
                .map(|backend| Box::new(backend) as Box<dyn BitBakeBackend>)
                .context("could not start the BitBake bridge; source oe-init-build-env or use --backend process")
        }
    }
}
async fn tui(
    _build_dir: PathBuf,
    targets: Vec<String>,
    log_entries: usize,
    log_bytes: usize,
    refresh: Duration,
    _color: bool,
) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(log_entries, log_bytes);
    if !targets.is_empty() {
        app.build.target = targets.first().cloned()
    }
    loop {
        terminal.draw(|f| render(f, &app))?;
        if event::poll(refresh)?
            && let Event::Key(k) = event::read()?
        {
            let input = match k.code {
                KeyCode::Char(c) => Input::Char(c),
                KeyCode::Esc => Input::Esc,
                KeyCode::Enter => Input::Enter,
                _ => continue,
            };
            if let Some(a) = key_action(input) {
                let _ = update(&mut app, a);
            }
        }
        if app.should_quit {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_retention_and_backend_settings() {
        let config: FileConfig = toml::from_str(
            "backend = 'process'\nlog_retention_entries = 42\nlog_retention_bytes = 1024\nrefresh_ms = 50\ndefault_target = 'core-image-minimal'",
        )
        .unwrap();
        assert!(matches!(config.backend, Some(Backend::Process)));
        assert_eq!(config.log_retention_entries, Some(42));
        assert_eq!(config.default_target.as_deref(), Some("core-image-minimal"));
    }
}
