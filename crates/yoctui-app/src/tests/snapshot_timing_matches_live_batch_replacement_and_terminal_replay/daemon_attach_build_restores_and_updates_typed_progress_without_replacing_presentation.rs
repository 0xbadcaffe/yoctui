use super::*;

#[test]
fn daemon_attach_build_restores_and_updates_typed_progress_without_replacing_presentation() {
    use std::collections::HashMap;
    use yoctui_protocol::daemon::{DaemonBuildEvent, DaemonEvent, SequencedEvent};
    use yoctui_protocol::{TaskStatsData, WorkspaceData};

    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.build_events = vec![
        DaemonBuildEvent::Reset {
            targets: vec!["core-image-minimal".into()],
        },
        DaemonBuildEvent::Workspace {
            data: WorkspaceData {
                build_dir: Some("/work/build".into()),
                source_dir: Some("/work/poky".into()),
                variables: HashMap::from([
                    ("MACHINE".into(), "qemux86-64".into()),
                    ("DISTRO".into(), "poky".into()),
                ]),
                variable_provenance: HashMap::new(),
                variable_provenance_chain: HashMap::new(),
                bitbake_version: Some("2.8.1".into()),
                release: Some("5.0.19".into()),
                layers: Vec::new(),
                recipes: Vec::new(),
            },
        },
        DaemonBuildEvent::Started {
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskStarted {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            pid: Some(42),
            worker: Some("worker-1".into()),
            log_path: None,
            stats: Some(TaskStatsData {
                completed: 102,
                total: 4090,
                active: 8,
                failed: 0,
            }),
            started_unix_ms: None,
        },
        DaemonBuildEvent::TaskProgress {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(77),
        },
    ];
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Inspector;
    app.theme = yoctui_model::Theme::MatrixGreen;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    let mut replica = DaemonClientSnapshot::default();
    replica.replace_app(&mut app, snapshot);

    assert_eq!(app.backend, "bridge");
    assert!(matches!(
        app.build_environment,
        yoctui_model::BuildEnvironmentState::Connected(ref profile)
            if profile.source_dir == std::path::Path::new("/work/poky")
                && profile.build_dir == std::path::Path::new("/work/build")
    ));
    assert_eq!(app.build.status, yoctui_model::BuildStatus::Running);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert_eq!((app.build.completed, app.build.total), (102, Some(4090)));
    assert_eq!(
        app.tasks[&yoctui_model::TaskId("busybox:do_compile".into())].progress,
        Some(77)
    );
    assert_eq!(app.workspace.release.as_deref(), Some("5.0.19"));
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.focus, FocusTarget::Inspector);
    assert_eq!(app.theme, yoctui_model::Theme::MatrixGreen);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::QuitConfirmation)
    ));

    replica
        .apply_event_to_app(
            &mut app,
            &SequencedEvent {
                sequence: 1,
                generation: 1,
                event: DaemonEvent::Build(DaemonBuildEvent::TaskProgress {
                    recipe: "busybox".into(),
                    task: "do_compile".into(),
                    progress: Some(88),
                }),
            },
        )
        .unwrap();
    assert_eq!(
        app.tasks[&yoctui_model::TaskId("busybox:do_compile".into())].progress,
        Some(88)
    );
}
