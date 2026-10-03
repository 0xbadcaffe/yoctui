use super::*;
use yoctui_model::{ClientReplicaStatus, DaemonModelInstanceId, update_with_workspace_authority};

#[test]
fn native_navigation_renders_optional_screens_without_faking_capability_inspection() {
    for (screen, label, pending) in [
        (Screen::Sdk, "SDK", "SDK artifacts are not loaded"),
        (Screen::Testing, "Testing", "capability pending"),
        (Screen::Security, "Security", "not inspected"),
        (Screen::Qa, "QA", "not inspected"),
    ] {
        let mut app = App::new(16, 4096);
        app.daemon.status = ClientReplicaStatus::Current;
        app.daemon.instance_id = Some(DaemonModelInstanceId([42; 16]));
        assert!(update_with_workspace_authority(&mut app, Action::Open(screen)).is_none());
        assert_eq!(app.screen, screen);
        app.focus = FocusTarget::Workspace;
        for (width, height) in [(160, 50), (120, 30), (80, 24)] {
            let output = rendered_text(&app, width, height);
            assert!(
                output.contains(label),
                "{screen:?} {width}x{height}: {output}"
            );
            if width == 160 {
                assert!(
                    output.contains(pending),
                    "{screen:?} {width}x{height}: {output}"
                );
            }
            assert!(!output.contains("Build Overview"), "{output}");
            assert!(
                !output.contains("Environment probing is daemon-owned"),
                "{output}"
            );
        }
        assert!(app.daemon.jobs.is_empty());
        assert!(app.daemon.pty_sessions.is_empty());
    }
}
