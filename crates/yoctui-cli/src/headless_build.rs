//! Headless build.
use super::*;

pub(crate) async fn headless(
    backend_kind: Backend,
    build_dir: PathBuf,
    targets: Vec<String>,
    log_entries: usize,
    log_bytes: usize,
) -> Result<()> {
    let mut backend = select_backend(backend_kind, build_dir.clone()).await?;
    let result = async {
        let mut app = App::new(log_entries, log_bytes);
        let mut build_jobs = BuildJobCoordinator::default();
        let workspace = backend.inspect_workspace().await?;
        let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
        if targets.is_empty() {
            println!("headless inspection completed");
            return Ok(());
        }
        let request = BuildRequest {
            targets,
            task: None,
            force: false,
        };
        if let Some(actions) = build_jobs.queue_build(&request, SystemTime::now()) {
            for action in actions {
                let _ = update(&mut app, action);
            }
        }
        if let Err(error) = backend.start_build(request).await {
            for action in build_jobs.start_failed(error.to_string(), SystemTime::now()) {
                let _ = update(&mut app, action);
            }
            return Err(error).context("could not start bitbake");
        }
        loop {
            let event = backend.next_event().await?;
            if let BackendEvent::Log(entry) = &event {
                println!("{}", entry.message);
            }
            let completion = match &event {
                BackendEvent::BuildCompleted { success, exit_code } => Some((*success, *exit_code)),
                _ => None,
            };
            for action in build_jobs.actions_for_backend_event(event, SystemTime::now()) {
                let _ = update(&mut app, action);
            }
            if let Some((success, exit_code)) = completion {
                println!(
                    "build {}{}",
                    if success { "completed" } else { "failed" },
                    exit_code.map_or_else(String::new, |code| format!(" (exit code {code})"))
                );
                if success {
                    return Ok(());
                }
                return Err(anyhow::anyhow!("BitBake build failed"));
            }
        }
    }
    .await;
    let shutdown = backend.shutdown().await;
    result?;
    shutdown?;
    Ok(())
}
