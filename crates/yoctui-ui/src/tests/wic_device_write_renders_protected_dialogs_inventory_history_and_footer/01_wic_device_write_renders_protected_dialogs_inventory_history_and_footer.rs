use super::*;

#[test]
fn wic_device_write_renders_protected_dialogs_inventory_history_and_footer() {
    let mut app = wic_workspace_app();
    let Some(yoctui_model::Effect::GetWicDevices(request)) =
        yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicDeviceWrite)
    else {
        panic!("expected protected device discovery");
    };
    let loading = rendered_text(&app, 80, 24);
    assert!(
        loading.contains("Select protected Wic write device"),
        "{loading}"
    );
    assert!(loading.contains("Discovering removable"), "{loading}");

    let mut empty = app.clone();
    empty.wic_devices = WicDeviceInventoryState::Available {
        request: request.clone(),
        devices: Vec::new(),
    };
    let rendered = rendered_text(&empty, 100, 30);
    assert!(
        rendered.contains("No eligible removable whole devices"),
        "{rendered}"
    );

    let mut failed = app.clone();
    failed.wic_devices = WicDeviceInventoryState::Failed {
        request: request.clone(),
        message: "lsblk permission denied".into(),
    };
    let rendered = rendered_text(&failed, 100, 30);
    assert!(
        rendered.contains("Device discovery failed: lsblk permission denied"),
        "{rendered}"
    );

    let device = WicDevice {
        identity: yoctui_model::WicDeviceIdentity {
            path: "/dev/sdz".into(),
            major_minor: "8:240".into(),
            size_bytes: 16_384,
            model: Some("Protected USB".into()),
            serial: Some("SERIAL-123".into()),
            transport: Some("usb".into()),
        },
        removable: true,
        writable: true,
        read_only: false,
        descendant_mounts: Vec::new(),
        unavailable_reason: None,
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::WicDeviceInventoryLoaded {
            request: request.clone(),
            devices: vec![device],
            limitations: vec!["one udev property was unavailable".into()],
        },
    );
    for (theme, color_enabled) in [
        (Theme::DarkPro, true),
        (Theme::WhiteClassic, true),
        (Theme::MatrixGreen, true),
        (Theme::HighContrast, true),
        (Theme::Monochrome, false),
    ] {
        app.theme = theme;
        app.color_enabled = color_enabled;
        for (width, height) in [(80, 24), (110, 30), (160, 40)] {
            let rendered = rendered_text(&app, width, height);
            assert!(rendered.contains("/dev/sdz"), "{rendered}");
            assert!(rendered.contains("8:240"), "{rendered}");
            assert!(rendered.contains("Protected USB"), "{rendered}");
            assert!(rendered.contains("SERIAL-123"), "{rendered}");
            assert!(rendered.contains("transport=usb"), "{rendered}");
            assert!(rendered.contains("removable=true"), "{rendered}");
            assert!(
                rendered.contains("one udev property was unavailable"),
                "{rendered}"
            );
        }
    }

    let _ = yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicDeviceSelection);
    let phrase = rendered_text(&app, 80, 24);
    assert!(
        phrase.contains("Required phrase: WRITE /dev/sdz"),
        "{phrase}"
    );
    assert!(phrase.contains("phrase alone does not write"), "{phrase}");
    for character in "WRONG".chars() {
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::AppendWicWritePhrase(character),
        );
    }
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicDeviceWrite);
    assert!(rendered_text(&app, 100, 30).contains("Validation:"));
    for _ in 0.."WRONG".len() {
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BackspaceWicWritePhrase);
    }
    for character in "WRITE /dev/sdz".chars() {
        let _ = yoctui_model::update(
            &mut app,
            yoctui_model::Action::AppendWicWritePhrase(character),
        );
    }
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicDeviceWrite);
    for (width, height) in [(80, 24), (110, 30), (160, 40)] {
        let preview = rendered_text(&app, width, height);
        assert!(preview.contains("DESTRUCTIVE OPERATION"), "{preview}");
        assert!(preview.contains("Exact argument vector"), "{preview}");
        assert!(preview.contains("[1]=write"), "{preview}");
        assert!(preview.contains("[2]=/deploy/qemux86-64"), "{preview}");
        assert!(preview.contains("[3]=/dev/sdz"), "{preview}");
    }

    let Some(yoctui_model::Effect::StartWicSession { id, .. }) =
        yoctui_model::update(&mut app, yoctui_model::Action::ConfirmWicDeviceWrite)
    else {
        panic!("expected managed Wic device write");
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::WicSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = yoctui_model::update(&mut app, yoctui_model::Action::WicSessionRunning { id });
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::AppendWicSessionOutput {
            id,
            stream: yoctui_model::WicOutputStream::Stderr,
            line: "write progress".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    app.host_telemetry = yoctui_model::HostTelemetry {
        cpu_utilization_percent: Some(42),
        disk_available_bytes: Some(1_048_576),
        ..yoctui_model::HostTelemetry::default()
    };
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::BeginActiveWicSessionCancellation,
    );
    let warning = rendered_text(&app, 80, 24);
    assert!(
        warning.contains("Confirm Wic device-write cancellation"),
        "{warning}"
    );
    assert!(warning.contains("incomplete"), "{warning}");
    assert!(warning.contains("unusable"), "{warning}");
    app.dialogs.clear();
    let inspector = wic_inspector_text(&app);
    assert!(inspector.contains("write\nimage=/deploy/qemux86-64"));
    assert!(inspector.contains("device=/dev/sdz major:minor=8:240"));
    assert!(inspector.contains("Host telemetry: CPU 42%"));
    assert!(inspector.contains("write progress [truncated]"));
    assert!(inspector.contains("Dropped output: 0 entries"));

    let footer = footer_shortcuts(&app);
    assert!(footer.contains("D write device"), "{footer}");
    assert!(
        responsive_footer_shortcuts(&app, 80).contains("D write"),
        "{}",
        responsive_footer_shortcuts(&app, 80)
    );
}

