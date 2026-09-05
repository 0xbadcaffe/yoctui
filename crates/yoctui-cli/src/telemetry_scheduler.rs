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
mod tests {
    use super::*;

    #[test]
    fn client_sampling_is_fast_only_where_host_telemetry_is_visible() {
        let mut app = App::new(8, 1024);
        assert_eq!(client_telemetry_interval(&app), CLIENT_VISIBLE_INTERVAL);
        app.screen = Screen::Tasks;
        assert_eq!(client_telemetry_interval(&app), CLIENT_VISIBLE_INTERVAL);
        app.screen = Screen::Recipes;
        assert!(!client_telemetry_visible(&app));
        assert_eq!(client_telemetry_interval(&app), CLIENT_BACKGROUND_INTERVAL);
    }

    #[test]
    fn daemon_sampling_pauses_without_clients_and_scales_with_work() {
        assert_eq!(daemon_telemetry_interval(0, false), None);
        assert_eq!(daemon_telemetry_interval(0, true), None);
        assert_eq!(
            daemon_telemetry_interval(1, false),
            Some(DAEMON_ATTACHED_IDLE_INTERVAL)
        );
        assert_eq!(
            daemon_telemetry_interval(1, true),
            Some(DAEMON_ACTIVE_INTERVAL)
        );
    }

    #[test]
    fn telemetry_cadences_are_low_frequency_and_bounded() {
        assert!(CLIENT_VISIBLE_INTERVAL >= Duration::from_secs(1));
        assert!(CLIENT_BACKGROUND_INTERVAL >= Duration::from_secs(5));
        assert!(DAEMON_ACTIVE_INTERVAL >= Duration::from_secs(1));
        assert!(DAEMON_ATTACHED_IDLE_INTERVAL >= Duration::from_secs(5));
    }
}
