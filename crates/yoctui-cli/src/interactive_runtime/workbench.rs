use super::*;
use yoctui_model::{PopupEditorCommand, WorkbenchAction as W, WorkbenchFocus as F};

impl InteractiveRuntime {
    pub(super) async fn workbench_key(&mut self, key: crossterm::event::KeyEvent) -> Result<bool> {
        if !yoctui_app::workbench_owns_input(&self.app) {
            return Ok(false);
        }
        let input = input_from_key(key);
        // Application menus remain reachable; ordinary letters never leak to global actions.
        if matches!(
            input,
            Some(Input::F1 | Input::F2 | Input::F3 | Input::F9 | Input::F12)
        ) && self.app.workbench.form.is_none()
            && self.app.workbench.menu.is_none()
        {
            if self
                .app
                .workbench
                .current()
                .is_some_and(|s| s.focus == F::Terminal)
            {
                self.release_workbench_writer();
            }
            return Ok(false);
        }
        if !self.workbench_store.ready() {
            self.app.notification = Some(
                "Workbench recovery is still loading or unavailable; see the workspace status."
                    .into(),
            );
            return Ok(true);
        }
        let terminal = self
            .app
            .workbench
            .current()
            .is_some_and(|s| s.focus == F::Terminal);
        if terminal && self.app.workbench.form.is_none() && self.app.workbench.menu.is_none() {
            if input == Some(Input::CtrlV) {
                match crate::clipboard::read_system_clipboard().await {
                    Ok(text) => {
                        let _ = update(&mut self.app, Action::TerminalStagePaste(text));
                    }
                    Err(error) => self.app.notification = Some(format!("Could not paste: {error}")),
                }
                return Ok(true);
            }
            if self.app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Live
                || input == Some(Input::Alt('/'))
            {
                if let Some(action) =
                    input.and_then(|key| yoctui_app::terminal_workspace_action(&self.app, key))
                    && let Some(effect) = compatibility_workspace_action(&mut self.app, action)
                {
                    self.dispatch_workbench_effect(effect).await;
                }
                return Ok(true);
            }
        }
        let mut action = input.and_then(|i| yoctui_app::workbench_action(&self.app, i));
        use crossterm::event::{KeyCode as K, KeyModifiers as M};
        if self.app.workbench.form.is_none() && self.app.workbench.menu.is_none() {
            action = match (key.code, key.modifiers) {
                (K::Left, M::ALT) => Some(Action::Workbench(W::Tab(-1))),
                (K::Right, M::ALT) => Some(Action::Workbench(W::Tab(1))),
                (K::Up, M::ALT) => Some(Action::Workbench(W::Folder(-1))),
                (K::Down, M::ALT) => Some(Action::Workbench(W::Folder(1))),
                (K::Char('w'), M::CONTROL) if !terminal => Some(Action::Workbench(W::CloseTab)),
                (K::Char('a'), M::CONTROL) if !terminal => Some(Action::Workbench(W::SelectAll)),
                (K::Char('x'), M::CONTROL) if !terminal => Some(Action::Workbench(W::Cut)),
                (code @ (K::Left | K::Right | K::Up | K::Down), M::SHIFT) if !terminal => self
                    .app
                    .workbench
                    .current()
                    .and_then(|s| s.tabs.get(s.tab))
                    .map(|tab| {
                        let p = tab.document.position();
                        let (line, column) = match code {
                            K::Up => (p.line.saturating_sub(1), p.column),
                            K::Down => (p.line + 1, p.column),
                            K::Left => (p.line, p.column.saturating_sub(1)),
                            _ => (p.line, p.column + 1),
                        };
                        Action::Workbench(W::Edit(PopupEditorCommand::SelectPosition {
                            line,
                            column,
                            extend: true,
                        }))
                    }),
                (K::Char('z'), M::CONTROL) if !terminal => {
                    Some(Action::Workbench(W::Edit(PopupEditorCommand::Undo)))
                }
                (K::Char('y'), M::CONTROL) if !terminal => {
                    Some(Action::Workbench(W::Edit(PopupEditorCommand::Redo)))
                }
                _ => action,
            };
        }
        if let Some(action) = action {
            if terminal && !matches!(action, Action::Workbench(W::Terminal | W::TogglePanel)) {
                self.release_workbench_writer();
            }
            if let Some(effect) = compatibility_workspace_action(&mut self.app, action) {
                self.dispatch_workbench_effect(effect).await;
            }
        } else if terminal
            && self.app.workbench.form.is_none()
            && self.app.workbench.menu.is_none()
            && let Some(effect) = terminal_workspace::terminal_writer_key_effect(&self.app, key)
        {
            let _ = submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect);
        }
        Ok(true)
    }
    pub(super) async fn dispatch_workbench_effect(&mut self, effect: Effect) {
        match effect {
            effect @ Effect::Workbench(_) => self.workbench_io.submit(&mut self.app, effect),
            Effect::CopyToClipboard(content) => copy_to_clipboard(&mut self.app, content).await,
            Effect::OpenWorkspaceEditor { label, root } => {
                open_workspace_editor(&mut self.app, label, root).await
            }
            Effect::InspectDevtoolStatus(_) => self.begin_selected_devtool_status(),
            Effect::Start(request) => {
                begin_runtime_build(
                    &mut self.daemon_runtime,
                    &mut self.backend,
                    &mut self.app,
                    &mut self.build_jobs,
                    request,
                )
                .await;
            }
            effect => {
                let _ = submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect);
            }
        }
    }
    pub(super) fn release_workbench_writer(&mut self) {
        if !self.app.workbench.current().is_some_and(|s| {
            s.terminal_daemon == self.app.daemon.instance_id
                && s.terminal == self.app.selected_terminal_session().map(|pty| pty.id)
        }) {
            return;
        }
        if !self.app.selected_terminal_is_writer() {
            return;
        }
        if let Some(details) = self.app.selected_terminal_details() {
            let effect = Effect::Terminal(yoctui_model::TerminalEffect::ReleaseControl {
                session_id: details.id,
                writer_epoch: details.writer_epoch,
            });
            let _ = submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect);
        }
    }
    pub(super) async fn poll_workbench(&mut self) {
        let changed = self.workbench_store.poll(&mut self.app).await;
        self.render_scheduler
            .invalidate_if(changed, RenderCause::State);
        if self.workbench_store.ready()
            && let Some((recipe, root)) = self.app.workbench.deferred_recipe.take()
        {
            open_workspace_editor(&mut self.app, recipe, root).await;
        }
        self.poll_workbench_terminal();
        if self.app.screen == Screen::Workbench {
            let bound = self
                .app
                .workbench
                .current()
                .filter(|s| s.terminal_daemon == self.app.daemon.instance_id)
                .and_then(|s| s.terminal);
            if let Some(index) = bound.and_then(|id| {
                self.app
                    .daemon
                    .pty_sessions
                    .iter()
                    .position(|pty| pty.id == id)
            }) && self.app.selected_terminal_session().map(|s| s.id) != bound
            {
                self.app
                    .select_terminal_pane(self.app.pane_layout.focused, index);
                self.app.terminal.single_session = true;
            }
        }
        if self.workbench_io.finished() {
            if let Some(effect) = self.workbench_io.poll(&mut self.app).await {
                self.dispatch_workbench_effect(effect).await;
            }
            self.render_scheduler.invalidate(RenderCause::State);
        }
        if self.app.screen == Screen::Workbench
            && self.app.workbench.current().is_some_and(|s| {
                s.folders
                    .get(s.folder)
                    .is_some_and(|f| f.browser.nodes.is_empty() && !f.loading && f.error.is_none())
            })
            && let Some(effect) = update(&mut self.app, Action::Workbench(W::Refresh))
        {
            self.workbench_io.submit(&mut self.app, effect);
        }
    }
    pub(super) async fn flush_workbench(&mut self) -> Result<()> {
        while self.workbench_io.busy() {
            if let Some(effect) = self.workbench_io.poll(&mut self.app).await {
                self.workbench_io.submit(&mut self.app, effect);
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
        self.workbench_store.flush(&mut self.app).await
    }
    fn poll_workbench_terminal(&mut self) {
        let Some((id, name, after, instance, started)) = self.pending_workbench_terminal.clone()
        else {
            return;
        };
        if instance != self.app.daemon.instance_id || started.elapsed() > Duration::from_secs(30) {
            self.pending_workbench_terminal = None;
            self.app.notification = Some(
                "Workspace terminal did not become available. Check Daemons or try again.".into(),
            );
            return;
        }
        let Some(index) = ready_terminal(&self.app, &name, after) else {
            return;
        };
        let terminal_id = self.app.daemon.pty_sessions[index].id;
        self.pending_workbench_terminal = None;
        if let Some(session) = self.app.workbench.sessions.iter_mut().find(|s| s.id == id) {
            session.terminal = Some(terminal_id);
            session.terminal_daemon = instance;
        }
        if self.app.screen == Screen::Workbench
            && self.app.workbench.active == Some(id)
            && self
                .app
                .workbench
                .current()
                .is_some_and(|s| s.focus == F::Terminal)
        {
            self.app
                .select_terminal_pane(self.app.pane_layout.focused, index);
            self.app.terminal.browser_open = false;
            self.app.terminal.single_session = true;
            if let Some(effect) =
                compatibility_workspace_action(&mut self.app, Action::TerminalTakeControl)
            {
                let _ = submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect);
            }
            self.render_scheduler.invalidate(RenderCause::State);
        }
    }
}

fn ready_terminal(app: &App, name: &str, after: u64) -> Option<usize> {
    app.daemon.pty_sessions.iter().position(|s| {
        s.id > after
            && s.name == name
            && s.lifecycle == yoctui_model::ClientDaemonLifecycle::Running
    })
}

#[cfg(test)]
#[path = "../tests/interactive_runtime/workbench.rs"]
mod tests;
