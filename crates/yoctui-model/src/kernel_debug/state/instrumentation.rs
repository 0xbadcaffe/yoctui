use super::*;
use crate::KernelInstrumentationPreset as P;

fn covered(app: &App) -> bool {
    app.menu.is_open() || app.command_palette_open || app.onboarding.open
}

fn error(app: &mut App, message: String) {
    app.kernel_debug.error = Some(message.clone());
    if let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() {
        dialog.error = Some(message);
    }
}

pub(super) fn finished(
    app: &mut App,
    pending: KernelDebugOperation,
    result: Result<KernelDebugResult, String>,
) {
    match (pending, result) {
        (
            KernelDebugOperation::InspectInstrumentation { draft },
            Ok(KernelDebugResult::InstrumentationPrepared(preview)),
        ) => {
            if covered(app)
                || preview.draft != draft
                || preview.report.preset != draft.preset
                || !matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.tool.configuration_prep() && *d.draft.instrumentation == draft)
            {
                return;
            }
            app.kernel_debug.instrumentation_preview = Some(preview);
            if let Some(Dialog::KernelDebug(d)) = app.active_dialog_mut() {
                d.guide_scroll = 0;
                d.error = None;
            }
        }
        (
            KernelDebugOperation::ExportInstrumentation { expected },
            Ok(KernelDebugResult::InstrumentationExported(path)),
        ) if path == std::path::Path::new(&expected.draft.output) => {
            app.kernel_debug.instrumentation_preview = None;
            app.notification = Some(format!(
                "Exported {}. Not applied: integrate manually and verify resolved config before build/boot.",
                path.display()
            ));
            if matches!(app.active_dialog(), Some(Dialog::KernelDebug(d)) if d.draft.tool.configuration_prep() && *d.draft.instrumentation == expected.draft)
            {
                crate::close_dialog(app);
                crate::synchronize_focus(app);
            }
        }
        (_, Err(message)) => {
            app.kernel_debug.instrumentation_preview = None;
            error(app, message);
        }
        _ => {
            app.kernel_debug.instrumentation_preview = None;
            error(
                app,
                "Unexpected instrumentation result; inspect again before export".into(),
            );
        }
    }
}

pub(super) fn input(app: &mut App, action: KernelDebugAction) -> Option<Effect> {
    use KernelDebugAction as A;
    if app.kernel_debug.pending.is_some() || covered(app) {
        return None;
    }
    if matches!(action, A::Review) {
        let Some(Dialog::KernelDebug(dialog)) = app.active_dialog().cloned() else {
            return None;
        };
        if let Some(expected) = app.kernel_debug.instrumentation_preview.clone() {
            if expected.draft != *dialog.draft.instrumentation {
                app.kernel_debug.instrumentation_preview = None;
                error(app, "Inputs changed; inspect again before export".into());
                return None;
            }
            return begin(
                app,
                KernelDebugOperation::ExportInstrumentation {
                    expected: Box::new(expected),
                },
            );
        }
        if let Err(message) = dialog.draft.instrumentation.validate() {
            error(app, message);
            return None;
        }
        return begin(
            app,
            KernelDebugOperation::InspectInstrumentation {
                draft: *dialog.draft.instrumentation,
            },
        );
    }
    if !matches!(action, A::ScrollGuide(_)) {
        // No edit can leave a reviewed snapshot available for confirmation.
        app.kernel_debug.instrumentation_preview = None;
    }
    let Some(Dialog::KernelDebug(dialog)) = app.active_dialog_mut() else {
        return None;
    };
    let fields = dialog.draft.fields();
    let field = fields.get(dialog.selection).copied();
    match action {
        A::Field(delta) => dialog.selection = shifted(dialog.selection, delta, fields.len()),
        A::ChangeScope if field == Some(KernelDebugField::InstrumentationPreset) => {
            let current = P::SANITIZERS
                .iter()
                .position(|p| *p == dialog.draft.instrumentation.preset)
                .unwrap_or(0);
            dialog.draft.instrumentation.preset =
                P::SANITIZERS[(current + 1) % P::SANITIZERS.len()];
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
            if let Some(value) = field.and_then(|f| dialog.draft.value_mut(f)) {
                value.pop();
            }
        }
        A::Clear => {
            if let Some(value) = field.and_then(|f| dialog.draft.value_mut(f)) {
                value.clear();
            }
        }
        A::ScrollGuide(delta) => {
            dialog.guide_scroll = shifted(dialog.guide_scroll as usize, delta, 4097) as u16
        }
        _ => return None,
    }
    dialog.error = None;
    None
}
