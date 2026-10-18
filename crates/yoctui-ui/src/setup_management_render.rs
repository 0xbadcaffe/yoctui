use super::*;
use yoctui_model::{SetupOperation, SetupOperationDialog};

pub(crate) fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.bitbake_setup.management;
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Build environment · 3 Fragments / setup details")
        .style(ThemePalette::for_app(app).base());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(inner);
    let build = state
        .profile
        .as_ref()
        .map(|p| p.build_dir.display().to_string())
        .unwrap_or_default();
    frame.render_widget(
        Paragraph::new(format!(
            "1 Environment | 2 Setups | 3 Fragments\nBuild: {build}\n{}",
            if state.loading {
                "Running reviewed inspection… Esc cancels"
            } else if state.output.is_some() {
                &state.output_title
            } else {
                "[x] enabled · [ ] unused · [value] built-in pattern (edit full ID)"
            }
        )),
        rows[0],
    );
    let capacity = usize::from(rows[1].height).max(1);
    let lines = if let Some(lines) = &state.output {
        lines
            .iter()
            .skip(state.output_scroll)
            .take(capacity)
            .cloned()
            .map(Line::from)
            .collect()
    } else if state.fragments.is_empty() {
        vec![Line::from(
            "No fragment inventory yet. r queries the selected build; unsupported tools show an error.",
        )]
    } else {
        let start = state.selected.saturating_sub(capacity.saturating_sub(1));
        state
            .fragments
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
            .map(|(i, f)| {
                Line::styled(
                    format!(
                        "{} {} {} · {}",
                        if i == state.selected { ">" } else { " " },
                        if f.pattern {
                            "[value]"
                        } else if f.enabled {
                            "[x]"
                        } else {
                            "[ ]"
                        },
                        f.name,
                        f.summary
                    ),
                    if i == state.selected {
                        selected_style(app, true)
                    } else {
                        ThemePalette::for_app(app).base()
                    },
                )
            })
            .collect()
    };
    // Bounded visible lines only; rendering never executes a tool.
    frame.render_widget(Paragraph::new(lines), rows[1]);
    frame.render_widget(Paragraph::new(format!("{}\n{}",
        state.error.as_deref().unwrap_or("Changes use bitbake-config-build and require review; no image build starts."),
        if state.output.is_some() {"↑/↓ PgUp/PgDn Home/End scroll · Esc back · 1/2 environment/setups"}
        else {"Enter show · e enable · d disable · n enter ID · r refresh · s status · u update · Esc back"}
    )).wrap(Wrap {trim:false}),rows[2]);
}

pub(crate) fn review(frame: &mut Frame, app: &App, dialog: &SetupOperationDialog, area: Rect) {
    let width = area.width.saturating_sub(4).min(112);
    let height = area.height.saturating_sub(2).min(27);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    frame.render_widget(
        dialog_block(app, dialog.operation.label(), DialogTone::Standard),
        popup,
    );
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(popup.inner(Margin::new(1, 1)));
    let mut lines = vec![Line::from(format!(
        "Build: {}",
        dialog.profile.build_dir.display()
    ))];
    if let Some(editor) = &dialog.editor {
        lines.push(Line::from(
            "Edit fragment ID (e.g. machine/qemux86-64 or core/yocto/monitor-disk-space)",
        ));
        let width = usize::from(rows[0].width.saturating_sub(2)) / 2;
        let before: String = editor.text[..editor.cursor]
            .chars()
            .rev()
            .take(width)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let after: String = editor.text[editor.cursor..].chars().take(width).collect();
        lines.push(Line::from(vec![
            Span::raw(before),
            Span::styled("|", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw(after),
        ]));
    } else {
        match dialog.operation {
            SetupOperation::Fragments
            | SetupOperation::Show
            | SetupOperation::Enable
            | SetupOperation::Disable => {
                lines.push(Line::from(format!(
                    "Source trusted initializer: {}",
                    dialog.profile.init_script.display()
                )));
                lines.push(Line::from(
                    "Tool: bitbake-config-build (from the selected initialized build)",
                ));
                if dialog.operation != SetupOperation::Fragments {
                    lines.push(Line::from(format!("Fragment: {}", dialog.fragment)));
                }
                if matches!(
                    dialog.operation,
                    SetupOperation::Enable | SetupOperation::Disable
                ) {
                    lines.push(Line::from(format!(
                        "Writes: {}",
                        dialog.profile.build_dir.join("conf/toolcfg.conf").display()
                    )));
                    lines.push(Line::from(
                        "Enabling machine/distro replaces the same built-in prefix.",
                    ));
                    lines.push(Line::from("Other config files may override OE_FRAGMENTS; refresh verifies effective state."));
                    lines.push(Line::from(
                        "Runs in a daemon-owned terminal; o takes writer control.",
                    ));
                } else {
                    lines.push(Line::from(
                        "Read-only metadata inspection; may start/connect a BitBake server.",
                    ));
                }
            }
            SetupOperation::Status | SetupOperation::Update => {
                lines.push(Line::from(format!("Tool: {}", dialog.tool)));
                if let Some(root) = dialog.profile.build_dir.parent() {
                    lines.push(Line::from(format!("Setup: {}", root.display())));
                }
                lines.push(Line::from(format!(
                    "Network: {}",
                    if dialog.offline {
                        "disabled (--no-network)"
                    } else {
                        "allowed (configuration/source checks may fetch)"
                    }
                )));
                if dialog.operation == SetupOperation::Update {
                    lines.push(Line::from(format!(
                        "Configuration policy: {} · c cycles prompt/keep/apply",
                        dialog.config_policy()
                    )));
                    lines.push(Line::from(format!(
                        "Rebase conflicts: {} · b toggles",
                        if dialog.backup_conflicts {
                            "BACKUP and re-clone"
                        } else {
                            "abort (default)"
                        }
                    )));
                    lines.push(Line::from(
                        "Updates source revisions and possibly build/conf; not a dry run.",
                    ));
                    lines.push(Line::from(
                        "Shared/local source trees may affect OTHER builds.",
                    ));
                    lines.push(Line::from(
                        "Commit/save edits first; abort does not undo earlier repositories.",
                    ));
                    lines.push(Line::from(
                        "n toggles network; prompts run in the embedded terminal (o to type).",
                    ));
                } else {
                    lines.push(Line::from("Inspect synchronization status; may refresh tool caches. n toggles network."));
                }
            }
        }
        lines.push(Line::from("Stop external BitBake processes too. Yoctui never retargets a daemon or starts an image build here."));
    }
    frame.render_widget(Paragraph::new(lines), rows[0]);
    frame.render_widget(
        Paragraph::new(dialog.error.as_deref().unwrap_or_default())
            .style(severity_style(app, Severity::Warning))
            .wrap(Wrap { trim: false }),
        rows[1],
    );
    let hint = if dialog.editor.is_some() {
        "Type/paste · Ctrl-U clear · Enter accept ID · Esc discard edit"
    } else if dialog.preparing.is_some() {
        "Checking reviewed paths… Esc cancels before launch"
    } else {
        "Enter confirm · e edit fragment ID · Esc cancel (no command runs)"
    };
    frame.render_widget(Paragraph::new(hint).wrap(Wrap { trim: false }), rows[2]);
}

#[cfg(test)]
#[path = "tests/setup_management_render.rs"]
mod tests;
