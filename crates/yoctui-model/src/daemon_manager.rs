//! Client-local controls for the single local daemon. No process handles in the model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonControl {
    Start,
    Stop,
    Restart,
    Configure,
}

impl DaemonControl {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Restart => "Restart",
            Self::Configure => "Save configuration and restart",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DaemonManagerState {
    pub details: Vec<String>,
    pub logs: Vec<String>,
    pub service_active: bool,
    pub service_installed: bool,
    pub loading: bool,
    pub message: Option<String>,
    pub logs_visible: bool,
    pub scroll: usize,
    pub visible_rows: usize,
    pub editing: bool,
    pub build_directory: String,
    pub source_directory: String,
    pub source_field: bool,
    pub review: Option<DaemonControl>,
}

impl DaemonManagerState {
    pub fn edited_path(&mut self) -> &mut String {
        if self.source_field {
            &mut self.source_directory
        } else {
            &mut self.build_directory
        }
    }
}

impl crate::App {
    pub fn daemon_manager_lines(&self) -> Vec<String> {
        let state = &self.daemon_manager;
        if state.logs_visible {
            return state.logs.clone();
        }
        let mut lines = state.details.clone();
        lines.push(format!(
            "IPC replica: {:?} · BitBake: {:?}",
            self.daemon.status, self.daemon.bitbake
        ));
        lines.push(format!(
            "Compatibility: {}",
            if self.workspace_compatibility.authority().is_some() {
                "ready (see Compatibility for exact evidence)"
            } else {
                "pending or unavailable; see daemon logs"
            }
        ));
        lines.push(format!(
            "Workspace inventory: {} recipes · build {}",
            self.workspace.recipes.len(),
            self.workspace
                .build_dir
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "not loaded".into())
        ));
        if let Some(t) = self.daemon.telemetry {
            lines.push(format!(
                "Uptime {}s · memory {:?} bytes · queue {} · resync {} · slow clients {}",
                t.uptime_seconds,
                t.memory_bytes,
                t.queue_depth,
                t.pressure.forced_resynchronizations,
                t.pressure.slow_client_disconnects
            ));
        }
        if self
            .daemon
            .jobs
            .iter()
            .all(|job| job.lifecycle.is_terminal())
            && self
                .daemon
                .pty_sessions
                .iter()
                .all(|session| session.lifecycle.is_terminal())
        {
            lines.push(if self.workspace_compatibility.authority().is_none() {
                "Current work: compatibility discovery pending or unavailable".into()
            } else if self.workspace.recipes.is_empty() {
                "Current work: workspace inventory pending or unavailable".into()
            } else {
                "Current work: idle (no active jobs or terminal sessions)".into()
            });
        }
        lines.extend(
            self.daemon
                .jobs
                .iter()
                .filter(|job| !job.lifecycle.is_terminal())
                .map(|job| {
                    format!(
                        "Working: {} · {:?} · {:?}/{:?}",
                        job.label, job.lifecycle, job.progress_current, job.progress_total
                    )
                }),
        );
        lines.extend(
            self.daemon
                .pty_sessions
                .iter()
                .filter(|session| !session.lifecycle.is_terminal())
                .map(|session| format!("Terminal: {} · {:?}", session.name, session.lifecycle)),
        );
        lines.extend(
            self.daemon
                .recovery_warnings
                .iter()
                .map(|warning| format!("Recovery: {warning}")),
        );
        lines.extend(
            self.daemon
                .recent_logs
                .iter()
                .rev()
                .take(5)
                .map(|log| format!("Recent activity: {log}")),
        );
        lines
    }

    pub fn scroll_daemon_manager(&mut self, delta: isize) {
        let maximum = self
            .daemon_manager_lines()
            .len()
            .saturating_sub(self.daemon_manager.visible_rows.max(1));
        self.daemon_manager.scroll = self
            .daemon_manager
            .scroll
            .min(maximum)
            .saturating_add_signed(delta)
            .min(maximum);
    }
}

#[cfg(test)]
#[path = "tests/daemon_manager.rs"]
mod tests;
