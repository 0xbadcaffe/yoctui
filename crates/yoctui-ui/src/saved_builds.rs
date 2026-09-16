//! Saved history has its own selection; it never replaces live task/log state.
use super::*;
use yoctui_model::{SavedBuildOutcome, SavedBuildView};
pub(crate) fn saved_build_history(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    let state = &app.saved_builds;
    let block = pane_block(
        app,
        "Saved builds · offline archive · r refresh · l live jobs",
        true,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if state.loading {
        frame.render_widget(Paragraph::new("Loading saved builds…"), inner);
        return;
    }
    let Some(record) = state.records.get(state.selection) else {
        frame.render_widget(Paragraph::new(state.notice.as_deref().unwrap_or("No saved builds. New daemon builds are saved automatically; older logs may be unavailable.")).wrap(Wrap { trim:true }),inner);
        return;
    };
    if let Some(view) = state.view {
        let mut lines = vec![
            Line::from(format!(
                "{} · {:?} · saved {} ago",
                record.target,
                record.outcome,
                format_duration(
                    now.duration_since(UNIX_EPOCH + Duration::from_millis(record.saved_unix_ms))
                        .unwrap_or_default()
                )
            )),
            Line::from(format!(
                "←/→ Summary | Logs | Tasks | Errors · {view:?} · PgUp/PgDn scroll · Esc list"
            )),
        ];
        let content: Vec<String> = match view {
            SavedBuildView::Summary => {
                let elapsed = record
                    .started_unix_ms
                    .zip(record.finished_unix_ms)
                    .and_then(|(s, e)| e.checked_sub(s));
                let mut rows = vec![
                    format!(
                        "Machine: {}",
                        record.machine.as_deref().unwrap_or("not recorded")
                    ),
                    format!(
                        "Source: {}",
                        record.source.as_deref().unwrap_or("not recorded")
                    ),
                    format!(
                        "Build directory: {}",
                        record.build_dir.as_deref().unwrap_or("not recorded")
                    ),
                    format!(
                        "Duration: {}",
                        elapsed
                            .map(|v| format!("{} s", v / 1000))
                            .unwrap_or_else(|| "not recorded".into())
                    ),
                    format!(
                        "Saved logs: {} · recorded tasks: {}",
                        record.logs.len(),
                        record.tasks.len()
                    ),
                ];
                if record.outcome == SavedBuildOutcome::Incomplete {
                    rows.push("Last observed incomplete; this archive does not prove the build is still running.".into());
                }
                rows.extend(record.limitations.clone());
                rows
            }
            SavedBuildView::Tasks => {
                if record.tasks.is_empty() {
                    vec!["Recorded tasks unavailable for this build.".into()]
                } else {
                    record
                        .tasks
                        .iter()
                        .map(|t| format!("{}:{} · {}", t.recipe, t.task, t.status))
                        .collect()
                }
            }
            SavedBuildView::Logs | SavedBuildView::Errors => {
                let rows: Vec<_> = record
                    .logs
                    .iter()
                    .filter(|l| {
                        view != SavedBuildView::Errors
                            || matches!(l.severity, Severity::Error | Severity::Warning)
                    })
                    .map(|l| {
                        format!(
                            "+{}s {:?} {}",
                            l.unix_ms
                                .saturating_sub(record.started_unix_ms.unwrap_or(l.unix_ms))
                                / 1000,
                            l.severity,
                            l.message
                        )
                    })
                    .collect();
                if rows.is_empty() {
                    vec!["Saved logs/errors unavailable: no matching lines retained.".into()]
                } else {
                    rows
                }
            }
        };
        let rows = content
            .iter()
            .flat_map(|s| s.lines())
            .take(4096)
            .collect::<Vec<_>>();
        let offset = state.scroll.min(rows.len().saturating_sub(1));
        lines.extend(
            rows.into_iter()
                .skip(offset)
                .map(|s| Line::from(s.to_owned())),
        );
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    } else {
        let capacity = usize::from(inner.height.saturating_sub(2)).max(1);
        let start = state.selection.saturating_sub(capacity.saturating_sub(1));
        let rows = state
            .records
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
            .map(|(i, r)| {
                Row::new([
                    r.target.clone(),
                    r.machine.clone().unwrap_or_else(|| "unknown".into()),
                    format!("{:?}", r.outcome),
                    format!("{}", r.logs.len()),
                ])
                .style(selected_style(app, i == state.selection))
            });
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Percentage(45),
                    Constraint::Percentage(25),
                    Constraint::Percentage(20),
                    Constraint::Percentage(10),
                ],
            )
            .header(Row::new([
                "Target · Enter details",
                "Machine",
                "Saved outcome",
                "Logs",
            ])),
            inner,
        );
    }
}