#[test]
fn wic_workspace_handles_long_source_themes_and_exact_footer_hints() {
    let mut app = wic_workspace_app();
    if let WicCapability::Available { kickstarts, .. } = &mut app.wic_capability {
        kickstarts[0].source = (0..200)
            .map(|index| format!("part /p{index} --source=rootfs # row {index}"))
            .collect::<Vec<_>>()
            .join("\n");
    }
    for theme in [
        Theme::DarkPro,
        Theme::WhiteClassic,
        Theme::MatrixGreen,
        Theme::HighContrast,
        Theme::Monochrome,
    ] {
        app.theme = theme;
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::BeginSelectedWicCreate);
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::PreviewWicCreate);
        let rendered = rendered_text(&app, 80, 24);
        assert!(
            rendered.contains("Confirm managed Wic creation"),
            "{rendered}"
        );
        let _ = yoctui_model::update(&mut app, yoctui_model::Action::CancelWicCreatePreview);
    }
    let footer = footer_shortcuts(&app);
    assert_eq!(app.focus, FocusTarget::Workspace);
    for expected in [
        "Q QEMU",
        "W create Wic",
        "D write device",
        "x cancel",
        "[/] output",
        "O open output",
        "w Wic",
    ] {
        assert!(footer.contains(expected), "{footer}");
    }
    app.theme = Theme::DarkPro;
    let preview = source_preview(
        "part / --source=rootfs # root partition",
        "directdisk.wks",
        &app,
    );
    assert_ne!(preview.lines[0].spans[0].style, Style::default());
}

#[test]
fn dashboard_footer_documents_keyboard_prefix_layer() {
    let app = App::new(32, 4096);
    let footer = footer_shortcuts(&app);
    assert!(footer.contains("Ctrl+B prefix"), "{footer}");
}

#[test]
fn pane_split_renders_daemon_sessions_with_focus_and_narrow_safety() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "build shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    let rendered = rendered_text(&app, 80, 24);
    assert!(rendered.contains("build shell"), "{rendered}");
    assert!(rendered.contains("1 viewer(s)"), "{rendered}");
}

