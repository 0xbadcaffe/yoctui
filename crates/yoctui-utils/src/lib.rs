use std::path::{Path, PathBuf};

pub fn home_dir() -> PathBuf {
    dirs::home_dir()
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn home_path<P: AsRef<Path>>(path: P) -> PathBuf {
    home_dir().join(path)
}