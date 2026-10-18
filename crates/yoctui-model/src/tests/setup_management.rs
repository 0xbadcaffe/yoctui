use super::*;
fn profile() -> BuildEnvironmentProfile {
    BuildEnvironmentProfile {
        source_dir: "/tmp/setups/demo/layers/core".into(),
        build_dir: "/tmp/setups/demo/build".into(),
        init_script: "/tmp/setups/demo/build/init-build-env".into(),
    }
}
fn dialog(operation: SetupOperation) -> SetupOperationDialog {
    SetupOperationDialog {
        operation,
        profile: profile(),
        tool: "/tools/bitbake-setup".into(),
        fragment: "machine/qemux86-64".into(),
        editor: None,
        config_policy: 0,
        backup_conflicts: false,
        offline: false,
        preparing: None,
        error: None,
    }
}
fn send(app: &mut App, a: SetupManageAction) -> Option<Effect> {
    update(app, Action::BitBakeSetup(BitBakeSetupAction::Manage(a)))
}

#[test]
fn setup_management_argv_is_exact_and_update_defaults_are_conservative() {
    let mut d = dialog(SetupOperation::Update);
    let r = d.launch().unwrap();
    assert_eq!(r.program, std::path::PathBuf::from("/tools/bitbake-setup"));
    assert!(
        r.arguments
            .windows(2)
            .any(|w| w == ["--update-bb-conf", "prompt"])
    );
    assert!(
        r.arguments
            .windows(2)
            .any(|w| w == ["--rebase-conflicts-strategy", "abort"])
    );
    assert!(
        r.arguments
            .windows(2)
            .any(|w| w == ["--setup-dir", "/tmp/setups/demo"])
    );
    d.offline = true;
    d.backup_conflicts = true;
    d.config_policy = 1;
    let r = d.launch().unwrap();
    assert!(r.arguments.contains(&"--no-network".into()));
    assert!(
        r.arguments
            .windows(2)
            .any(|w| w == ["--update-bb-conf", "no"])
    );
    let r = dialog(SetupOperation::Enable).launch().unwrap();
    assert!(r.arguments.contains(&"machine/qemux86-64".into()));
    assert!(
        r.arguments
            .windows(2)
            .any(|w| w == ["--confpath", "/tmp/setups/demo/build/conf/toolcfg.conf"])
    );
    for n in [
        "machine/",
        "../bad",
        "machine/../../etc",
        "machine/...",
        "--help",
        "machine/x;false",
    ] {
        assert!(!valid_setup_fragment(n), "{n}");
    }
}

#[test]
fn setup_management_review_does_not_mutate_and_busy_daemon_is_rejected() {
    let mut app = App::new(20, 2000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.build_environment = BuildEnvironmentState::Configured(profile());
    send(&mut app, SetupManageAction::Update);
    assert!(
        matches!(app.active_dialog(),Some(Dialog::BitBakeSetupOperation(d)) if d.config_policy()=="prompt" && !d.backup_conflicts)
    );
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 1,
        name: "shell".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    assert!(send(&mut app, SetupManageAction::Confirm).is_none());
    assert!(
        matches!(app.active_dialog(),Some(Dialog::BitBakeSetupOperation(d)) if d.error.as_ref().is_some_and(|e|e.contains("close live")))
    );
    app.daemon.pty_sessions.clear();
    assert!(matches!(
        send(&mut app, SetupManageAction::Confirm),
        Some(Effect::BitBakeSetup(BitBakeSetupRequest::Manage(
            SetupManageRequest::Prepare { .. }
        )))
    ));
    send(&mut app, SetupManageAction::Cancel);
    assert!(
        send(
            &mut app,
            SetupManageAction::Prepared {
                generation: 1,
                result: dialog(SetupOperation::Update).launch()
            }
        )
        .is_none()
    );
}

#[test]
fn setup_management_rejects_stale_results_and_unwinds_detail_then_list() {
    let mut app = App::new(20, 2000);
    let s = &mut app.bitbake_setup.management;
    s.visible = true;
    s.generation = 2;
    s.output = Some(vec!["detail".into()]);
    s.loading = true;
    send(
        &mut app,
        SetupManageAction::Read {
            generation: 1,
            result: Ok("stale".into()),
        },
    );
    assert!(app.bitbake_setup.management.loading);
    send(&mut app, SetupManageAction::Cancel);
    assert!(app.bitbake_setup.management.visible);
    assert!(app.bitbake_setup.management.output.is_none());
    send(&mut app, SetupManageAction::Cancel);
    assert!(!app.bitbake_setup.management.visible);
}

#[test]
fn setup_management_refresh_keeps_status_semantics_and_clears_stale_output() {
    let mut app = App::new(20, 2000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.bitbake_setup.management.visible = true;
    app.bitbake_setup.management.profile = Some(profile());
    app.bitbake_setup.management.output = Some(vec!["old status".into()]);
    app.bitbake_setup.management.last_read = Some(dialog(SetupOperation::Status));
    let Some(Effect::BitBakeSetup(BitBakeSetupRequest::Manage(SetupManageRequest::Read {
        generation,
        dialog: d,
    }))) = send(&mut app, SetupManageAction::Refresh)
    else {
        panic!("status refresh must not inspect/switch to fragments")
    };
    assert_eq!(d.operation, SetupOperation::Status);
    assert!(
        app.bitbake_setup
            .management
            .output
            .as_ref()
            .unwrap()
            .is_empty()
    );
    send(
        &mut app,
        SetupManageAction::Read {
            generation,
            result: Ok(String::new()),
        },
    );
    assert!(app.bitbake_setup.management.output.as_ref().unwrap()[0].contains("no text"));
}
