use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratabake_app::{Input, key_action};
use ratabake_bitbake::{BitBakeBackend, ProcessBackend};
use ratabake_model::{Action, App, BuildRequest, update};
use ratabake_ui::render;
use ratatui::Terminal;
use std::{
    io,
    path::{Path, PathBuf},
    time::Duration,
};
#[derive(Parser, Debug)]
#[command(about = "A Ratatui frontend and control client for BitBake")]
struct Cli {
    #[arg(long, value_enum, default_value = "bridge")]
    backend: Backend,
    #[arg(long)]
    build_dir: Option<PathBuf>,
    #[arg(long)]
    config: Option<PathBuf>,
    #[arg(long, default_value = "info")]
    log_level: String,
    #[arg(long)]
    no_color: bool,
    #[arg(long)]
    headless: bool,
    #[command(subcommand)]
    command: Option<Command>,
    targets: Vec<String>,
}
#[derive(Clone, Debug, ValueEnum)]
enum Backend {
    Bridge,
    Process,
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
        execute!(io::stdout(), EnterAlternateScreen)?;
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}
fn config_path(cli: &Cli) -> Option<PathBuf> {
    cli.config.clone().or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME").map(|p| PathBuf::from(p).join("ratabake/config.toml"))
    })
}
#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .with_env_filter(cli.log_level.clone())
        .with_writer(std::io::stderr)
        .init();
    let build_dir = cli.build_dir.clone().unwrap_or(std::env::current_dir()?);
    if matches!(cli.command, Some(Command::Doctor)) {
        return doctor(&build_dir).await;
    }
    let targets = match &cli.command {
        Some(Command::Build { targets }) => targets.clone(),
        _ => cli.targets.clone(),
    };
    if cli.headless {
        return headless(build_dir, targets).await;
    }
    if matches!(cli.backend, Backend::Bridge) {
        eprintln!(
            "Bridge backend requires an active BitBake Python environment; use --backend process for Knotty fallback."
        )
    }
    let _config = config_path(&cli);
    tui(build_dir, targets).await
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
async fn headless(build_dir: PathBuf, targets: Vec<String>) -> Result<()> {
    let mut backend = ProcessBackend::new(build_dir);
    let mut app = App::new(10_000, 16 * 1024 * 1024);
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
async fn tui(_build_dir: PathBuf, targets: Vec<String>) -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(ratatui::backend::CrosstermBackend::new(io::stdout()))?;
    let mut app = App::new(10_000, 16 * 1024 * 1024);
    if !targets.is_empty() {
        app.build.target = targets.first().cloned()
    }
    loop {
        terminal.draw(|f| render(f, &app))?;
        if event::poll(Duration::from_millis(100))?
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
