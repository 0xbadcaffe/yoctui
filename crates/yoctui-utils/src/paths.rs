use std::path::{Path, PathBuf};

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
mod tests {
    use super::*;

    #[test]
    fn xdg_overrides_require_absolute_paths_and_preserve_non_ascii_paths() {
        let home = std::env::temp_dir().join("home with spaces-λ");
        let custom = std::env::temp_dir().join("custom-λ");
        assert_eq!(
            xdg_directory(Some(custom.clone()), Some(home.clone()), ".config"),
            Some(custom)
        );
        for invalid in [PathBuf::new(), PathBuf::from("relative")] {
            assert_eq!(
                xdg_directory(Some(invalid), Some(home.clone()), ".config"),
                Some(home.join(".config"))
            );
        }
        assert_eq!(
            xdg_directory(None, Some("relative".into()), ".config"),
            None
        );
        assert_eq!(xdg_directory(None, None, ".config"), None);
    }
}
