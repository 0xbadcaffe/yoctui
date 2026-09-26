//! Session store.
use super::*;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct Session {
    #[serde(default)]
    pub(crate) preferences: Option<WorkbenchPreferences>,
    #[serde(default)]
    pub(crate) last_target: Option<String>,
    #[serde(default)]
    pub(crate) last_screen: Option<Screen>,
    #[serde(default)]
    pub(crate) log_filter: Option<Severity>,
    #[serde(default)]
    pub(crate) log_recipe_filter: Option<String>,
    #[serde(default)]
    pub(crate) log_task_filter: Option<String>,
    #[serde(default)]
    pub(crate) log_build_filter: Option<String>,
    #[serde(default)]
    pub(crate) log_wrap: Option<bool>,
    #[serde(default)]
    pub(crate) log_follow: Option<bool>,
    #[serde(default)]
    pub(crate) theme: Option<Theme>,
    #[serde(default)]
    pub(crate) animation_speed: Option<AnimationSpeed>,
    #[serde(default)]
    pub(crate) reduced_motion: Option<bool>,
    #[serde(default)]
    pub(crate) color_enabled: Option<bool>,
    #[serde(default)]
    pub(crate) last_backend: Option<Backend>,
    #[serde(default)]
    pub(crate) recent_build_dirs: Vec<PathBuf>,
    #[serde(default)]
    pub(crate) pane_layout: Option<yoctui_model::PaneLayout>,
    #[serde(default)]
    pub(crate) raw_favorites: Vec<yoctui_model::RawFavorite>,
    #[serde(default)]
    pub(crate) hardware_documents: Vec<yoctui_model::HardwareDocument>,
    #[serde(default)]
    pub(crate) hardware_last_directory: Option<PathBuf>,
    #[serde(default)]
    pub(crate) keymap: yoctui_model::KeymapPreferences,
    #[serde(default)]
    pub(crate) onboarding: Option<OnboardingProgress>,
}

pub(crate) const MAX_SESSION_BYTES: u64 = 1024 * 1024;

pub(crate) static NEXT_SESSION_TEMPORARY: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

pub(crate) fn read_session(path: Option<&Path>) -> Result<Session> {
    let Some(path) = path else {
        return Ok(Session::default());
    };
    if !path.exists() {
        return Ok(Session::default());
    }
    let metadata = fs::symlink_metadata(path)
        .with_context(|| format!("could not inspect session file {}", path.display()))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        anyhow::bail!("session file must be a regular non-symlink file");
    }
    if metadata.len() > MAX_SESSION_BYTES {
        anyhow::bail!("session file exceeds the 1 MiB limit");
    }
    let text = fs::read_to_string(path)
        .with_context(|| format!("could not read session file {}", path.display()))?;
    let mut session: Session = toml::from_str(&text)
        .with_context(|| format!("invalid session file {}", path.display()))?;
    session.keymap = std::mem::take(&mut session.keymap)
        .migrate()
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("invalid keymap in session file {}", path.display()))?;
    if let Some(preferences) = session.preferences.as_mut() {
        preferences.keymap = std::mem::take(&mut preferences.keymap)
            .migrate()
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("invalid preference keymap in {}", path.display()))?;
        preferences
            .validate()
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("invalid preferences in {}", path.display()))?;
    }
    validate_raw_favorites(&session.raw_favorites)
        .map_err(anyhow::Error::msg)
        .with_context(|| format!("invalid Raw favorites in session file {}", path.display()))?;
    yoctui_model::validate_hardware_documents(&session.hardware_documents)
        .map_err(anyhow::Error::msg)
        .with_context(|| {
            format!(
                "invalid Hardware library in session file {}",
                path.display()
            )
        })?;
    if let Some(onboarding) = session.onboarding.as_ref() {
        onboarding
            .validate()
            .map_err(anyhow::Error::msg)
            .with_context(|| format!("invalid onboarding progress in {}", path.display()))?;
    }
    Ok(session)
}

