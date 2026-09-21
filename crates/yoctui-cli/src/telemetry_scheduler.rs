//! Demand-aware cadence for host and daemon telemetry.

use std::time::Duration;

use yoctui_model::{App, Screen};

pub(crate) const CLIENT_VISIBLE_INTERVAL: Duration = Duration::from_secs(1);
pub(crate) const CLIENT_BACKGROUND_INTERVAL: Duration = Duration::from_secs(10);
pub(crate) const DAEMON_ACTIVE_INTERVAL: Duration = Duration::from_secs(1);
pub(crate) const DAEMON_ATTACHED_IDLE_INTERVAL: Duration = Duration::from_secs(5);

pub(crate) fn client_telemetry_visible(app: &App) -> bool {
    matches!(app.screen, Screen::Dashboard | Screen::Tasks)
}

pub(crate) fn client_telemetry_interval(app: &App) -> Duration {
    if client_telemetry_visible(app) {
        CLIENT_VISIBLE_INTERVAL
    } else {
        CLIENT_BACKGROUND_INTERVAL
    }
}

pub(crate) fn daemon_telemetry_interval(
    connected_clients: usize,
    active_work: bool,
) -> Option<Duration> {
    if connected_clients == 0 {
        None
    } else if active_work {
        Some(DAEMON_ACTIVE_INTERVAL)
    } else {
        Some(DAEMON_ATTACHED_IDLE_INTERVAL)
    }
}

#[cfg(test)]
#[path = "tests/telemetry_scheduler/mod.rs"]
mod tests;
