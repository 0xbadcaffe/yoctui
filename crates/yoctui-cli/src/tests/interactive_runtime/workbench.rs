use super::*;
use yoctui_model::{ClientDaemonLifecycle as Life, ClientDaemonPtySummary};

#[test]
fn workbench_writer_waits_for_running_and_matches_only_its_new_shell() {
    let mut app = App::new(16, 4096);
    app.daemon.pty_sessions = vec![
        ClientDaemonPtySummary {
            id: 1,
            name: "Workbench".into(),
            lifecycle: Life::Running,
            viewers: 0,
        },
        ClientDaemonPtySummary {
            id: 2,
            name: "QEMU console".into(),
            lifecycle: Life::Running,
            viewers: 0,
        },
        ClientDaemonPtySummary {
            id: 3,
            name: "Workbench".into(),
            lifecycle: Life::Connecting,
            viewers: 0,
        },
    ];
    assert_eq!(ready_terminal(&app, "Workbench", 1), None);
    app.daemon.pty_sessions[2].lifecycle = Life::Running;
    assert_eq!(ready_terminal(&app, "Workbench", 1), Some(2));
    app.daemon.pty_sessions[2].lifecycle = Life::Exited;
    assert_eq!(ready_terminal(&app, "Workbench", 1), None);
}