#[test]
fn ux_terminal_adapter_renders_typed_cells_cursor_styles_and_accessible_color() {
    let mut cells = vec![yoctui_model::ClientDaemonTerminalCell::default(); 8];
    cells[0] = yoctui_model::ClientDaemonTerminalCell {
        contents: "A".into(),
        foreground: yoctui_model::ClientDaemonTerminalColor::Rgb(7, 8, 9),
        background: yoctui_model::ClientDaemonTerminalColor::Indexed(17),
        bold: true,
        italic: true,
        underline: true,
        ..yoctui_model::ClientDaemonTerminalCell::default()
    };
    cells[1] = yoctui_model::ClientDaemonTerminalCell {
        contents: "界".into(),
        wide: true,
        ..yoctui_model::ClientDaemonTerminalCell::default()
    };
    cells[2].wide_continuation = true;
    let screen = yoctui_model::ClientDaemonPtyScreen {
        session_id: 9,
        columns: 4,
        rows_count: 2,
        cursor_column: 3,
        cursor_row: 0,
        cursor_hidden: false,
        scrollback_offset: 1,
        rows: vec!["A界".into(), String::new()],
        cells,
        scrollback_lines: 3,
        dropped_line_feeds_lower_bound: 0,
    };
    let mut app = App::new(8, 128);
    let mut terminal = Terminal::new(TestBackend::new(4, 2)).unwrap();
    terminal
        .draw(|frame| {
            let area = frame.area();
            assert!(render_terminal_replica_content(
                frame, &app, &screen, area, 0, 0,
            ));
        })
        .unwrap();
    let buffer = terminal.backend().buffer();
    assert_eq!(buffer[(0, 0)].symbol(), "A");
    assert_eq!(buffer[(0, 0)].fg, Color::Rgb(7, 8, 9));
    assert_eq!(buffer[(0, 0)].bg, Color::Indexed(17));
    assert!(buffer[(0, 0)].modifier.contains(Modifier::BOLD));
    assert!(buffer[(0, 0)].modifier.contains(Modifier::ITALIC));
    assert!(buffer[(0, 0)].modifier.contains(Modifier::UNDERLINED));
    assert_eq!(buffer[(1, 0)].symbol(), "界");
    assert_eq!(buffer[(3, 1)].symbol(), "█");

    app.color_enabled = false;
    terminal
        .draw(|frame| {
            let area = frame.area();
            assert!(render_terminal_replica_content(
                frame, &app, &screen, area, 0, 0,
            ));
        })
        .unwrap();
    let accessible = terminal.backend().buffer();
    assert_eq!(accessible[(0, 0)].fg, Color::Reset);
    assert_eq!(accessible[(0, 0)].bg, Color::Reset);
    assert!(accessible[(0, 0)].modifier.contains(Modifier::BOLD));
}

#[test]
fn image_console_qemu_and_ssh_panes_use_tui_term_typed_cells_not_plain_fallback() {
    for kind in [
        yoctui_model::ClientDaemonPtyKind::QemuConsole,
        yoctui_model::ClientDaemonPtyKind::SshConsole,
    ] {
        let mut app = concept_terminal_sessions_app();
        app.terminal.query.clear();
        app.daemon.pty_details[0].kind = kind;
        app.daemon.pty_sessions[0].name = format!("{kind:?}");
        let screen = &mut app.daemon.pty_screens[0];
        screen.rows = vec!["WRONG_PLAIN_FALLBACK".into()];
        screen.cells = vec![
            yoctui_model::ClientDaemonTerminalCell::default();
            usize::from(screen.columns) * usize::from(screen.rows_count)
        ];
        for (index, character) in "typed-console-login:".chars().enumerate() {
            screen.cells[index].contents = character.to_string();
            screen.cells[index].bold = true;
        }
        for (width, height) in [(160, 50), (100, 30), (80, 24)] {
            let text = rendered_text(&app, width, height);
            assert!(text.contains("typed-console-login:"), "{kind:?}: {text}");
            assert!(!text.contains("WRONG_PLAIN_FALLBACK"), "{text}");
        }
    }
}

#[test]
fn mouse_input_footer_keeps_keyboard_route_visible() {
    let app = App::new(16, 4096);
    let footer = footer_shortcuts(&app);
    assert!(footer.contains("no other actionable panes"), "{footer}");
    assert!(footer.contains("Ctrl+B prefix"), "{footer}");
}

#[test]
fn mouse_runtime_terminal_workspace_keeps_pane_labels_visible() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 3,
            name: "devshell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 2,
        });
    let rendered = rendered_text(&app, 120, 30);
    assert!(rendered.contains("devshell"), "{rendered}");
    assert!(rendered.contains("2 viewer(s)"), "{rendered}");
}

#[test]
fn mouse_split_resizes_client_local_layout_and_keeps_keyboard_path() {
    let mut app = App::new(16, 4096);
    let root = app.pane_layout.focused;
    app.pane_layout
        .split(root, yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    let focused = app.pane_layout.focused;
    let _ = update(
        &mut app,
        Action::ResizeFocusedPane {
            delta_per_mille: 25,
        },
    );
    assert!(app.pane_layout.contains(focused));
    assert!(app.pane_layout.validate().is_ok());
    assert!(matches!(
        app.pane_layout.root,
        yoctui_model::PaneNode::Split { .. }
    ));
}

#[test]
fn keyboard_mouse_parity_keeps_keyboard_focus_route_visible() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Tasks;
    let before = app.focus;
    let _ = yoctui_model::update(
        &mut app,
        yoctui_model::Action::CycleFocus { backwards: false },
    );
    assert_ne!(app.focus, before);
    let footer = responsive_footer_shortcuts(&app, 160);
    assert!(footer.contains("Focus Workspace"), "{footer}");
    assert!(footer.contains("Tab Navigator"), "{footer}");
    assert!(!footer.contains("Inspector"), "{footer}");
}
