impl App {
    pub fn terminal_kill_target_is_current(&self) -> bool {
        self.terminal.kill_target.as_ref().is_some_and(|target| {
            self.daemon.status == ClientReplicaStatus::Current
                && self.daemon.instance_id == target.daemon_instance
                && self.pane_layout.focused == target.pane
                && self.pane_layout.contains(target.pane)
                && self.selected_terminal_session().is_some_and(|session| {
                    session.id == target.session_id
                        && session.lifecycle == ClientDaemonLifecycle::Running
                })
        })
    }

    pub fn invalidate_terminal_kill_review(&mut self) {
        if self.terminal.mode == TerminalWorkbenchMode::KillConfirmation
            && !self.terminal_kill_target_is_current()
        {
            self.terminal.reset_transient_mode();
            self.notification = Some(
                "Terminal kill review cancelled: the reviewed session is no longer current.".into(),
            );
        }
    }
}
