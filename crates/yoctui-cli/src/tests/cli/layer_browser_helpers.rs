use super::*;

pub(super) async fn load_layer_browser_directory(
    app: &mut App,
    layer: String,
    root: PathBuf,
    directory: PathBuf,
) {
    let mut worker = crate::layer_browser_io::LayerBrowserIo::default();
    worker.directory(app, layer, root, directory);
    drain(&mut worker, app).await;
}

pub(super) async fn load_layer_browser_preview(app: &mut App, path: PathBuf) {
    let mut worker = crate::layer_browser_io::LayerBrowserIo::default();
    worker.preview(app, path);
    drain(&mut worker, app).await;
}

async fn drain(worker: &mut crate::layer_browser_io::LayerBrowserIo, app: &mut App) {
    // Exercise the production polling path, not a second synchronous loader.
    tokio::time::timeout(Duration::from_secs(5), async {
        while worker.pending() {
            worker.poll(app);
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .expect("browser acquisition did not finish");
}
