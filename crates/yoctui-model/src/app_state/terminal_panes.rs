impl App {
    pub fn terminal_browser_indexes(&self) -> Vec<usize> {
        self.daemon.pty_sessions.iter().enumerate()
            .filter(|(_, session)| !session.lifecycle.is_terminal())
            .map(|(index, _)| index).collect()
    }

    pub fn terminal_browser_visible(&self) -> bool {
        self.screen == Screen::TerminalSessions && self.terminal.browser_open
    }

    pub fn terminal_console_fullscreen(&self) -> bool {
        self.screen == Screen::TerminalSessions && !self.terminal.browser_open
            && self.selected_terminal_session().is_some()
    }

    pub fn select_terminal_browser_session(&mut self, delta: isize) {
        let indexes = self.terminal_browser_indexes();
        if indexes.is_empty() { return; }
        let current = indexes.iter().position(|index| Some(*index) == self.selected_terminal_index()).unwrap_or(0);
        let next = if delta < 0 { current.saturating_sub(delta.unsigned_abs()) }
            else { current.saturating_add(delta as usize).min(indexes.len() - 1) };
        self.select_terminal_pane(self.pane_layout.focused, indexes[next]);
    }

    /// Reopening during release waits for the authoritative writer epoch, then
    /// requests control once. Another client's lease is never taken over.
    pub fn pending_terminal_writer_effect(&mut self) -> Option<TerminalEffect> {
        let (session_id, released_epoch) = self.terminal.releasing_writer?;
        if self.daemon.status != ClientReplicaStatus::Current { return None; }
        let details = self.daemon.pty_details.iter().find(|details| details.id == session_id);
        let running = self.daemon.pty_sessions.iter().any(|session|
            session.id == session_id && session.lifecycle == ClientDaemonLifecycle::Running);
        if running && details.is_some_and(|details|
            details.writer_epoch == released_epoch && details.writer.is_some()
                && details.writer == self.terminal.client_id) { return None; }
        let reopen = self.terminal.reopen_after_release;
        self.terminal.releasing_writer = None;
        self.terminal.reopen_after_release = false;
        let workbench_console = self.screen == Screen::Workbench
            && !self.workbench.modal_open() && self.active_dialog().is_none()
            && !self.menu.is_open() && !self.command_palette_open
            && self.workbench.current().is_some_and(|session|
                session.focus == crate::WorkbenchFocus::Terminal
                    && session.terminal == Some(session_id)
                    && session.terminal_daemon == self.daemon.instance_id);
        if reopen && running && (self.terminal_console_fullscreen() || workbench_console)
            && self.focus == FocusTarget::Workspace
            && self.selected_terminal_session().is_some_and(|session| session.id == session_id)
            && let Some(details) = details.filter(|details| details.writer.is_none()) {
            return Some(TerminalEffect::TakeControl {
                session_id,
                expected_epoch: details.writer_epoch,
            });
        }
        None
    }

    /// Shared typed projection for terminal rendering, mouse selection and resize.
    pub fn terminal_pane_session_index(
        &self,
        pane: PaneId,
        ordinal: usize,
        pane_count: usize,
    ) -> Option<usize> {
        if pane_count == 0 || !self.pane_layout.contains(pane) {
            return None;
        }
        if pane_count == 1 || pane == self.pane_layout.focused {
            return self.selected_terminal_index();
        }
        if !self.terminal.pane_sessions.is_empty()
            && self.terminal.pane_daemon_instance != self.daemon.instance_id
        {
            return None;
        }
        match self
            .terminal
            .pane_sessions
            .iter()
            .find(|(id, _)| *id == pane)
        {
            Some((_, session)) => session.and_then(|id| {
                self.daemon
                    .pty_sessions
                    .iter()
                    .position(|session| session.id == id)
            }),
            None => (ordinal < self.daemon.pty_sessions.len()).then_some(ordinal),
        }
    }

    pub fn reconcile_terminal_panes(&mut self) {
        let replaced = self.terminal.pane_daemon_instance != self.daemon.instance_id;
        if replaced {
            self.terminal.releasing_writer = None;
            self.terminal.reopen_after_release = false;
            self.terminal.retain_created_output = false;
            self.terminal.retained_output_session = None;
        }
        if replaced && !self.terminal.pane_sessions.is_empty() {
            self.pty_selection = usize::MAX;
        }
        self.terminal.pane_daemon_instance = self.daemon.instance_id;
        self.terminal.pane_sessions.retain_mut(|(pane, session)| {
            if !self.pane_layout.contains(*pane) {
                return false;
            }
            if replaced
                || !self
                    .daemon
                    .pty_sessions
                    .iter()
                    .any(|pty| Some(pty.id) == *session)
            {
                *session = None;
            }
            true
        });
        if self.terminal.awaiting_session {
            if replaced {
                self.terminal.awaiting_session = false;
                self.terminal.awaiting_after_session = None;
                self.pty_selection = usize::MAX;
            } else if let Some((index, id)) = self.daemon.pty_sessions.iter().enumerate()
                .filter(|(_, session)| session.id > self.terminal.awaiting_after_session.unwrap_or(0))
                .min_by_key(|(_, session)| session.id)
                .map(|(index, session)| (index, session.id)) {
                self.bind_terminal_pane(self.pane_layout.focused, Some(id));
                if self.terminal.retain_created_output {
                    self.terminal.retained_output_session = Some(id);
                }
                self.terminal.retain_created_output = false;
                self.pty_selection = index;
                self.terminal.awaiting_session = false;
                self.terminal.awaiting_after_session = None;
            }
        }
        if let Some((_, session)) = self
            .terminal
            .pane_sessions
            .iter()
            .find(|(pane, _)| *pane == self.pane_layout.focused)
        {
            self.pty_selection = session
                .and_then(|id| {
                    self.daemon
                        .pty_sessions
                        .iter()
                        .position(|session| session.id == id)
                })
                .unwrap_or(usize::MAX);
        }
        self.invalidate_terminal_kill_review();
        // Do not delete daemon history or hide sessions on a transient disconnect.
        // A terminal that actually exited, disappeared, or belongs to an old
        // daemon identity is no longer an openable console in this browser.
        // Keep an explicitly requested debug report readable until the user
        // returns to the tree. It remains excluded from the tree itself.
        let completed_report = !replaced && !self.terminal.browser_open
            && self.selected_terminal_session().is_some_and(|session|
                self.terminal.retained_output_session == Some(session.id)
                    && matches!(session.lifecycle, ClientDaemonLifecycle::Exited
                        | ClientDaemonLifecycle::Failed | ClientDaemonLifecycle::Cancelled))
            && self.selected_terminal_details().is_some_and(|details|
                details.kind == ClientDaemonPtyKind::Utility);
        if self.screen == Screen::TerminalSessions
            && self.daemon.status == ClientReplicaStatus::Current
            && !self.terminal.awaiting_session
            && !completed_report
            && self.selected_terminal_session().is_none_or(|session| session.lifecycle.is_terminal()) {
            self.terminal.browser_open = true;
        }
    }

    fn bind_terminal_pane(&mut self, pane: PaneId, session: Option<u64>) {
        if let Some((_, current)) = self
            .terminal
            .pane_sessions
            .iter_mut()
            .find(|(id, _)| *id == pane)
        {
            *current = session;
        } else {
            self.terminal.pane_sessions.push((pane, session));
        }
    }

    fn remember_focused_terminal(&mut self) -> Option<u64> {
        self.reconcile_terminal_panes();
        let session = self.selected_terminal_session().map(|session| session.id);
        self.bind_terminal_pane(self.pane_layout.focused, session);
        session
    }

    /// Preserve the existing confirmed-launch next-slot selection, not an old binding.
    pub fn prepare_created_terminal_selection(&mut self) {
        self.remember_focused_terminal();
        self.terminal.browser_open = false;
        self.terminal.awaiting_session = true;
        self.terminal.retain_created_output = false;
        self.terminal.retained_output_session = None;
        self.terminal.awaiting_after_session = Some(
            self.daemon.pty_sessions.iter().map(|session| session.id).max().unwrap_or(0)
        );
        self.terminal.single_session = true;
        self.terminal
            .pane_sessions
            .retain(|(pane, _)| *pane != self.pane_layout.focused);
        self.pty_selection = self.daemon.pty_sessions.len();
    }

    pub fn select_terminal_session(&mut self, delta: isize) {
        self.reconcile_terminal_panes();
        let count = self.daemon.pty_sessions.len();
        let selected = self.selected_terminal_index().unwrap_or(0);
        self.pty_selection = if count == 0 {
            0
        } else if delta.is_negative() {
            selected.saturating_sub(delta.unsigned_abs()).min(count - 1)
        } else {
            selected.saturating_add(delta as usize).min(count - 1)
        };
        let session = self
            .daemon
            .pty_sessions
            .get(self.pty_selection)
            .map(|session| session.id);
        self.bind_terminal_pane(self.pane_layout.focused, session);
        self.invalidate_terminal_kill_review();
    }

    pub fn select_terminal_pane(&mut self, pane: PaneId, index: usize) -> bool {
        if index >= self.daemon.pty_sessions.len() || !self.pane_layout.contains(pane) {
            return false;
        }
        self.remember_focused_terminal();
        if self.pane_layout.focus(pane).is_err() {
            return false;
        }
        self.pty_selection = index;
        self.bind_terminal_pane(pane, Some(self.daemon.pty_sessions[index].id));
        self.invalidate_terminal_kill_review();
        true
    }

    pub fn split_terminal_pane(&mut self, axis: SplitAxis) -> Result<PaneId, PaneLayoutError> {
        let mut layout = self.pane_layout.clone();
        let pane = layout.split(layout.focused, axis)?;
        let session = self.remember_focused_terminal();
        self.pane_layout = layout;
        self.terminal.single_session = false;
        self.bind_terminal_pane(pane, session);
        self.invalidate_terminal_kill_review();
        Ok(pane)
    }

    pub fn close_terminal_pane(&mut self) -> Result<PaneId, PaneLayoutError> {
        let mut layout = self.pane_layout.clone();
        let pane = layout.close(layout.focused)?;
        self.remember_focused_terminal();
        self.pane_layout = layout;
        self.reconcile_terminal_panes();
        Ok(pane)
    }
}
