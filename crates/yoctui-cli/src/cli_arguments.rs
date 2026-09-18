//! Cli arguments.
use super::*;

#[derive(Parser, Debug)]
#[command(version, about = "A Terminal Workbanch for Yocto/BitBake")]
pub(crate) struct Cli {
    #[arg(long, value_enum)]
    pub(crate) backend: Option<Backend>,
    #[arg(long)]
    pub(crate) build_dir: Option<PathBuf>,
    #[arg(long)]
    pub(crate) config: Option<PathBuf>,
    #[arg(long)]
    pub(crate) log_level: Option<String>,
    #[arg(long)]
    pub(crate) no_color: bool,
    #[arg(long)]
    pub(crate) headless: bool,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
    pub(crate) targets: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Backend {
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

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
    Inspect,
    Profile,
    Build {
        targets: Vec<String>,
    },
    Recipes,
    Layers,
    Config {
        name: String,
    },
    Doctor {
        /// Emit the bounded daemon-owned compatibility report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Attach the interactive client to the persistent daemon.
    Attach,
    /// List daemon-owned terminal sessions.
    Sessions,
    /// Manage one daemon-owned terminal session.
    Session {
        #[command(subcommand)]
        command: SessionCliCommand,
    },
    Daemon {
        #[command(subcommand)]
        command: DaemonCliCommand,
    },
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionCliCommand {
    Attach {
        id: u64,
    },
    Kill {
        id: u64,
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub(crate) enum DaemonCliCommand {
    Start,
    Build {
        targets: Vec<String>,
    },
    Status,
    Stop,
    Restart,
    Foreground,
    Service {
        #[command(subcommand)]
        command: DaemonServiceCommand,
    },
}

#[derive(Subcommand, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DaemonServiceCommand {
    Install,
    Uninstall,
    Start,
    Stop,
    Restart,
    Status,
}