pub(crate) fn write_session(path: Option<&Path>, session: &Session) -> Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    if let Some(directory) = path.parent() {
        fs::create_dir_all(directory).with_context(|| {
            format!("could not create session directory {}", directory.display())
        })?;
    }
    validate_raw_favorites(&session.raw_favorites)
        .map_err(anyhow::Error::msg)
        .context("invalid Raw favorites cannot be persisted")?;
    yoctui_model::validate_hardware_documents(&session.hardware_documents)
        .map_err(anyhow::Error::msg)
        .context("invalid Hardware library cannot be persisted")?;
    yoctui_model::EffectiveKeymap::from_preferences(&session.keymap)
        .map_err(anyhow::Error::msg)
        .context("invalid keymap cannot be persisted")?;
    if let Some(preferences) = session.preferences.as_ref() {
        preferences
            .validate()
            .map_err(anyhow::Error::msg)
            .context("invalid workbench preferences cannot be persisted")?;
    }
    if let Some(onboarding) = session.onboarding.as_ref() {
        onboarding
            .validate()
            .map_err(anyhow::Error::msg)
            .context("invalid onboarding progress cannot be persisted")?;
    }
    let text = toml::to_string(session)?;
    if text.len() as u64 > MAX_SESSION_BYTES {
        anyhow::bail!("session file exceeds the 1 MiB limit");
    }
    let temporary = path.with_extension(format!(
        "toml.{}.{}.tmp",
        std::process::id(),
        NEXT_SESSION_TEMPORARY.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let result = (|| -> Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .with_context(|| format!("could not create session file {}", temporary.display()))?;
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)
            .with_context(|| format!("could not replace session file {}", path.display()))?;
        if let Some(directory) = path.parent() {
            fs::File::open(directory)?.sync_all()?;
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

pub(crate) fn persist_raw_favorites(
    path: Option<&Path>,
    session: &mut Session,
    favorites: &[yoctui_model::RawFavorite],
) -> Result<()> {
    validate_raw_favorites(favorites).map_err(anyhow::Error::msg)?;
    let mut updated = session.clone();
    updated.raw_favorites = favorites.to_vec();
    write_session(path, &updated)?;
    *session = updated;
    Ok(())
}

pub(crate) fn install_session_raw_favorites(session: &Session, app: &mut App) -> Result<()> {
    validate_raw_favorites(&session.raw_favorites).map_err(anyhow::Error::msg)?;
    app.raw_mode.favorites.clone_from(&session.raw_favorites);
    Ok(())
}

pub(crate) fn install_session_hardware(session: &Session, app: &mut App) -> Result<()> {
    yoctui_model::validate_hardware_documents(&session.hardware_documents)
        .map_err(anyhow::Error::msg)?;
    app.hardware
        .documents
        .clone_from(&session.hardware_documents);
    app.hardware.missing_paths = app
        .hardware
        .documents
        .iter()
        .filter(|document| {
            std::fs::symlink_metadata(&document.path)
                .map(|metadata| metadata.file_type().is_symlink() || !metadata.is_file())
                .unwrap_or(true)
        })
        .map(|document| document.path.clone())
        .collect();
    app.hardware.last_directory = session.hardware_last_directory.clone().or_else(|| {
        std::env::var_os("HOME")
            .filter(|home| !home.is_empty())
            .map(PathBuf::from)
    });
    Ok(())
}

pub(crate) fn persist_hardware(
    path: Option<&Path>,
    session: &mut Session,
    app: &App,
) -> Result<()> {
    yoctui_model::validate_hardware_documents(&app.hardware.documents)
        .map_err(anyhow::Error::msg)?;
    let mut updated = session.clone();
    updated
        .hardware_documents
        .clone_from(&app.hardware.documents);
    updated.hardware_last_directory = app.hardware.last_directory.clone();
    write_session(path, &updated)?;
    *session = updated;
    Ok(())
}

pub(crate) fn session_preferences(session: &Session) -> Result<WorkbenchPreferences> {
    if let Some(preferences) = session.preferences.as_ref() {
        preferences.validate().map_err(anyhow::Error::msg)?;
        return Ok(preferences.clone());
    }
    let preferences = WorkbenchPreferences {
        theme: session.theme.unwrap_or_default(),
        animation_speed: session.animation_speed.unwrap_or_default(),
        reduced_motion: session.reduced_motion.unwrap_or(false),
        color_enabled: session.color_enabled.unwrap_or(true),
        log_wrap: session.log_wrap.unwrap_or(false),
        log_follow: session.log_follow.unwrap_or(true),
        keymap: session.keymap.clone(),
        ..WorkbenchPreferences::default()
    };
    preferences.validate().map_err(anyhow::Error::msg)?;
    Ok(preferences)
}

pub(crate) fn install_session_onboarding(session: &Session, app: &mut App) -> Result<()> {
    let first_run = session.onboarding.is_none();
    app.onboarding
        .install(session.onboarding.clone().unwrap_or_default(), false)
        .map_err(anyhow::Error::msg)?;
    if first_run {
        let _ = update(app, Action::OpenOnboarding);
    }
    Ok(())
}

pub(crate) fn persist_onboarding(
    path: Option<&Path>,
    session: &mut Session,
    app: &App,
) -> Result<()> {
    app.onboarding
        .progress
        .validate()
        .map_err(anyhow::Error::msg)?;
    let mut updated = session.clone();
    updated.onboarding = Some(app.onboarding.progress.clone());
    write_session(path, &updated)?;
    *session = updated;
    Ok(())
}

pub(crate) fn persist_settings(
    path: Option<&Path>,
    session: &mut Session,
    app: &App,
    persist_color: bool,
) -> Result<()> {
    let mut updated = session.clone();
    let mut preferences = app.effective_preferences();
    if !persist_color {
        preferences.color_enabled = app.preferences.color_enabled;
    }
    updated.preferences = Some(preferences.clone());
    updated.pane_layout = preferences
        .remember_pane_sizes
        .then(|| app.pane_layout.clone());
    updated.theme = None;
    updated.animation_speed = None;
    updated.reduced_motion = None;
    updated.color_enabled = None;
    updated.log_wrap = None;
    updated.log_follow = None;
    updated.keymap = yoctui_model::KeymapPreferences::default();
    write_session(path, &updated)?;
    *session = updated;
    Ok(())
}
