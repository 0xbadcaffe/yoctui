use super::*;

pub(super) fn reduce(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    use WorkbenchAction as W;
    match action {
        W::Tabs => {
            app.workbench.tab_picker = app
                .workbench
                .current()
                .filter(|s| !s.tabs.is_empty())
                .map(|s| s.tab);
        }
        W::TabPickerSelect(delta) => {
            let count = app.workbench.current()?.tabs.len();
            app.workbench.tab_picker = Some(shifted_index(app.workbench.tab_picker?, delta, count));
        }
        W::TabPickerOpen => {
            let index = app.workbench.tab_picker.take()?;
            return super::reduce(app, W::SelectTab(index));
        }
        W::TabPickerClose => {
            let index = app.workbench.tab_picker?;
            let s = app.workbench.current_mut()?;
            let t = s.tabs.get(index)?;
            if t.is_dirty() || t.saving {
                app.notification = Some("Save this tab before closing it.".into());
                return None;
            }
            s.tabs.remove(index);
            s.tab = if s.tab > index {
                s.tab - 1
            } else {
                s.tab.min(s.tabs.len().saturating_sub(1))
            };
            app.workbench.tab_picker =
                (!s.tabs.is_empty()).then_some(index.min(s.tabs.len().saturating_sub(1)));
            app.workbench.changed();
        }
        W::ToggleWhitespace => {
            let s = app.workbench.current_mut()?;
            s.show_whitespace = !s.show_whitespace;
            app.workbench.changed();
        }
        W::SearchMatch { backwards } => {
            let s = app.workbench.current_mut()?;
            let t = s.tabs.get_mut(s.tab)?;
            if !t.document.next_match(backwards) {
                app.notification =
                    Some("Ctrl+f searches the current file; Ctrl+g next, Alt+p previous.".into());
            } else {
                t.reveal_cursor();
                app.workbench.changed();
            }
        }
        W::ToggleBreakpoint | W::BreakpointAt(_) => {
            let s = app.workbench.current_mut()?;
            let count: usize = s.tabs.iter().map(|t| t.breakpoints.len()).sum();
            let t = s.tabs.get_mut(s.tab)?;
            let line = if let W::BreakpointAt(line) = action {
                line
            } else {
                t.document.position().line + 1
            };
            if line == 0 || line > t.document.line_count() {
                return None;
            }
            s.focus = WorkbenchFocus::Editor;
            if !t.breakpoints.remove(&line) {
                if count >= WORKBENCH_MAX_BREAKPOINTS {
                    app.notification =
                        Some("Remove a breakpoint before adding another (128 limit).".into());
                    return None;
                }
                t.breakpoints.insert(line);
            }
            app.workbench.changed();
        }
        W::ScrollEditor(delta) => {
            let s = app.workbench.current_mut()?;
            let t = s.tabs.get_mut(s.tab)?;
            t.viewport.top = shifted_index(
                t.viewport.top,
                delta,
                t.document
                    .line_count()
                    .saturating_sub(t.viewport.rows.max(1))
                    .saturating_add(1),
            );
            t.viewport.follow_cursor = false;
        }
        W::Viewport { rows, columns } => {
            let s = app.workbench.current_mut()?;
            let t = s.tabs.get_mut(s.tab)?;
            t.viewport.rows = rows.max(1);
            t.viewport.columns = columns.max(1);
            t.document.layout_mut().viewport_rows = rows.max(1);
            if t.viewport.follow_cursor {
                t.reveal_cursor();
            }
            t.viewport.top = t
                .viewport
                .top
                .min(t.document.line_count().saturating_sub(rows.max(1)));
        }
        _ => {}
    }
    None
}
