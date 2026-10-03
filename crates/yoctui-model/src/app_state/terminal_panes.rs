impl App {
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
        true
    }

    pub fn split_terminal_pane(&mut self, axis: SplitAxis) -> Result<PaneId, PaneLayoutError> {
        let mut layout = self.pane_layout.clone();
        let pane = layout.split(layout.focused, axis)?;
        let session = self.remember_focused_terminal();
        self.pane_layout = layout;
        self.bind_terminal_pane(pane, session);
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
