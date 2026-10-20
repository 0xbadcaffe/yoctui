use yoctui_model::*;

fn preparing(screen: Screen) -> App {
    let mut app = App::new(16, 4096);
    app.screen = screen;
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.begin_platform_menuconfig(screen, "kernel menuconfig".into());
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 1,
        name: "kernel menuconfig".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 1,
        kind: ClientDaemonPtyKind::Menuconfig,
        cwd: "/work/build".into(),
        columns: 120,
        rows: 40,
        writer: None,
        writer_epoch: 0,
        exit_code: None,
        restartable: true,
    });
    app.daemon.pty_screens.push(ClientDaemonPtyScreen {
        session_id: 1,
        columns: 120,
        rows_count: 40,
        cursor_column: 0,
        cursor_row: 0,
        cursor_hidden: false,
        application_cursor: false,
        scrollback_offset: 0,
        rows: vec!["Preparing menuconfig with BitBake…".into()],
        cells: Vec::new(),
        scrollback_lines: 0,
        dropped_line_feeds_lower_bound: 0,
    });
    app.reconcile_platform_menuconfigs();
    app
}

#[test]
fn preparing_menuconfig_does_not_replace_the_normal_kernel_workspace() {
    let app = preparing(Screen::Kernel);
    assert!(app.platform_menuconfig_running());
    assert!(app.platform_menuconfig_preparing());
    assert!(app.platform_menuconfig_waiting());
    assert!(!app.platform_menuconfig_visible());
    assert_eq!(app.kernel.menuconfig_terminal.session_id, Some(1));
}

#[test]
fn screens_remain_navigable_while_menuconfig_prepares() {
    let mut app = preparing(Screen::Kernel);
    for screen in [
        Screen::Dashboard,
        Screen::Tasks,
        Screen::Layers,
        Screen::Images,
    ] {
        let _ = update(&mut app, Action::Open(screen));
        assert_eq!(app.screen, screen);
        assert!(!app.platform_menuconfig_visible());
        assert!(app.platform_menuconfig_waiting());
        assert_eq!(app.kernel.menuconfig_terminal.session_id, Some(1));
    }
}

#[test]
fn menuconfig_becoming_ready_does_not_steal_another_screens_focus() {
    let mut app = preparing(Screen::Kernel);
    let _ = update(&mut app, Action::Open(Screen::Tasks));
    app.focus = FocusTarget::Navigator;
    app.daemon.pty_screens[0].application_cursor = true;
    app.reconcile_platform_menuconfigs();
    assert_eq!(app.screen, Screen::Tasks);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert!(!app.platform_menuconfig_visible());
    assert!(!app.platform_menuconfig_waiting());
    app.screen = Screen::Kernel;
    assert!(app.platform_menuconfig_visible());
}

#[test]
fn ncurses_readiness_requires_both_cursor_mode_and_nonempty_content() {
    let mut app = preparing(Screen::Kernel);
    app.daemon.pty_screens[0].application_cursor = true;
    app.daemon.pty_screens[0].rows = vec![" ".into()];
    assert!(!app.platform_menuconfig_visible());
    app.daemon.pty_screens[0].rows = vec!["Linux Kernel Configuration".into()];
    assert!(app.platform_menuconfig_visible());
    assert!(!app.platform_menuconfig_preparing());
}

#[test]
fn auto_open_can_be_disabled_without_stopping_the_preparing_session() {
    let mut app = preparing(Screen::Kernel);
    app.toggle_platform_menuconfig_foreground();
    app.daemon.pty_screens[0].application_cursor = true;
    assert!(!app.platform_menuconfig_visible());
    assert!(app.platform_menuconfig_running());
    app.toggle_platform_menuconfig_foreground();
    assert!(app.platform_menuconfig_visible());
}
