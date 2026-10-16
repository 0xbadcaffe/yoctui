use super::*;
use std::time::{Duration, Instant};

fn terminal() -> App {
    let mut app = App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "QEMU console".into(),
            viewers: 1,
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
        });
    app
}

#[test]
fn terminal_echo_has_a_separate_budget_even_during_saturated_builds() {
    let mut app = terminal();
    app.build.status = BuildStatus::Running;
    app.host_telemetry.cpu_utilization_percent = Some(99);
    assert_eq!(foreground_frame_interval(&app), Duration::from_millis(16));
    assert_eq!(
        terminal_event_wait(&app, Duration::from_millis(100)),
        Duration::from_millis(8)
    );
    assert_eq!(
        terminal_event_wait(&app, Duration::from_millis(4)),
        Duration::from_millis(4)
    );
    assert_eq!(animation_interval(&app), SATURATED_FRAME_INTERVAL);
    app.screen = Screen::Dashboard;
    assert_eq!(foreground_frame_interval(&app), SATURATED_FRAME_INTERVAL);
    assert_eq!(
        terminal_event_wait(&app, Duration::from_millis(100)),
        Duration::from_millis(100)
    );
}

#[test]
fn terminal_output_renders_after_the_key_frame_without_waiting_a_dashboard_tick() {
    let app = terminal();
    let now = Instant::now();
    let interval = foreground_frame_interval(&app);
    let mut scheduler = RenderScheduler::default();
    assert!(scheduler.take_frame_at(now, interval));
    scheduler.invalidate(RenderCause::State); // The remote echo arrives after the input redraw.
    assert!(!scheduler.take_frame_at(now + Duration::from_millis(8), interval));
    assert!(scheduler.take_frame_at(now + Duration::from_millis(16), interval));
    assert!(!scheduler.take_frame_at(now + Duration::from_millis(32), interval));
}

#[test]
fn terminal_browser_overlays_scrollback_and_exited_sessions_keep_the_idle_budget() {
    let mut app = terminal();
    app.terminal.browser_open = true;
    assert!(!has_visible_live_terminal(&app));
    app.terminal.browser_open = false;
    app.onboarding.open = true;
    assert!(!has_visible_live_terminal(&app));
    app.onboarding.open = false;
    app.command_palette_open = true;
    assert!(!has_visible_live_terminal(&app));
    app.command_palette_open = false;
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Copy;
    assert!(!has_visible_live_terminal(&app));
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Live;
    app.daemon.pty_sessions[0].lifecycle = yoctui_model::ClientDaemonLifecycle::Exited;
    assert_eq!(foreground_frame_interval(&app), ORDINARY_FRAME_INTERVAL);
}
