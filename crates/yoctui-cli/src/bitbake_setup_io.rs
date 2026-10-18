//! Bounded local discovery and launch validation; the official tool owns setup.
use std::{fs, path::Path};
use yoctui_model::{
    Action, App, BitBakeSetupAction as A, BitBakeSetupEntry, BitBakeSetupPlan,
    BitBakeSetupRequest as Request, Effect, TerminalLaunchRequest,
};

#[derive(Default)]
pub(crate) struct BitBakeSetupIo {
    worker: Option<tokio::task::JoinHandle<A>>,
    queued: Option<Request>,
    active: Option<Request>,
}

pub(crate) fn defaults(app: &mut App) {
    if app.bitbake_setup.top_dir.is_empty() {
        app.bitbake_setup.top_dir = yoctui_utils::home_dir()
            .join("bitbake-builds")
            .display()
            .to_string();
    }
    if app.bitbake_setup.tool.is_empty() {
        let sibling = app
            .workspace
            .source_dir
            .as_deref()
            .and_then(Path::parent)
            .map(|p| p.join("bitbake/bin/bitbake-setup"));
        app.bitbake_setup.tool = sibling
            .filter(|p| p.is_file())
            .map_or_else(|| "bitbake-setup".into(), |p| p.display().to_string());
    }
}

fn scan(root: &Path) -> Result<Vec<BitBakeSetupEntry>, String> {
    // Validate even requests constructed outside the UI. No recursive walk or
    // sourcing scripts, and no discovery commands that might fetch a registry.
    let values = [
        "bitbake-setup".into(),
        root.display().to_string(),
        "scan".into(),
        "".into(),
        "".into(),
    ];
    BitBakeSetupPlan::from_values(&values)?;
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Cannot list {}: {e}", root.display())),
    };
    let mut result = Vec::new();
    for path in std::iter::once(root.to_path_buf())
        .chain(entries.take(1024).filter_map(|e| e.ok().map(|e| e.path())))
    {
        if result.len() == 256 {
            break;
        }
        if let Some(profile) = yoctui_bitbake::discover_build_environment(&path, None) {
            if result
                .iter()
                .any(|entry: &BitBakeSetupEntry| entry.profile.build_dir == profile.build_dir)
            {
                continue;
            }
            result.push(BitBakeSetupEntry {
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                profile,
            });
        }
    }
    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

fn executable(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    metadata.is_file()
}

fn prepare(plan: &BitBakeSetupPlan) -> Result<TerminalLaunchRequest, String> {
    let mut launch = plan.launch()?;
    match fs::symlink_metadata(plan.destination()) {
        Ok(_) => return Err("Destination already exists. Choose a new name; existing or partial setups are never overwritten.".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {},
        Err(e) => return Err(format!("Cannot inspect destination: {e}")),
    }
    if !launch.cwd.is_dir() {
        return Err("The top directory's parent must already exist.".into());
    }
    match fs::symlink_metadata(&plan.top_dir) {
        Ok(m) if m.file_type().is_symlink() || !m.is_dir() => {
            return Err("Top directory must be a directory, not a symlink/file.".into());
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("Cannot inspect top directory: {e}")),
    }
    if !launch.program.is_absolute() {
        launch.program = std::env::var_os("PATH").and_then(|value|std::env::split_paths(&value)
            .take(64).filter(|p|p.is_absolute()).map(|p|p.join("bitbake-setup"))
            .find(|p|executable(p)))
            .ok_or("bitbake-setup is not on PATH. Install it in a virtual environment or choose its absolute executable path.")?;
    }
    if !executable(&launch.program) {
        return Err(format!(
            "{} is not an executable file.",
            launch.program.display()
        ));
    }
    Ok(launch)
}

impl BitBakeSetupIo {
    pub fn submit(&mut self, effect: Effect) {
        if let Effect::BitBakeSetup(request) = effect {
            self.queued = Some(request);
            self.start();
        }
    }
    fn start(&mut self) {
        if self.worker.is_some() {
            return;
        }
        let Some(request) = self.queued.take() else {
            return;
        };
        self.active = Some(request.clone());
        self.worker = Some(tokio::task::spawn_blocking(move || match request {
            Request::List { generation, root } => A::Listed {
                generation,
                result: scan(&root),
            },
            Request::Prepare { generation, plan } => A::Prepared {
                generation,
                result: prepare(&plan),
            },
        }));
    }
    pub async fn poll(&mut self, app: &mut App) -> (bool, Option<Effect>) {
        if !self.worker.as_ref().is_some_and(|w| w.is_finished()) {
            return (false, None);
        }
        let result = self.worker.take().expect("finished worker").await;
        let request = self.active.take().expect("active request");
        let action = result.unwrap_or_else(|error| match request {
            Request::List { generation, .. } => A::Listed {
                generation,
                result: Err(format!("Scan failed: {error}")),
            },
            Request::Prepare { generation, .. } => A::Prepared {
                generation,
                result: Err(format!("Validation failed: {error}")),
            },
        });
        let effect =
            yoctui_model::update_with_workspace_authority(app, Action::BitBakeSetup(action));
        self.start();
        (true, effect)
    }
}

#[cfg(test)]
#[path = "tests/bitbake_setup_io.rs"]
mod tests;
