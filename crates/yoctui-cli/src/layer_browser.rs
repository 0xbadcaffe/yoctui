//! Layer browser.
use super::*;

pub(crate) fn scan_layer_directory(
    scan: &Path,
    inspect_git: bool,
) -> io::Result<Vec<LayerBrowserEntry>> {
    let git_output = inspect_git
        .then(|| {
            ProcessCommand::new("git")
                .args([
                    "status",
                    "--porcelain=v1",
                    "--ignored",
                    "--untracked-files=all",
                    "--",
                    ".",
                ])
                .current_dir(scan)
                .output()
                .ok()
                .filter(|output| output.status.success())
        })
        .flatten();
    let git_lines = git_output.as_ref().map(|output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let status = line.get(..2)?;
                let path = line.get(3..)?;
                Some((status.to_owned(), scan.join(path)))
            })
            .collect::<Vec<_>>()
    });
    let mut entries = fs::read_dir(scan)?
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let name = entry.file_name();
            if name == ".git" {
                return None;
            }
            let metadata = entry.metadata().ok();
            let is_dir = metadata.as_ref().is_some_and(|value| value.is_dir());
            let git = git_lines
                .as_ref()
                .map_or(GitFileState::Unavailable, |lines| {
                    lines
                        .iter()
                        .find(|(_, changed)| {
                            changed == &path || (is_dir && changed.starts_with(&path))
                        })
                        .map_or(GitFileState::Clean, |(status, _)| match status.as_str() {
                            "??" => GitFileState::Untracked,
                            "!!" => GitFileState::Ignored,
                            _ => GitFileState::Modified,
                        })
                });
            Some(LayerBrowserEntry {
                path,
                is_dir,
                depth: 0,
                is_hidden: name.to_string_lossy().starts_with('.'),
                size: metadata.as_ref().map(|value| value.len()),
                modified: metadata.and_then(|value| value.modified().ok()),
                git,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| {
        (
            !entry.is_dir,
            entry.path.file_name().map(|name| name.to_owned()),
        )
    });
    Ok(entries)
}

pub(crate) async fn load_layer_browser_directory(
    app: &mut App,
    layer: String,
    root: PathBuf,
    directory: PathBuf,
) {
    let scan = directory.clone();
    let inspect_git = !layer.starts_with("Rootfs:") && layer != "Rootfs system";
    match tokio::task::spawn_blocking(move || scan_layer_directory(&scan, inspect_git)).await {
        Ok(Ok(entries)) => {
            if let Some(Effect::LoadLayerBrowserPreview(path)) = compatibility_workspace_action(
                app,
                Action::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                    entries,
                },
            ) {
                load_layer_browser_preview(app, path).await;
            }
        }
        Ok(Err(error)) => {
            app.notification = Some(format!("Could not read layer directory: {error}"))
        }
        Err(error) => app.notification = Some(format!("Layer directory scan failed: {error}")),
    }
}

pub(crate) fn read_layer_preview(path: &Path) -> io::Result<(String, PreviewKind, bool)> {
    const MAX_PREVIEW_BYTES: usize = 64 * 1024;
    let mut file = fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut bytes = Vec::new();
    std::io::Read::by_ref(&mut file)
        .take(MAX_PREVIEW_BYTES as u64)
        .read_to_end(&mut bytes)?;
    let truncated = size > bytes.len() as u64;
    if bytes.contains(&0) || std::str::from_utf8(&bytes).is_err() {
        Ok((String::new(), PreviewKind::Binary, truncated))
    } else {
        Ok((
            String::from_utf8(bytes).expect("UTF-8 was validated"),
            PreviewKind::Text,
            truncated,
        ))
    }
}

pub(crate) async fn load_layer_browser_preview(app: &mut App, path: PathBuf) {
    let preview_path = path.clone();
    match tokio::task::spawn_blocking(move || read_layer_preview(&preview_path)).await {
        Ok(Ok((content, kind, truncated))) => {
            let _ = update(
                app,
                Action::LoadLayerBrowserPreview {
                    path,
                    content,
                    kind,
                    truncated,
                },
            );
        }
        Ok(Err(error)) => app.notification = Some(format!("Could not preview layer file: {error}")),
        Err(error) => app.notification = Some(format!("Layer preview failed: {error}")),
    }
}
