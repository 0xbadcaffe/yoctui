//! No private BitBake imports or direct toolcfg edits: execute public CLI tools.
use std::{path::Path, process::Stdio, time::Duration};
use tokio::io::AsyncReadExt;
use yoctui_model::{
    SetupManageAction as A, SetupManageRequest as R, SetupOperation, SetupOperationDialog,
    TerminalLaunchRequest,
};
mod fragments;

pub(super) fn failed(request: R, error: String) -> A {
    match request {
        R::Inspect { generation, .. } => A::Loaded {
            generation,
            result: Err(error),
        },
        R::Prepare { generation, .. } => A::Prepared {
            generation,
            result: Err(error),
        },
        R::Read { generation, .. } => A::Read {
            generation,
            result: Err(error),
        },
    }
}

pub(super) async fn perform(request: R) -> A {
    match request {
        R::Prepare { generation, dialog } => A::Prepared {
            generation,
            result: prepare(&dialog),
        },
        R::Inspect {
            generation,
            profile,
        } => {
            let dialog = SetupOperationDialog {
                operation: SetupOperation::Fragments,
                profile,
                tool: String::new(),
                fragment: String::new(),
                editor: None,
                config_policy: 0,
                backup_conflicts: false,
                offline: true,
                preparing: None,
                error: None,
            };
            let result = match prepare(&dialog) {
                Ok(request) => capture(request, Duration::from_secs(60))
                    .await
                    .and_then(|text| fragments::parse(&text)),
                Err(e) => Err(e),
            };
            A::Loaded { generation, result }
        }
        R::Read { generation, dialog } => {
            let result = match prepare(&dialog) {
                Ok(request) => capture(request, Duration::from_secs(60)).await,
                Err(e) => Err(e),
            };
            A::Read { generation, result }
        }
    }
}

fn regular(path: &Path) -> Result<(), String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() || metadata.len() > 1024 * 1024 {
        return Err(format!(
            "{} must be a regular file up to 1 MiB, not a symlink/device",
            path.display()
        ));
    }
    Ok(())
}

fn prepare(dialog: &SetupOperationDialog) -> Result<TerminalLaunchRequest, String> {
    let mut launch = dialog.launch()?;
    let profile = &dialog.profile;
    let discovered =
        yoctui_bitbake::discover_build_environment(&profile.build_dir, Some(&profile.source_dir))
            .ok_or("Build configuration/initializer disappeared. Refresh the setup selection.")?;
    if discovered != *profile {
        return Err("Reviewed paths changed or contain symlinks. Re-import the canonical setup before continuing.".into());
    }
    regular(&profile.init_script)?;
    regular(&profile.build_dir.join("conf/local.conf"))?;
    regular(&profile.build_dir.join("conf/bblayers.conf"))?;
    if matches!(
        dialog.operation,
        SetupOperation::Enable | SetupOperation::Disable
    ) {
        let conf = profile.build_dir.join("conf/toolcfg.conf");
        if conf.symlink_metadata().is_ok() {
            regular(&conf)?;
        }
    }
    if matches!(
        dialog.operation,
        SetupOperation::Status | SetupOperation::Update
    ) {
        regular(
            &profile
                .build_dir
                .parent()
                .ok_or("Missing setup root")?
                .join("config/config-upstream.json"),
        )?;
        if !launch.program.is_absolute() {
            launch.program=std::env::var_os("PATH").and_then(|v| std::env::split_paths(&v).take(64)
                .filter(|p|p.is_absolute()).map(|p|p.join("bitbake-setup")).find(|p|super::executable(p)))
                .ok_or("bitbake-setup unavailable on PATH; set its absolute path in the new-setup wizard.")?;
        }
    }
    if !super::executable(&launch.program) {
        return Err(format!("{} is not executable", launch.program.display()));
    }
    Ok(launch)
}

async fn bounded<R: tokio::io::AsyncRead + Unpin>(
    stream: R,
    limit: usize,
) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    stream
        .take((limit + 1) as u64)
        .read_to_end(&mut out)
        .await
        .map_err(|e| e.to_string())?;
    if out.len() > limit {
        return Err(format!(
            "Tool output exceeded {limit} bytes; use its embedded terminal instead."
        ));
    }
    Ok(out)
}

struct ProcessGroup(Option<u32>);
impl Drop for ProcessGroup {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.0.take() {
            unsafe {
                libc::kill(-(pid as i32), libc::SIGKILL);
            }
        }
    }
}

async fn capture(request: TerminalLaunchRequest, timeout: Duration) -> Result<String, String> {
    let mut command = tokio::process::Command::new(&request.program);
    command
        .args(&request.arguments)
        .current_dir(&request.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for name in [
        "BBSERVER",
        "BBPATH",
        "BUILDDIR",
        "OEROOT",
        "BITBAKEDIR",
        "TEMPLATECONF",
    ] {
        command.env_remove(name);
    }
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|e| e.to_string())?;
    let mut group = ProcessGroup(child.id());
    let stdout = child.stdout.take().ok_or("Missing stdout")?;
    let stderr = child.stderr.take().ok_or("Missing stderr")?;
    let result = tokio::time::timeout(timeout, async {
        tokio::try_join!(
            bounded(stdout, 1024 * 1024),
            bounded(stderr, 64 * 1024),
            async { child.wait().await.map_err(|e| e.to_string()) }
        )
    })
    .await
    .map_err(|_| {
        "Setup inspection timed out after 60 seconds. No image build was started.".to_owned()
    })?;
    let (out, err, status) = result?;
    group.0 = None;
    let clean = |v: &[u8]| {
        yoctui_utils::strip_ansi(&String::from_utf8_lossy(v))
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            .collect::<String>()
    };
    if !status.success() {
        return Err(format!(
            "Tool failed ({status}): {}\n{}",
            clean(&err),
            clean(&out)
        ));
    }
    Ok(clean(&out))
}

#[cfg(test)]
#[path = "../tests/setup_management_io.rs"]
mod tests;
