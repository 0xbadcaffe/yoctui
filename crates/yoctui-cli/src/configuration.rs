//! Configuration.
use super::*;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct FileConfig {
    pub(crate) backend: Option<Backend>,
    pub(crate) build_dir: Option<PathBuf>,
    pub(crate) log_retention_entries: Option<usize>,
    pub(crate) log_retention_bytes: Option<usize>,
    pub(crate) refresh_ms: Option<u64>,
    pub(crate) cancellation_timeout_ms: Option<u64>,
    pub(crate) default_target: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) color: Option<bool>,
    pub(crate) theme: Option<Theme>,
    pub(crate) animation_speed: Option<AnimationSpeed>,
    pub(crate) reduced_motion: Option<bool>,
}

#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) backend: Backend,
    pub(crate) build_dir: PathBuf,
    pub(crate) build_dir_configured: bool,
    pub(crate) log_entries: usize,
    pub(crate) log_bytes: usize,
    pub(crate) refresh: Duration,
    pub(crate) cancellation_timeout: Duration,
    pub(crate) default_target: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) log_level: String,
    pub(crate) color: bool,
    pub(crate) color_forced_off: bool,
    pub(crate) theme: Theme,
    pub(crate) animation_speed: AnimationSpeed,
    pub(crate) reduced_motion: bool,
    pub(crate) preferences: WorkbenchPreferences,
    pub(crate) session_path: Option<PathBuf>,
}

pub(crate) fn config_path(cli: &Cli) -> Option<PathBuf> {
    cli.config
        .clone()
        .or_else(|| yoctui_utils::config_dir().map(|p| p.join("yoctui/config.toml")))
}

pub(crate) fn read_file_config(path: Option<&Path>) -> Result<FileConfig> {
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

pub(crate) fn session_path(config: Option<&Path>) -> Option<PathBuf> {
    config
        .and_then(Path::parent)
        .map(|directory| directory.join("session.toml"))
}

pub(crate) fn env_usize(name: &str) -> Result<Option<usize>> {
    env::var(name)
        .ok()
        .map(|value| {
            value
                .parse()
                .with_context(|| format!("{name} must be a positive integer"))
        })
        .transpose()
}

pub(crate) fn resolve_config(cli: &Cli, session: &Session) -> Result<Config> {
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
    let mut preferences = session_preferences(session)?;
    if session.preferences.is_none() {
        if session.theme.is_none() {
            preferences.theme = file.theme.unwrap_or_default();
        }
        if session.animation_speed.is_none() {
            preferences.animation_speed = file.animation_speed.unwrap_or_default();
        }
        if session.reduced_motion.is_none() {
            preferences.reduced_motion = file.reduced_motion.unwrap_or(false);
        }
        if session.color_enabled.is_none() {
            preferences.color_enabled = file.color.unwrap_or(true);
        }
    }
    preferences.validate().map_err(anyhow::Error::msg)?;
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
        color: !cli.no_color && preferences.color_enabled,
        color_forced_off: cli.no_color,
        theme: preferences.theme,
        animation_speed: preferences.animation_speed,
        reduced_motion: preferences.reduced_motion,
        preferences,
        session_path: session_path(configured_path.as_deref()),
    })
}
