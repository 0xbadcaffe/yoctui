//! PTY events change only one terminal, never the build/log/raw projections.
use super::DaemonClientSnapshot;
use yoctui_model::{App, ClientDaemonPtySummary, ClientReplicaStatus, DaemonModelInstanceId};
use yoctui_protocol::daemon::DaemonEvent;

impl DaemonClientSnapshot {
    pub(super) fn install_terminal_event(&self, app: &mut App, event: &DaemonEvent) -> bool {
        let Some(snapshot) = &self.snapshot else {
            return false;
        };
        if app.daemon.status != ClientReplicaStatus::Current
            || app.daemon.instance_id != Some(DaemonModelInstanceId(snapshot.daemon_instance_id.0))
            || app.daemon.sequence.checked_add(1) != Some(snapshot.sequence)
        {
            // Initial attachment, a caller that skipped projection, or a new
            // daemon must still install the full authoritative snapshot.
            return false;
        }
        match event {
            DaemonEvent::PtyOutput { .. } => {} // ANSI bytes are not reparsed in the client.
            DaemonEvent::PtyScreen(screen) => {
                app.daemon
                    .pty_screens
                    .retain(|old| old.session_id != screen.session_id.0);
                app.daemon
                    .pty_screens
                    .push(crate::terminal_view::terminal_screen(screen));
            }
            DaemonEvent::PtyChanged(pty) => {
                replace(
                    &mut app.daemon.pty_sessions,
                    ClientDaemonPtySummary {
                        id: pty.id.0,
                        name: pty.name.clone(),
                        lifecycle: crate::daemon_view::client_daemon_lifecycle(pty.lifecycle),
                        viewers: pty.viewers,
                    },
                    |entry| entry.id,
                );
                replace(
                    &mut app.daemon.pty_details,
                    crate::terminal_view::terminal_details(pty),
                    |entry| entry.id,
                );
                app.reconcile_platform_menuconfigs();
                app.reconcile_terminal_panes();
            }
            DaemonEvent::PtyRemoved { session_id } => {
                app.daemon.pty_sessions.retain(|pty| pty.id != session_id.0);
                app.daemon.pty_details.retain(|pty| pty.id != session_id.0);
                app.daemon
                    .pty_screens
                    .retain(|screen| screen.session_id != session_id.0);
                app.reconcile_platform_menuconfigs();
                app.reconcile_terminal_panes();
            }
            _ => return false,
        }
        app.daemon.sequence = snapshot.sequence;
        app.daemon.generation = snapshot.generation;
        app.observe_daemon(true, std::time::SystemTime::now());
        true
    }
}

fn replace<T>(items: &mut Vec<T>, value: T, id: impl Fn(&T) -> u64) {
    if let Some(current) = items.iter_mut().find(|item| id(item) == id(&value)) {
        *current = value;
    } else {
        items.push(value);
    }
}
