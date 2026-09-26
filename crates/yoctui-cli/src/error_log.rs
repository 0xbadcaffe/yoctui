//! Bounded client-local reads for the Errors workspace source-log viewer.
use anyhow::{Context, Result, ensure};
use std::{fs, io::Read, path::PathBuf};
use yoctui_model::{Action, App};

const MAX_ERROR_LOG_BYTES: u64 = 8 * 1024 * 1024;

fn read(path: &PathBuf) -> Result<String> {
    let metadata =
        fs::symlink_metadata(path).with_context(|| format!("cannot inspect {}", path.display()))?;
    ensure!(
        !metadata.file_type().is_symlink(),
        "source log is a symbolic link"
    );
    ensure!(metadata.is_file(), "source log is not a regular file");
    ensure!(
        metadata.len() <= MAX_ERROR_LOG_BYTES,
        "source log exceeds the 8 MiB viewer limit"
    );
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .with_context(|| format!("cannot open {}", path.display()))?
        .take(MAX_ERROR_LOG_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_ERROR_LOG_BYTES,
        "source log exceeds the 8 MiB viewer limit"
    );
    String::from_utf8(bytes).context("source log is not UTF-8 text")
}

pub(crate) async fn load_error_log(app: &mut App, path: PathBuf) {
    let requested = path.clone();
    let result = tokio::task::spawn_blocking(move || read(&path)).await;
    let action = match result {
        Ok(Ok(content)) => Action::ErrorLogLoaded {
            path: requested,
            content,
        },
        Ok(Err(error)) => Action::ErrorLogLoadFailed {
            path: requested,
            message: error.to_string(),
        },
        Err(error) => Action::ErrorLogLoadFailed {
            path: requested,
            message: format!("source log reader failed: {error}"),
        },
    };
    let _ = yoctui_model::update(app, action);
}

#[cfg(test)]
#[path = "tests/error_log/mod.rs"]
mod tests;
