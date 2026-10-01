//! Local tool presence discovery and non-mutating launch preparation.
use std::{collections::BTreeMap, path::Path};
use yoctui_model::{KernelDebugDraft, KernelDebugTool, KernelDebugTools, TerminalLaunchRequest};
mod worker;
pub(crate) use worker::KernelDebugIo;

pub(crate) fn discover() -> Result<KernelDebugTools, String> {
    let cwd = std::env::current_dir()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("Could not resolve host working directory: {error}"))?;
    let mut programs = BTreeMap::new();
    for name in KernelDebugTool::ALL
        .iter()
        .filter_map(|tool| tool.program())
        .chain(std::iter::once("ssh"))
    {
        if programs.contains_key(name) {
            continue;
        }
        let candidate = if name == "gdb" {
            crate::terminal_launcher::executable_on_initialized_path("gdb-multiarch")
                .or_else(|| crate::terminal_launcher::executable_on_initialized_path("gdb"))
        } else {
            crate::terminal_launcher::executable_on_initialized_path(name)
        };
        if let Some(program) = candidate.and_then(|path| path.canonicalize().ok())
            && program.is_absolute()
        {
            programs.insert(name.to_owned(), program);
        }
    }
    let helper = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| error.to_string())?;
    programs.insert("yoctui".into(), helper);
    Ok(KernelDebugTools { cwd, programs })
}

pub(crate) fn prepare(
    draft: &KernelDebugDraft,
    tools: &KernelDebugTools,
) -> Result<TerminalLaunchRequest, String> {
    let request = draft.plan(tools)?;
    if draft.tool == KernelDebugTool::QemuGdb {
        crate::qemu_debug::validate_files(&draft.qemu_spec(tools)?)
            .map_err(|error| error.to_string())?;
    }
    if !request.cwd.is_dir() {
        return Err("Host working directory disappeared; refresh tools".into());
    }
    let metadata = std::fs::metadata(&request.program)
        .map_err(|error| format!("Executable unavailable: {error}"))?;
    if !crate::terminal_launcher::executable(&metadata) {
        return Err("Detected program is no longer executable; refresh tools".into());
    }
    use KernelDebugTool as T;
    if matches!(draft.tool, T::GdbRemote | T::GdbCore | T::Crash) {
        regular_file(Path::new(&draft.symbols))?;
    }
    if matches!(draft.tool, T::GdbCore | T::Crash | T::TraceCmd) {
        regular_file(Path::new(&draft.data))?;
    }
    Ok(request)
}

fn regular_file(path: &Path) -> Result<(), String> {
    let metadata =
        std::fs::symlink_metadata(path).map_err(|error| format!("{}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "{} must be an existing regular file, not a symlink/device/directory",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/kernel_debug_tools.rs"]
mod tests;
