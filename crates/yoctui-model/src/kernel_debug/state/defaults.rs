use super::*;

/// Captured selected environment. Discovery never searches a different build.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDebugDefaultContext {
    pub build_dir: PathBuf,
    pub machine: String,
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KernelDebugDefaults {
    pub values: Vec<(KernelDebugField, String)>,
    pub boot_mode: Option<crate::QemuDebugBootMode>,
    pub note: String,
}

impl KernelDebugState {
    pub fn fields_editable(&self) -> bool {
        self.pending.is_none()
            || matches!(
                self.pending,
                Some(KernelDebugOperation::DiscoverDefaults { .. })
            )
    }
}

pub(super) fn context(app: &App) -> Option<KernelDebugDefaultContext> {
    let environment = app
        .workspace_compatibility
        .authority()
        .map(|a| &a.snapshot.environment);
    let build_dir = environment
        .and_then(|e| e.build_directory.value())
        .or(app.workspace.build_dir.as_ref())?
        .clone();
    let machine = environment
        .and_then(|e| e.machine.value())
        .or_else(|| app.workspace.variables.get("MACHINE"))?
        .clone();
    let image = app.build.target.clone();
    Some(KernelDebugDefaultContext {
        build_dir,
        machine,
        image,
    })
}

pub(super) fn selected_build(app: &App) -> Option<&PathBuf> {
    app.workspace_compatibility
        .authority()
        .and_then(|a| a.snapshot.environment.build_directory.value())
        .or(app.workspace.build_dir.as_ref())
}

pub(super) fn finished(
    app: &mut App,
    expected: KernelDebugDefaultContext,
    tool: KernelDebugTool,
    result: Result<KernelDebugResult, String>,
) {
    if context(app).as_ref() != Some(&expected)
        || app.screen != Screen::Kernel
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
        || !matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.tool == tool)
    {
        return;
    }
    let defaults = match result {
        Ok(KernelDebugResult::Defaults(defaults)) => defaults,
        Err(error) => {
            app.kernel_debug.defaults_note = Some(format!("Defaults unavailable: {error}"));
            return;
        }
        _ => {
            app.kernel_debug.defaults_note =
                Some("Unexpected defaults result; enter paths manually".into());
            return;
        }
    };
    let edits = app.kernel_debug.default_edits.clone();
    if edits.contains(&KernelDebugField::BuildDirectory) {
        app.kernel_debug.defaults_note =
            Some("Build directory edited; enter matching artifacts manually".into());
        return;
    }
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
        return;
    };
    for (field, value) in defaults.values {
        if edits.contains(&field)
            || !dialog.draft.fields().contains(&field)
            || value.len() > MAX_KERNEL_DEBUG_FIELD_BYTES
            || value.chars().any(char::is_control)
        {
            continue;
        }
        if let Some(target) = dialog.draft.value_mut(field) {
            *target = value;
        }
    }
    if !edits.contains(&KernelDebugField::QemuBootMode)
        && tool == KernelDebugTool::QemuGdb
        && let Some(mode) = defaults.boot_mode
    {
        dialog.draft.qemu.boot_mode = mode;
    }
    app.kernel_debug.defaults_note = Some(defaults.note);
}

#[cfg(test)]
#[path = "../../tests/kernel_debug_defaults.rs"]
mod tests;
