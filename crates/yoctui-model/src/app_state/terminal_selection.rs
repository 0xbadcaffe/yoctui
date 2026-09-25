impl App {
    pub fn selected_terminal_index(&self) -> Option<usize> {
        self.embedded_platform_terminal()
            .and_then(|terminal| terminal.session_id)
            .and_then(|id| {
                self.daemon
                    .pty_sessions
                    .iter()
                    .position(|session| session.id == id)
            })
            .or_else(|| {
                (self.pty_selection < self.daemon.pty_sessions.len()).then_some(self.pty_selection)
            })
    }

    pub fn selected_terminal_session(&self) -> Option<&ClientDaemonPtySummary> {
        self.selected_terminal_index()
            .and_then(|index| self.daemon.pty_sessions.get(index))
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

    pub fn begin_platform_menuconfig(&mut self, screen: Screen, name: String) {
        let terminal = match screen {
            Screen::Kernel => &mut self.kernel.menuconfig_terminal,
            Screen::Firmware => &mut self.firmware.menuconfig_terminal,
            _ => return,
        };
        *terminal = PlatformTerminalState {
            name: Some(name),
            prior_session_ids: self
                .daemon
                .pty_sessions
                .iter()
                .map(|session| session.id)
                .collect(),
            session_id: None,
            writer_control_requested: false,
        };
    }

    pub fn reconcile_platform_menuconfigs(&mut self) {
        reconcile_platform_terminal(&mut self.kernel.menuconfig_terminal, &self.daemon);
        reconcile_platform_terminal(&mut self.firmware.menuconfig_terminal, &self.daemon);
    }

    pub fn cancel_pending_platform_menuconfig(&mut self) {
        for terminal in [
            &mut self.kernel.menuconfig_terminal,
            &mut self.firmware.menuconfig_terminal,
        ] {
            if terminal.name.is_some() && terminal.session_id.is_none() {
                *terminal = PlatformTerminalState::default();
            }
        }
    }

    pub fn platform_menuconfig_visible(&self) -> bool {
        self.embedded_platform_terminal()
            .and_then(|terminal| terminal.session_id)
            .is_some_and(|id| {
                self.daemon.pty_sessions.iter().any(|session| {
                    session.id == id && !session.lifecycle.is_terminal()
                })
            })
    }

    pub fn platform_menuconfig_waiting(&self) -> bool {
        [
            (&self.kernel.menuconfig_terminal, "Kernel"),
            (&self.firmware.menuconfig_terminal, "U-Boot"),
        ]
        .into_iter()
        .any(|(terminal, _)| platform_terminal_waiting(terminal, &self.daemon))
    }

    pub fn platform_menuconfig_waiting_label(&self) -> Option<String> {
        [
            (&self.kernel.menuconfig_terminal, "Kernel"),
            (&self.firmware.menuconfig_terminal, "U-Boot"),
        ]
        .into_iter()
        .find(|(terminal, _)| platform_terminal_waiting(terminal, &self.daemon))
        .map(|(_, label)| format!("Starting {label} menuconfig"))
    }

    pub fn embedded_platform_terminal_label(&self) -> Option<&'static str> {
        match self.screen {
            Screen::Kernel if self.platform_menuconfig_visible() => Some("Kernel menuconfig"),
            Screen::Firmware if self.platform_menuconfig_visible() => Some("U-Boot menuconfig"),
            _ => None,
        }
    }

    pub fn pending_platform_writer_effect(&mut self) -> Option<TerminalEffect> {
        let client_id = self.terminal.client_id?;
        for terminal in [
            &mut self.kernel.menuconfig_terminal,
            &mut self.firmware.menuconfig_terminal,
        ] {
            let Some(session_id) = terminal.session_id else {
                continue;
            };
            let Some(session) = self
                .daemon
                .pty_sessions
                .iter()
                .find(|session| session.id == session_id)
            else {
                continue;
            };
            let Some(details) = self
                .daemon
                .pty_details
                .iter()
                .find(|details| details.id == session_id)
            else {
                continue;
            };
            if details.writer == Some(client_id) {
                terminal.writer_control_requested = true;
                continue;
            }
            if session.lifecycle == ClientDaemonLifecycle::Running
                && details.writer.is_none()
                && !terminal.writer_control_requested
            {
                terminal.writer_control_requested = true;
                return Some(TerminalEffect::TakeControl {
                    session_id,
                    expected_epoch: details.writer_epoch,
                });
            }
        }
        None
    }

    pub fn retry_platform_writer_control(&mut self, session_id: u64) {
        for terminal in [
            &mut self.kernel.menuconfig_terminal,
            &mut self.firmware.menuconfig_terminal,
        ] {
            if terminal.session_id == Some(session_id) {
                terminal.writer_control_requested = false;
            }
        }
    }

    fn embedded_platform_terminal(&self) -> Option<&PlatformTerminalState> {
        match self.screen {
            Screen::Kernel => Some(&self.kernel.menuconfig_terminal),
            Screen::Firmware => Some(&self.firmware.menuconfig_terminal),
            _ => None,
        }
    }
}

fn reconcile_platform_terminal(terminal: &mut PlatformTerminalState, daemon: &ClientDaemonView) {
    if terminal.name.is_none() {
        return;
    }
    if let Some(session_id) = terminal.session_id {
        let Some(session) = daemon
            .pty_sessions
            .iter()
            .find(|session| session.id == session_id)
        else {
            *terminal = PlatformTerminalState::default();
            return;
        };
        if session.lifecycle.is_terminal() {
            *terminal = PlatformTerminalState::default();
        }
        return;
    }
    let Some(name) = terminal.name.as_deref() else {
        return;
    };
    terminal.session_id = daemon
        .pty_sessions
        .iter()
        .find(|session| {
            session.name == name
                && !terminal.prior_session_ids.contains(&session.id)
                && daemon.pty_details.iter().any(|details| {
                    details.id == session.id && details.kind == ClientDaemonPtyKind::Menuconfig
                })
        })
        .map(|session| session.id);
}

fn platform_terminal_waiting(
    terminal: &PlatformTerminalState,
    daemon: &ClientDaemonView,
) -> bool {
    let Some(_) = terminal.name.as_ref() else {
        return false;
    };
    let Some(session_id) = terminal.session_id else {
        return true;
    };
    let running = daemon.pty_sessions.iter().any(|session| {
        session.id == session_id && session.lifecycle == ClientDaemonLifecycle::Running
    });
    let screen_ready = daemon
        .pty_screens
        .iter()
        .any(|screen| screen.session_id == session_id && !screen.rows.is_empty());
    !running || !screen_ready
}
