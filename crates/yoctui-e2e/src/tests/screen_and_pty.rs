    #[test]
    fn punctuation_csi_keeps_following_text() {
        assert!(
            super::parse_screen(b"a\x1b[1~after", 20, 2)
                .text()
                .starts_with("aafter")
        );
    }

    #[test]
    fn pty_harness_parses_ansi_screen() {
        let bytes = b"\x1b[2J\x1b[1;1HYoctui\r\nFooter\x1b[0m";
        let screen = parse_screen(bytes, 20, 4);
        assert!(screen.text().contains("Yoctui"));
        assert!(screen.text().contains("Footer"));
    }

    #[cfg(unix)]
    #[test]
    fn pty_harness_executes_child_on_real_pty() {
        let output = run_pty("/bin/sh", &["-c", "printf 'pty-ok\\n'"], b"").expect("pty");
        assert!(String::from_utf8_lossy(&output).contains("pty-ok"));
    }

    #[cfg(unix)]
    #[test]
    fn next_generation_pty_renders_real_output_and_preserves_workbench_ownership() {
        use yoctui_model::{
            ClientDaemonLifecycle, ClientDaemonPtyScreen, ClientDaemonPtySummary, PaneId,
            PtyClientId, PtyCommandIdentity, PtyDimensions, PtySession, PtySessionAction,
            PtySessionId, PtySessionKind, PtySessionSpec, PtyWorkspaceContext, SplitAxis,
            TerminalEmulator,
        };

        let output = run_pty(
            "/bin/sh",
            &[
                "-c",
                "printf '\\033[2J\\033[1;1H\\033[1;3;4;38;2;7;8;9mreal-pty-ready\\033[0m\\r\\nline-two\\r\\nline-three\\r\\n'",
            ],
            b"",
        )
        .expect("real PTY fixture");
        let mut emulator = TerminalEmulator::new(
            PtyDimensions {
                columns: 24,
                rows: 6,
            },
            4,
        )
        .unwrap();
        emulator.process(&output).unwrap();
        let snapshot = emulator.snapshot(0).unwrap();
        assert!(snapshot.plain_text.contains("real-pty-ready"));
        assert!(snapshot.max_scrollback_offset <= 4);

        emulator
            .resize(PtyDimensions {
                columns: 32,
                rows: 8,
            })
            .unwrap();
        let resized = emulator.snapshot(usize::MAX).unwrap();
        assert_eq!(resized.dimensions.columns, 32);
        assert!(resized.scrollback_offset <= resized.max_scrollback_offset);

        let client = PtyClientId([7; 16]);
        let mut lifecycle = PtySession::new(
            PtySessionSpec {
                id: PtySessionId(7),
                name: "acceptance shell".into(),
                kind: PtySessionKind::BuildShell,
                cwd: "/work/poky/build".into(),
                command: PtyCommandIdentity {
                    executable: "/bin/sh".into(),
                    arguments: Vec::new(),
                },
                dimensions: PtyDimensions {
                    columns: 32,
                    rows: 8,
                },
                restartable: true,
                workspace: PtyWorkspaceContext {
                    source_dir: "/work/poky".into(),
                    build_dir: "/work/poky/build".into(),
                    authorized_context_roots: Vec::new(),
                    owner_identity: "pty-ui-acceptance".into(),
                },
            },
            77,
        )
        .unwrap();
        lifecycle.apply(PtySessionAction::MarkRunning).unwrap();
        lifecycle.apply(PtySessionAction::Attach(client)).unwrap();
        lifecycle
            .apply(PtySessionAction::TakeControl {
                client,
                expected_epoch: 0,
            })
            .unwrap();
        lifecycle.apply(PtySessionAction::Detach(client)).unwrap();
        assert!(lifecycle.attached_clients.is_empty());
        assert!(lifecycle.writer.is_none());

        let mut app = yoctui_model::App::new(32, 8192);
        app.screen = AppScreen::TerminalSessions;
        app.focus = FocusTarget::Workspace;
        let second = app
            .pane_layout
            .split(PaneId(1), SplitAxis::Horizontal)
            .unwrap();
        app.daemon.pty_sessions = vec![
            ClientDaemonPtySummary {
                id: 1,
                name: "left".into(),
                lifecycle: ClientDaemonLifecycle::Running,
                viewers: 1,
            },
            ClientDaemonPtySummary {
                id: 7,
                name: "real shell".into(),
                lifecycle: ClientDaemonLifecycle::Running,
                viewers: 1,
            },
        ];
        app.daemon.pty_screens = vec![
            ClientDaemonPtyScreen {
                session_id: 1,
                columns: 24,
                rows_count: 6,
                cursor_column: 0,
                cursor_row: 0,
                cursor_hidden: false,
                scrollback_offset: 0,
                rows: vec!["left-session-only".into()],
                cells: Vec::new(),
                scrollback_lines: 0,
                dropped_line_feeds_lower_bound: 0,
            },
            ClientDaemonPtyScreen {
                session_id: 7,
                columns: resized.dimensions.columns,
                rows_count: resized.dimensions.rows,
                cursor_column: resized.cursor.1,
                cursor_row: resized.cursor.0,
                cursor_hidden: resized.modes.cursor_hidden,
                scrollback_offset: resized.scrollback_offset as u32,
                rows: resized.plain_text.lines().map(str::to_owned).collect(),
                cells: resized
                    .cells
                    .iter()
                    .map(|cell| yoctui_model::ClientDaemonTerminalCell {
                        contents: cell.contents.clone(),
                        foreground: match cell.foreground {
                            yoctui_model::TerminalColor::Default => {
                                yoctui_model::ClientDaemonTerminalColor::Default
                            }
                            yoctui_model::TerminalColor::Indexed(index) => {
                                yoctui_model::ClientDaemonTerminalColor::Indexed(index)
                            }
                            yoctui_model::TerminalColor::Rgb(red, green, blue) => {
                                yoctui_model::ClientDaemonTerminalColor::Rgb(red, green, blue)
                            }
                        },
                        background: match cell.background {
                            yoctui_model::TerminalColor::Default => {
                                yoctui_model::ClientDaemonTerminalColor::Default
                            }
                            yoctui_model::TerminalColor::Indexed(index) => {
                                yoctui_model::ClientDaemonTerminalColor::Indexed(index)
                            }
                            yoctui_model::TerminalColor::Rgb(red, green, blue) => {
                                yoctui_model::ClientDaemonTerminalColor::Rgb(red, green, blue)
                            }
                        },
                        bold: cell.bold,
                        dim: cell.dim,
                        italic: cell.italic,
                        underline: cell.underline,
                        inverse: cell.inverse,
                        wide: cell.wide,
                        wide_continuation: cell.wide_continuation,
                    })
                    .collect(),
                scrollback_lines: resized.max_scrollback_offset as u32,
                dropped_line_feeds_lower_bound: resized.dropped_line_feeds_lower_bound,
            },
        ];
        let _ = yoctui_model::update(
            &mut app,
            Action::SelectPtyPane {
                pane: second,
                index: 1,
            },
        );
        assert_eq!(app.pane_layout.focused, second);
        assert_eq!(app.pty_selection, 1);

        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal
            .draw(|frame| yoctui_ui::render_at(frame, &app, UNIX_EPOCH))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let row_text = |row| {
            (0..120)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        };
        let output_row = (1..29)
            .find(|row| row_text(*row).contains("real-pty-ready"))
            .expect("real PTY output appears inside a workbench pane");
        let output_column = row_text(output_row).find("real-pty-ready").unwrap();
        let styled = &buffer[(output_column as u16, output_row)];
        assert_eq!(styled.fg, ratatui::style::Color::Rgb(7, 8, 9));
        assert!(styled.modifier.contains(ratatui::style::Modifier::BOLD));
        assert!(styled.modifier.contains(ratatui::style::Modifier::ITALIC));
        assert!(
            styled
                .modifier
                .contains(ratatui::style::Modifier::UNDERLINED)
        );
        assert!(
            output_column >= 60,
            "output must be in the selected right pane"
        );
        assert!((0..2).any(|row| row_text(row).to_ascii_lowercase().contains("yoctui")));
        assert!((28..30).any(|row| row_text(row).contains("F1 Help")));
        assert!(row_text(output_row).contains("real-pty-ready"));
        assert!((1..29).any(|row| row_text(row).contains("left-session-only")));

        let retained_focus = app.focus;
        let mut prefix = PrefixState::new(Duration::from_secs(1));
        assert_eq!(
            prefix.feed(Input::CtrlB, Instant::now()),
            PrefixEvent::Awaiting
        );
        assert_eq!(
            prefix.feed(Input::Char('d'), Instant::now()),
            PrefixEvent::Command(PrefixCommand::Detach)
        );
        assert_eq!(app.focus, retained_focus);
    }
