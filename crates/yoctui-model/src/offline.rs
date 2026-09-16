//! Client connection provenance, independent of configured filesystem paths.
use crate::{App, BuildEnvironmentState, ClientReplicaStatus};
use std::time::SystemTime;

impl App {
    pub fn is_offline(&self) -> bool {
        self.require_daemon && self.daemon.status != ClientReplicaStatus::Current
    }

    pub fn offline_notice(&self) -> Option<String> {
        if self.require_daemon
            && matches!(self.build_environment, BuildEnvironmentState::Unconfigured)
            && self.last_daemon_update.is_none()
        {
            return Some(
                "No build environment · Build Environment: configure/clone · F3 saved builds"
                    .into(),
            );
        }
        if !self.is_offline() {
            return None;
        }
        if let Some(updated) = self.last_daemon_update {
            let age = SystemTime::now()
                .duration_since(updated)
                .unwrap_or_default()
                .as_secs();
            Some(format!(
                "Disconnected · last updated {age}s ago · cached data, not live · F3 saved builds · reconnect daemon"
            ))
        } else if matches!(self.build_environment, BuildEnvironmentState::Unconfigured) {
            Some(
                "No build environment · Build Environment: configure/clone · F3 saved builds"
                    .into(),
            )
        } else {
            Some("Daemon disconnected · local files available · F3 saved builds · start/connect daemon".into())
        }
    }

    pub fn observe_daemon(&mut self, current: bool, now: SystemTime) {
        self.require_daemon = true;
        if current {
            self.last_daemon_update = Some(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Action, Screen};
    use std::time::UNIX_EPOCH;
    #[test]
    fn offline_navigation_preserves_last_observed_state() {
        let mut app = App::new_unconfigured(32, 4096);
        app.require_daemon = true;
        for screen in [
            Screen::Dashboard,
            Screen::BuildHistory,
            Screen::Logs,
            Screen::Tasks,
            Screen::Settings,
            Screen::Kernel,
            Screen::Firmware,
            Screen::Images,
            Screen::Packages,
        ] {
            assert!(
                crate::update_with_workspace_authority(&mut app, Action::Open(screen)).is_none()
            );
            assert_eq!(app.screen, screen);
        }
        assert!(
            app.offline_notice()
                .unwrap()
                .contains("No build environment")
        );
        app.daemon.status = ClientReplicaStatus::Current;
        app.observe_daemon(true, UNIX_EPOCH);
        assert!(app.offline_notice().is_none());
        app.daemon.status = ClientReplicaStatus::Disconnected;
        app.observe_daemon(false, SystemTime::now());
        assert_eq!(app.last_daemon_update, Some(UNIX_EPOCH));
        assert!(app.offline_notice().unwrap().contains("not live"));
    }
}
