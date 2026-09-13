use std::env;
use std::path::PathBuf;

/// Read a filesystem path from an environment variable.
pub fn env_path(name: &str) -> PathBuf {
    env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} environment variable is not set"))
}