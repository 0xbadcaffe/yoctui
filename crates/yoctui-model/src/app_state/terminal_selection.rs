impl App {
    pub fn selected_terminal_session(&self) -> Option<&ClientDaemonPtySummary> {
        self.daemon.pty_sessions.get(self.pty_selection)
    }

    pub fn selected_terminal_screen(&self) -> Option<&ClientDaemonPtyScreen> {
        let id = self.selected_terminal_session()?.id;
        self.daemon
            .pty_screens
            .iter()
            .find(|screen| screen.session_id == id)
    }

    pub fn selected_terminal_details(&self) -> Option<&ClientDaemonPtyDetails> {
        let id = self.selected_terminal_session()?.id;
        self.daemon
            .pty_details
            .iter()
            .find(|details| details.id == id)
    }

    pub fn selected_terminal_is_writer(&self) -> bool {
        self.daemon.status == ClientReplicaStatus::Current
            && self.selected_terminal_details().is_some_and(|details| {
                self.terminal.client_id.is_some() && details.writer == self.terminal.client_id
            })
    }

    pub fn selected_terminal_is_menuconfig(&self) -> bool {
        self.selected_terminal_details()
            .is_some_and(|details| details.kind == ClientDaemonPtyKind::Menuconfig)
    }
}
