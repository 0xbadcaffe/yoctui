use super::*;
use crate::layer_browser::{decorate_entries, read_layer_preview, scan_layer_directory_stream};

fn spawn(
    app: &App,
    layer: String,
    root: PathBuf,
    path: PathBuf,
    run: impl FnOnce(
        mpsc::Sender<Result<ResultData, String>>,
        Arc<AtomicBool>,
    ) -> tokio::task::JoinHandle<()>,
) -> Operation {
    let (sender, results) = mpsc::channel(2);
    let cancelled = Arc::new(AtomicBool::new(false));
    let task = run(sender, Arc::clone(&cancelled));
    Operation {
        context: Context::capture(app),
        layer,
        root,
        path,
        cancelled,
        task,
        results,
        published: false,
    }
}

pub(super) fn directory(
    app: &App,
    layer: String,
    root: PathBuf,
    path: PathBuf,
    pool: Arc<Semaphore>,
    cache: Arc<std::sync::Mutex<git::GitCache>>,
) -> Operation {
    let rootfs = layer.starts_with("Rootfs:") || layer == "Rootfs system";
    let with_git = !rootfs && layer != "Build directory";
    let scan_root = root.clone();
    let scan_path = path.clone();
    spawn(app, layer, root, path, move |sender, cancelled| {
        tokio::spawn(async move {
            let Ok(permit) = pool.acquire_owned().await else {
                return;
            };
            let chunks = sender.clone();
            let scan = scan_path.clone();
            let observed = Arc::clone(&cache);
            let result = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                if with_git {
                    observed
                        .lock()
                        .unwrap_or_else(|error| error.into_inner())
                        .observe(&scan);
                }
                let mut next_chunk = 128;
                let mut emit = |entries: &[LayerBrowserEntry]| {
                    if cancelled.load(Ordering::Relaxed) {
                        return false;
                    }
                    if entries.len() < next_chunk {
                        return true;
                    }
                    next_chunk *= 2;
                    chunks
                        .blocking_send(Ok(ResultData::Directory {
                            entries: entries.to_vec(),
                            complete: false,
                        }))
                        .is_ok()
                };
                if rootfs {
                    yoctui_bitbake::scan_rootfs_browser_directory_stream(
                        &scan_root, &scan, &mut emit,
                    )
                } else {
                    scan_layer_directory_stream(&scan, &mut emit)
                }
            })
            .await;
            let mut entries = match result {
                Ok(Ok(entries)) => entries,
                error => {
                    let message = match error {
                        Ok(Err(error)) => error.to_string(),
                        Err(error) => error.to_string(),
                        _ => unreachable!(),
                    };
                    let _ = sender.send(Err(message)).await;
                    return;
                }
            };
            if with_git {
                // Make the complete file list usable before waiting for Git.
                if sender
                    .send(Ok(ResultData::Directory {
                        entries: entries.clone(),
                        complete: false,
                    }))
                    .await
                    .is_err()
                {
                    return;
                }
                let output = git::status(&scan_path, cache).await;
                decorate_entries(&scan_path, &mut entries, output.as_deref());
            }
            let _ = sender
                .send(Ok(ResultData::Directory {
                    entries,
                    complete: true,
                }))
                .await;
        })
    })
}

pub(super) fn preview(
    app: &App,
    layer: String,
    root: PathBuf,
    path: PathBuf,
    pool: Arc<Semaphore>,
) -> Operation {
    let rootfs = layer.starts_with("Rootfs:") || layer == "Rootfs system";
    let read_root = root.clone();
    let read_path = path.clone();
    spawn(app, layer, root, path, move |sender, cancelled| {
        tokio::spawn(async move {
            let Ok(permit) = pool.acquire_owned().await else {
                return;
            };
            let result = tokio::task::spawn_blocking(move || {
                let _permit = permit;
                if cancelled.load(Ordering::Relaxed) {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Interrupted,
                        "preview cancelled",
                    ));
                }
                if rootfs {
                    yoctui_bitbake::read_rootfs_browser_preview(&read_root, &read_path)
                } else {
                    read_layer_preview(&read_path)
                }
            })
            .await;
            let result = match result {
                Ok(Ok((content, kind, truncated))) => Ok(ResultData::Preview {
                    content,
                    kind,
                    truncated,
                }),
                Ok(Err(error)) => Err(error.to_string()),
                Err(error) => Err(error.to_string()),
            };
            let _ = sender.send(result).await;
        })
    })
}
