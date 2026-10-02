use super::*;
use yoctui_model::{KernelDebugDialog, KernelDebugOperation};

pub(crate) fn dialog(frame: &mut Frame, app: &App, dialog: &KernelDebugDialog, area: Rect) {
    if area.is_empty() {
        return;
    }
    let preview = app.kernel_debug.instrumentation_preview.as_ref();
    let fields = dialog.draft.fields();
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(if preview.is_some() {
            0
        } else {
            fields.len() as u16
        }),
        Constraint::Min(1),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(area);
    frame.render_widget(Paragraph::new("CONFIG PREP · file evidence, NOT runtime readiness\nNo automatic config/layer edits, build, boot or self-tests.").wrap(Wrap { trim: false }), rows[0]);
    if preview.is_none() {
        let viewport = yoctui_model::centered_viewport_range(
            Some(dialog.selection),
            fields.len(),
            usize::from(rows[1].height).max(1),
        );
        let lines = viewport
            .map(|i| {
                Line::styled(
                    format!(
                        "{} {}: {}{}",
                        if i == dialog.selection { "▶" } else { " " },
                        fields[i].label(),
                        dialog.draft.value(fields[i]),
                        if i == dialog.selection { "_" } else { "" }
                    ),
                    selected_style(app, i == dialog.selection),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), rows[1]);
    }
    let preset = dialog.draft.instrumentation.preset;
    let mut lines = Vec::new();
    if let Some(preview) = preview {
        lines.push(Line::from(format!(
            "{} · {}",
            preset.label(),
            if preview.report.matches_requested() {
                "CONFIG MATCH (this file only)"
            } else {
                "NEEDS RESOLUTION"
            }
        )));
        lines.push(Line::from(format!("Inspected: {}", preview.draft.config)));
        lines.push(Line::from(format!("Export NEW: {}", preview.draft.output)));
        lines.push(Line::from(
            "Requested options · absent/unknown is NOT disabled:",
        ));
        for (name, expected) in preset.requested() {
            let observed = preview
                .report
                .options
                .get(*name)
                .and_then(|v| v.as_deref())
                .unwrap_or("absent/unknown");
            lines.push(Line::from(format!(
                "{name}: observed {observed}; requested {expected}"
            )));
        }
        lines.push(Line::from(
            "Capabilities observed separately; compiler/architecture support NOT proven:",
        ));
        for name in preset.capabilities() {
            let observed = preview
                .report
                .options
                .get(*name)
                .and_then(|v| v.as_deref())
                .unwrap_or("absent/unknown");
            lines.push(Line::from(format!("{name}: {observed}")));
        }
        lines.push(Line::from("Exact requested fragment (not applied):"));
        lines.extend(
            preset
                .fragment()
                .lines()
                .map(|line| Line::from(line.to_owned())),
        );
    }
    lines.push(Line::from(dialog.draft.tool.guide()));
    lines.push(Line::from("Next: manually integrate through supported Yocto kernel-provider fragments; validate resolved .config. Build/boot and controlled reproduction require separate approval; inspect matching diagnostic logs."));
    lines.push(Line::from(format!("Reference: {}", preset.reference())));
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((dialog.guide_scroll, 0)),
        rows[2],
    );
    let exporting = matches!(
        app.kernel_debug.pending,
        Some(KernelDebugOperation::ExportInstrumentation { .. })
    );
    let status = dialog.error.as_deref().unwrap_or(if exporting {
        "Exporting confirmed file; wait for result (cannot undo an in-flight write)."
    } else if app.kernel_debug.pending.is_some() {
        "Inspecting .config and destination (read-only)…"
    } else if preview.is_some() {
        "Export creates ONLY the reviewed new file; never overwrites or applies it."
    } else {
        "Supply an exact .config and NEW .cfg path; no file is written on inspection."
    });
    frame.render_widget(Paragraph::new(status).wrap(Wrap { trim: false }), rows[3]);
    let hints = if exporting {
        "Confirmed export is in progress; input locked until its result."
    } else if preview.is_some() {
        "↑/↓/PgUp/PgDn scroll exact report/fragment\nEnter export NEW .cfg · Esc back to edit (no write)"
    } else {
        "Tab/↑/↓ field · ←/→/Space preset · type · Ctrl+U clear · PgUp/Dn guide\nEnter inspect and review (no write) · Esc cancel"
    };
    frame.render_widget(Paragraph::new(hints).wrap(Wrap { trim: false }), rows[4]);
}
