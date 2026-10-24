use super::*;
use yoctui_model::{ClientDaemonLifecycle as Life, ClientDaemonPtySummary};

#[test]
fn workbench_compile_followup_respects_session_build_and_cancellation() {
    use yoctui_model::{Recipe, RecipeMetadata, WorkbenchAction as W};
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    app.workspace.recipes.push(Recipe {
        name: "demo".into(),
        ..Default::default()
    });
    update(
        &mut app,
        Action::Workbench(W::OpenRecipe {
            recipe: "demo".into(),
            root: "/source".into(),
        }),
    );
    app.workbench.pending_compile = Some((
        app.workbench.active.unwrap(),
        "demo".into(),
        Some("/build".into()),
    ));
    assert!(pending_compile_action(&mut app).is_none());
    update(
        &mut app,
        Action::RecipeMetadataLoaded(RecipeMetadata {
            recipe: "demo".into(),
            tasks: Some(vec!["do_compile".into()]),
            ..Default::default()
        }),
    );
    assert_eq!(
        pending_compile_action(&mut app),
        Some(Action::Workbench(W::Compile))
    );
    assert!(
        pending_compile_action(&mut app).is_none(),
        "follow-up runs only once"
    );
    app.workbench.pending_compile = Some((
        app.workbench.active.unwrap(),
        "demo".into(),
        Some("/other-build".into()),
    ));
    assert!(pending_compile_action(&mut app).is_none());
    assert!(app.workbench.pending_compile.is_none());
    app.workbench.pending_compile = Some((
        app.workbench.active.unwrap(),
        "demo".into(),
        Some("/build".into()),
    ));
    update(&mut app, Action::Workbench(W::Sessions));
    assert!(pending_compile_action(&mut app).is_none());
}

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
