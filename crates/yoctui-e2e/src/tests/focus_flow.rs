    #[test]
    fn next_generation_focus_flow_covers_shell_modals_terminal_and_narrow_switcher() {
        let dispatch = |app: &mut yoctui_model::App, input| {
            let action = key_action(input).expect("focus key has a typed global action");
            let _ = yoctui_model::update(app, action);
        };
        let rendered = |app: &yoctui_model::App, width, height| {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| yoctui_ui::render_at(frame, app, UNIX_EPOCH))
                .unwrap();
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
        };

        for (width, height) in [(160, 50), (100, 30), (80, 24)] {
            let mut app = yoctui_model::App::new(32, 8192);
            app.screen = AppScreen::Tasks;
            app.focus = FocusTarget::Navigator;
            app.navigator_selection = 6;
            app.task_progress_scroll = 3;

            for (input, expected) in [
                (Input::Tab, FocusTarget::Workspace),
                (Input::Tab, FocusTarget::Navigator),
                (Input::BackTab, FocusTarget::Workspace),
                (Input::BackTab, FocusTarget::Navigator),
            ] {
                dispatch(&mut app, input);
                assert_eq!(app.focus, expected, "{width}x{height} {input:?}");
                let output = rendered(&app, width, height);
                match (width, expected) {
                    (80..=99, FocusTarget::Navigator) => {
                        assert!(output.contains("Panes: [Navigator]"), "{output}")
                    }
                    (80..=99, FocusTarget::Workspace) => {
                        assert!(output.contains("[Workspace]"), "{output}")
                    }
                    (_, FocusTarget::Navigator) => {
                        assert!(output.contains("Navigator"), "{output}")
                    }
                    (_, FocusTarget::Workspace) => {
                        assert!(output.contains("Tasks:"), "{output}")
                    }
                    _ => unreachable!(),
                }
            }
            assert_eq!(app.navigator_selection, 6);
            assert_eq!(app.task_progress_scroll, 3);
        }

        let mut modal = yoctui_model::App::new(32, 8192);
        modal.screen = AppScreen::Tasks;
        modal.focus = FocusTarget::Workspace;
        let _ = yoctui_model::update(&mut modal, Action::OpenThemePicker);
        assert_eq!(modal.focus, FocusTarget::Dialog);
        assert_eq!(modal.focus_return, Some(FocusTarget::Workspace));
        dispatch(&mut modal, Input::Tab);
        dispatch(&mut modal, Input::BackTab);
        assert_eq!(modal.focus, FocusTarget::Dialog);
        assert_eq!(focus_action(FocusTarget::Dialog, Input::Tab), None);
        let _ = yoctui_model::update(&mut modal, Action::CloseThemePicker);
        assert_eq!(modal.focus, FocusTarget::Workspace);
        assert_eq!(modal.focus_return, None);

        modal.focus = FocusTarget::Navigator;
        let _ = yoctui_model::update(&mut modal, Action::OpenCommandPalette);
        assert_eq!(modal.focus, FocusTarget::CommandPalette);
        assert_eq!(modal.focus_return, Some(FocusTarget::Navigator));
        dispatch(&mut modal, Input::Tab);
        dispatch(&mut modal, Input::BackTab);
        assert_eq!(modal.focus, FocusTarget::CommandPalette);
        assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
        let _ = yoctui_model::update(&mut modal, Action::CloseCommandPalette);
        assert_eq!(modal.focus, FocusTarget::Navigator);
        assert_eq!(modal.focus_return, None);

        let mut terminal_app = yoctui_model::App::new(32, 8192);
        terminal_app.screen = AppScreen::TerminalSessions;
        terminal_app.focus = FocusTarget::Workspace;
        terminal_app
            .daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id: 1,
                name: "terminal".into(),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            });
        let terminal_output = rendered(&terminal_app, 160, 50);
        assert!(
            terminal_output.contains("terminal #1 Running"),
            "{terminal_output}"
        );
        let retained_focus = terminal_app.focus;
        let mut prefix = PrefixState::new(Duration::from_secs(1));
        assert_eq!(
            prefix.feed(Input::CtrlB, Instant::now()),
            PrefixEvent::Awaiting
        );
        assert_eq!(
            prefix.feed(Input::Char('n'), Instant::now()),
            PrefixEvent::Command(PrefixCommand::NextSession)
        );
        assert_eq!(terminal_app.focus, retained_focus);
        assert_eq!(key_action(Input::CtrlB), None);
        dispatch(&mut terminal_app, Input::Tab);
        assert_eq!(terminal_app.focus, FocusTarget::Navigator);
    }
