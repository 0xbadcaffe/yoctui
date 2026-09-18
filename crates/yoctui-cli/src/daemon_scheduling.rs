//! Daemon scheduling.
use super::*;

#[cfg(unix)]
pub(crate) const MAX_DAEMON_CLIENT_EVENTS_PER_TICK: usize = 32;

#[cfg(unix)]
const _: () = assert!(MAX_DAEMON_CLIENT_EVENTS_PER_TICK < client_runtime::MAX_EVENTS_PER_POLL);

#[cfg(unix)]
pub(crate) fn daemon_replay_is_bounded(event_count: usize) -> bool {
    event_count <= MAX_DAEMON_CLIENT_EVENTS_PER_TICK
}

pub(crate) const DAEMON_IDLE_WAIT: Duration = Duration::from_millis(100);

pub(crate) const DAEMON_ACTIVE_WAIT: Duration = Duration::from_millis(35);

pub(crate) fn daemon_has_active_work(snapshot: &yoctui_protocol::daemon::DaemonSnapshot) -> bool {
    snapshot.jobs.iter().any(|job| {
        matches!(
            job.lifecycle,
            yoctui_protocol::daemon::LifecycleState::Connecting
                | yoctui_protocol::daemon::LifecycleState::Running
                | yoctui_protocol::daemon::LifecycleState::Stopping
        )
    }) || snapshot.pty_sessions.iter().any(|session| {
        matches!(
            session.lifecycle,
            yoctui_protocol::daemon::LifecycleState::Connecting
                | yoctui_protocol::daemon::LifecycleState::Running
                | yoctui_protocol::daemon::LifecycleState::Stopping
        )
    })
}

pub(crate) fn daemon_service_wait(has_active_work: bool) -> Duration {
    if has_active_work {
        DAEMON_ACTIVE_WAIT
    } else {
        DAEMON_IDLE_WAIT
    }
}
