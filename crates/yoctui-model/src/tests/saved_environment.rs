use super::*;

fn fixture() -> (App, SavedEnvironmentPlan) {
    let plan = SavedEnvironmentPlan {
        profile: BuildEnvironmentProfile {
            source_dir: "/yocto/source".into(),
            build_dir: "/yocto/build".into(),
            init_script: "/yocto/source/oe-init-build-env".into(),
        },
        target: "historical-image".into(),
        machine: Some("romulus".into()),
        mode: SavedEnvironmentMode::Start,
    };
    let mut app = App::new(32, 4096);
    app.screen = Screen::BuildHistory;
    app.focus = FocusTarget::Workspace;
    app.saved_builds.records = std::sync::Arc::new(vec![SavedBuild {
        id: "past".into(),
        target: plan.target.clone(),
        machine: plan.machine.clone(),
        source: Some("/yocto/source".into()),
        build_dir: Some("/yocto/build".into()),
        outcome: SavedBuildOutcome::Failed,
        saved_unix_ms: 1,
        started_unix_ms: None,
        finished_unix_ms: None,
        logs: Vec::new(),
        tasks: Vec::new(),
        limitations: Vec::new(),
    }]);
    (app, plan)
}

fn act(app: &mut App, action: SavedEnvironmentAction) -> Option<Effect> {
    update(
        app,
        Action::SavedBuild(SavedBuildAction::Environment(action)),
    )
}

#[test]
fn saved_environment_requires_review_and_preserves_live_state() {
    let (mut app, plan) = fixture();
    let live = app.build.clone();
    assert!(matches!(
        act(&mut app, SavedEnvironmentAction::Begin),
        Some(Effect::SavedEnvironment(SavedEnvironmentRequest::Prepare {
            generation: 1,
            ..
        }))
    ));
    assert!(act(&mut app, SavedEnvironmentAction::Confirm).is_none());
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: 1,
            result: Ok(plan.clone()),
        },
    );
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::SavedEnvironmentReview(plan.clone()))
    );
    assert!(matches!(
        act(&mut app, SavedEnvironmentAction::Confirm),
        Some(Effect::SavedEnvironment(SavedEnvironmentRequest::Load {
            generation: 1,
            ..
        }))
    ));
    assert_eq!(app.build, live);
    assert!(app.saved_builds.environment.loading);
    act(
        &mut app,
        SavedEnvironmentAction::Finished {
            generation: 1,
            result: Ok(plan.profile),
        },
    );
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.build, live);
    assert!(!app.saved_builds.environment.loading);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("no saved job was replayed")
    );
}

#[test]
fn saved_environment_cancel_stale_and_error_results_are_safe() {
    let (mut app, plan) = fixture();
    act(&mut app, SavedEnvironmentAction::Begin);
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: 0,
            result: Ok(plan.clone()),
        },
    );
    assert!(app.active_dialog().is_none());
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: 1,
            result: Ok(plan.clone()),
        },
    );
    act(&mut app, SavedEnvironmentAction::Cancel);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(act(&mut app, SavedEnvironmentAction::Confirm).is_none());
    act(&mut app, SavedEnvironmentAction::Begin);
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: 2,
            result: Err("source missing".into()),
        },
    );
    assert_eq!(
        app.saved_builds.environment.error.as_deref(),
        Some("source missing")
    );
    act(&mut app, SavedEnvironmentAction::Begin);
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: 3,
            result: Ok(plan),
        },
    );
    act(&mut app, SavedEnvironmentAction::Confirm);
    act(
        &mut app,
        SavedEnvironmentAction::Finished {
            generation: 2,
            result: Err("stale".into()),
        },
    );
    assert!(app.saved_builds.environment.loading);
    act(
        &mut app,
        SavedEnvironmentAction::Finished {
            generation: 3,
            result: Err("daemon refused".into()),
        },
    );
    assert!(!app.saved_builds.environment.loading);
    assert_eq!(app.screen, Screen::BuildHistory);
    assert_eq!(
        app.saved_builds.environment.error.as_deref(),
        Some("daemon refused")
    );
}

#[test]
fn saved_environment_does_not_replace_an_unrelated_modal() {
    let (mut app, plan) = fixture();
    act(&mut app, SavedEnvironmentAction::Begin);
    open_dialog(&mut app, Dialog::QuitConfirmation);
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: 1,
            result: Ok(plan),
        },
    );
    assert_eq!(app.active_dialog(), Some(&Dialog::QuitConfirmation));
    assert!(act(&mut app, SavedEnvironmentAction::Begin).is_none());
}

#[test]
fn saved_environment_switch_clears_path_bound_inspection_not_preferences_or_archive() {
    let (mut app, _) = fixture();
    app.recipe_sources
        .insert("old".into(), vec!["/old/file.bb".into()]);
    app.layer_browser = Some(LayerBrowser::new("old".into(), "/old/root".into()));
    app.rootfs_request_generation = 12;
    let records = app.saved_builds.records.clone();
    let preferences = app.preferences.clone();
    app.kernel_debug.generation = 12;
    app.kernel_debug.pending = Some(crate::KernelDebugOperation::Inspect);
    app.kernel_debug.instrumentation_preview = Some(crate::KernelInstrumentationPreview {
        draft: crate::KernelInstrumentationDraft {
            config: "/old/.config".into(),
            output: "/old/debug.cfg".into(),
            ..Default::default()
        },
        report: crate::KernelInstrumentationReport::inspect(
            crate::KernelInstrumentationPreset::Kasan,
            "",
        )
        .unwrap(),
        destination_parent: "/old".into(),
        parent_identity: None,
    });
    app.kernel_debug.tools = Some(crate::KernelDebugTools {
        cwd: "/old-build".into(),
        programs: Default::default(),
    });
    clear_saved_environment_views(&mut app);
    assert_eq!(app.kernel_debug.generation, 13);
    assert!(app.kernel_debug.tools.is_none());
    assert!(app.kernel_debug.pending.is_none());
    assert!(app.kernel_debug.qemu_preview.is_none());
    assert!(app.kernel_debug.instrumentation_preview.is_none());
    assert!(app.recipe_sources.is_empty());
    assert!(app.layer_browser.is_none());
    assert_eq!(app.saved_builds.records, records);
    assert_eq!(app.preferences, preferences);
    assert_eq!(app.rootfs_request_generation, 12);
}
