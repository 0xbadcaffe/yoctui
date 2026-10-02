use super::*;
use crate::{
    App, Dialog, Effect, FocusTarget, Screen, TerminalLaunchDestination, TerminalLaunchDialog,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDebugDialog {
    pub draft: KernelDebugDraft,
    pub selection: usize,
    pub guide_scroll: u16,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KernelDebugState {
    pub visible: bool,
    pub selection: usize,
    pub tools: Option<KernelDebugTools>,
    pub error: Option<String>,
    pub generation: u64,
    pub pending: Option<KernelDebugOperation>,
    pub prepared: Option<TerminalLaunchRequest>,
    pub qemu_preview: Option<crate::QemuDebugSpec>,
    pub serial_preview: Option<KgdbSerialPreview>,
    pub preview_scroll: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KgdbSerialPreview {
    pub spec: crate::KgdbSerialSpec,
    pub report: crate::KgdbConfigReport,
}

impl KernelDebugState {
    pub fn tool(&self) -> KernelDebugTool {
        KernelDebugTool::ALL[self.selection.min(KernelDebugTool::ALL.len() - 1)]
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelDebugOperation {
    Inspect,
    InspectInstrumentation {
        draft: crate::KernelInstrumentationDraft,
    },
    ExportInstrumentation {
        expected: Box<crate::KernelInstrumentationPreview>,
    },
    Prepare {
        draft: Box<KernelDebugDraft>,
        tools: KernelDebugTools,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KernelDebugRequest {
    pub generation: u64,
    pub operation: KernelDebugOperation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelDebugResult {
    Tools(KernelDebugTools),
    InstrumentationPrepared(crate::KernelInstrumentationPreview),
    InstrumentationExported(std::path::PathBuf),
    Prepared(TerminalLaunchRequest),
    PreparedSerial {
        request: TerminalLaunchRequest,
        report: crate::KgdbConfigReport,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KernelDebugAction {
    Open,
    Inspect,
    Select(isize),
    SelectAt(usize),
    OpenSelected,
    Field(isize),
    ChangeScope,
    Insert(String),
    Backspace,
    Clear,
    ScrollGuide(isize),
    ScrollPreview(isize),
    Review,
    Cancel,
    Finished {
        generation: u64,
        result: Result<KernelDebugResult, String>,
    },
}

pub(crate) fn reduce(app: &mut App, action: KernelDebugAction) -> Option<Effect> {
    use KernelDebugAction as A;
    if let A::Finished { generation, result } = action {
        if generation != app.kernel_debug.generation {
            return None;
        }
        let pending = app.kernel_debug.pending.take()?;
        match (pending, result) {
            (KernelDebugOperation::Inspect, Ok(KernelDebugResult::Tools(tools))) => {
                app.kernel_debug.tools = Some(tools);
                app.kernel_debug.error = None;
            }
            (
                KernelDebugOperation::Prepare { draft, tools },
                Ok(KernelDebugResult::Prepared(request)),
            ) => {
                if draft.tool == KernelDebugTool::KgdbSerial {
                    app.kernel_debug.error =
                        Some("Serial preparation requires typed config observations; retry".into());
                    return None;
                }
                if !matches!(app.active_dialog(), Some(Dialog::KernelDebug(dialog)) if dialog.draft == *draft)
                    || draft.plan(&tools).ok().as_ref() != Some(&request)
                {
                    return None;
                }
                app.kernel_debug.prepared = Some(request.clone());
                app.kernel_debug.serial_preview = None;
                app.kernel_debug.qemu_preview = (draft.tool == KernelDebugTool::QemuGdb)
                    .then(|| draft.qemu_spec(&tools).ok())
                    .flatten();
                app.kernel_debug.preview_scroll = 0;
                crate::replace_dialog(
                    app,
                    Dialog::TerminalLaunch(TerminalLaunchDialog {
                        request,
                        destination: TerminalLaunchDestination::Embedded,
                        output_must_not_exist: None,
                    }),
                );
                crate::synchronize_focus(app);
            }
            (
                KernelDebugOperation::Prepare { draft, tools },
                Ok(KernelDebugResult::PreparedSerial { request, report }),
            ) => {
                if draft.tool != KernelDebugTool::KgdbSerial
                    || !matches!(app.active_dialog(), Some(Dialog::KernelDebug(dialog)) if dialog.draft == *draft)
                    || app.menu.is_open()
                    || app.command_palette_open
                    || app.onboarding.open
                    || draft.plan(&tools).ok().as_ref() != Some(&request)
                    || crate::KGDB_CONFIG_OPTIONS[..3].iter().any(|name| {
                        report.options.get(*name).and_then(|v| v.as_deref()) != Some("y")
                    })
                {
                    return None;
                }
                let Ok(spec) = draft.serial_spec(&tools) else {
                    return None;
                };
                app.kernel_debug.prepared = Some(request.clone());
                app.kernel_debug.qemu_preview = None;
                app.kernel_debug.serial_preview = Some(KgdbSerialPreview { spec, report });
                app.kernel_debug.preview_scroll = 0;
                crate::replace_dialog(
                    app,
                    Dialog::TerminalLaunch(TerminalLaunchDialog {
                        request,
                        destination: TerminalLaunchDestination::Embedded,
                        output_must_not_exist: None,
                    }),
                );
                crate::synchronize_focus(app);
            }
            (_, Err(message)) => {
                app.kernel_debug.error = Some(message.clone());
                if let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() {
                    dialog.error = Some(message);
                }
            }
            _ => {
                app.kernel_debug.error =
                    Some("Unexpected debugging worker result; refresh tools".into())
            }
        }
        return None;
    }
    if let A::ScrollPreview(delta) = action {
        if matches!(app.active_dialog(), Some(Dialog::TerminalLaunch(dialog)) if Some(&dialog.request) == app.kernel_debug.prepared.as_ref())
        {
            app.kernel_debug.preview_scroll =
                shifted(app.kernel_debug.preview_scroll as usize, delta, 4097) as u16;
        }
        return None;
    }
    if let A::Cancel = action {
        if matches!(app.active_dialog(), Some(Dialog::KernelDebug(_))) {
            if matches!(
                app.kernel_debug.pending,
                Some(KernelDebugOperation::Prepare { .. })
            ) {
                app.kernel_debug.pending = None;
                app.kernel_debug.generation = app.kernel_debug.generation.saturating_add(1);
            }
            crate::close_dialog(app);
            crate::synchronize_focus(app);
        }
        return None;
    }
    if app.screen != Screen::Kernel {
        return None;
    }
    if let Some(Dialog::KernelDebug(_)) = app.active_dialog() {
        if app.kernel_debug.pending.is_some() {
            return None;
        }
        if matches!(action, A::Review) {
            let Some(Dialog::KernelDebug(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            dialog.draft.tool.program()?;
            let Some(tools) = app.kernel_debug.tools.clone() else {
                if let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() {
                    dialog.error =
                        Some("Tool discovery unavailable; cancel and press r to refresh".into());
                }
                return None;
            };
            if let Err(message) = dialog.draft.plan(&tools) {
                if let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() {
                    dialog.error = Some(message);
                }
                return None;
            }
            return begin(
                app,
                KernelDebugOperation::Prepare {
                    draft: Box::new(dialog.draft),
                    tools,
                },
            );
        }
        let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
            return None;
        };
        let fields = dialog.draft.fields();
        let field = fields.get(dialog.selection).copied();
        match action {
            A::Field(delta) => {
                dialog.selection = shifted(dialog.selection, delta, fields.len());
            }
            A::ChangeScope if field == Some(KernelDebugField::Destination) => {
                dialog.draft.ssh = !dialog.draft.ssh;
                dialog.selection = 0;
            }
            A::Insert(text) => {
                if let Some(value) = field.and_then(|field| dialog.draft.value_mut(field)) {
                    if text.chars().any(char::is_control)
                        || text.len() + value.len() > MAX_KERNEL_DEBUG_FIELD_BYTES
                    {
                        dialog.error =
                            Some("Input exceeds 4096 bytes or contains control characters".into());
                        return None;
                    }
                    value.push_str(&text);
                }
            }
            A::Backspace => {
                if let Some(value) = field.and_then(|field| dialog.draft.value_mut(field)) {
                    value.pop();
                }
            }
            A::Clear => {
                if let Some(value) = field.and_then(|field| dialog.draft.value_mut(field)) {
                    value.clear();
                }
            }
            A::ScrollGuide(delta) => {
                dialog.guide_scroll = shifted(dialog.guide_scroll as usize, delta, 4097) as u16;
            }
            _ => return None,
        }
        dialog.error = None;
        return None;
    }
    if app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || app.onboarding.open
    {
        return None;
    }
    match action {
        A::Open => {
            app.kernel_debug.visible = true;
            app.focus = FocusTarget::Workspace;
            if app.kernel_debug.tools.is_none() && app.kernel_debug.pending.is_none() {
                return begin(app, KernelDebugOperation::Inspect);
            }
        }
        A::Inspect if app.kernel_debug.pending.is_none() => {
            return begin(app, KernelDebugOperation::Inspect);
        }
        A::Select(delta) => {
            app.kernel_debug.selection = shifted(
                app.kernel_debug.selection,
                delta,
                KernelDebugTool::ALL.len(),
            )
        }
        A::SelectAt(index) => {
            app.kernel_debug.selection = index.min(KernelDebugTool::ALL.len() - 1)
        }
        A::OpenSelected => {
            app.kernel_debug.prepared = None;
            app.kernel_debug.qemu_preview = None;
            app.kernel_debug.serial_preview = None;
            let mut draft = KernelDebugDraft::new(app.kernel_debug.tool());
            if draft.tool == KernelDebugTool::QemuGdb {
                draft.qemu.build_dir = app
                    .workspace
                    .build_dir
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
                let initialized = app
                    .workspace_compatibility
                    .authority()
                    .and_then(|authority| authority.snapshot.environment.available_tools.value())
                    .and_then(|tools| tools.iter().find(|tool| tool.id == "runqemu"))
                    .map(|tool| &tool.executable);
                let detected = app
                    .kernel_debug
                    .tools
                    .as_ref()
                    .and_then(|tools| tools.programs.get("runqemu"));
                draft.qemu.runqemu = initialized
                    .or(detected)
                    .map(|path| path.display().to_string())
                    .unwrap_or_default();
            }
            crate::open_dialog(
                app,
                Dialog::KernelDebug(KernelDebugDialog {
                    draft,
                    selection: 0,
                    guide_scroll: 0,
                    error: None,
                }),
            );
            crate::synchronize_focus(app);
        }
        _ => {}
    }
    None
}

fn begin(app: &mut App, operation: KernelDebugOperation) -> Option<Effect> {
    let generation = app.kernel_debug.generation.checked_add(1)?;
    app.kernel_debug.generation = generation;
    app.kernel_debug.pending = Some(operation.clone());
    app.kernel_debug.error = None;
    Some(Effect::KernelDebug(KernelDebugRequest {
        generation,
        operation,
    }))
}

fn shifted(index: usize, delta: isize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        index.saturating_add_signed(delta).min(count - 1)
    }
}
