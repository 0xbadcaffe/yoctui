use std::{
    io,
    path::{Path, PathBuf},
};

fn absolute(path: Option<PathBuf>) -> Option<PathBuf> {
    path.filter(|path| path.is_absolute())
}

/// Resolve the user's home, falling back to the current directory for browsing.
/// Do not use this fallback to choose a persistent configuration destination.
pub fn home_dir() -> PathBuf {
    absolute(dirs::home_dir())
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from(std::path::MAIN_SEPARATOR_STR))
}

pub fn home_path<P: AsRef<Path>>(path: P) -> PathBuf {
    home_dir().join(path)
}

/// Report whether any filesystem entry occupies `path`, including a dangling
/// symbolic link. Unlike `Path::exists`, this does not follow the final link.
pub fn path_entry_exists(path: &Path) -> io::Result<bool> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn xdg_directory(value: Option<PathBuf>, home: Option<PathBuf>, suffix: &str) -> Option<PathBuf> {
    absolute(value).or_else(|| absolute(home).map(|home| home.join(suffix)))
}

/// XDG configuration root. Relative or empty overrides are ignored.
pub fn config_dir() -> Option<PathBuf> {
    xdg_directory(
        std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
        dirs::home_dir(),
        ".config",
    )
}

/// XDG state root. Relative or empty overrides are ignored.
pub fn state_dir() -> Option<PathBuf> {
    xdg_directory(
        std::env::var_os("XDG_STATE_HOME").map(PathBuf::from),
        dirs::home_dir(),
        ".local/state",
    )
}

#[cfg(test)]
#[path = "tests/paths/mod.rs"]
mod tests;
