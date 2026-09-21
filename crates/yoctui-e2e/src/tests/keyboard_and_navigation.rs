    #[test]
    fn keyboard_matrix_is_complete_and_unique() {
        let keys = [
            "?",
            "F5",
            "Ctrl+P",
            "/",
            "Tab",
            "Shift+Tab",
            "Esc",
            "q",
            "Ctrl+C",
            "Up",
            "Down",
            "j",
            "k",
            "Enter",
            "Backspace",
            "Right",
            "Left",
            "l",
            "h",
            "e",
            "o",
            "R",
            "r",
            ".",
            "g",
            "m",
            "d",
            "f",
            "F",
            "n",
            "N",
            "w",
            "s",
            "T",
            "B",
            "C",
            "L",
            "1",
            "2",
            "c",
            "Q",
            "x",
            "D",
            "i",
            "v",
            "Space",
            "Ctrl+S",
            "Ctrl+B",
        ];
        let mut sorted = keys.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len());
        assert!(keys.contains(&"Shift+Tab"));
        assert!(keys.contains(&"Ctrl+P"));
    }

    #[test]
    fn navigation_flow_preserves_semantic_anchors_after_resize() {
        let before = parse_screen(b"\x1b[1;1HYoctui\x1b[24;1HTab Focus", 80, 24);
        let after = parse_screen(b"\x1b[1;1HYoctui\x1b[30;1HTab Focus", 100, 30);
        assert!(before.text().contains("Yoctui"));
        assert!(before.text().contains("Tab Focus"));
        assert!(after.text().contains("Yoctui"));
        assert!(after.text().contains("Tab Focus"));
    }

    #[test]
    fn visual_snapshot_normalization_discards_terminal_control_noise() {
        let screen = parse_screen(b"\x1b[?1049h\x1b[2J\x1b[1;1HYoctui\x1b[0m", 20, 4);
        assert_eq!(screen.text().lines().next(), Some("Yoctui              "));
        assert!(!screen.text().contains('\x1b'));
    }

    #[test]
    fn next_generation_keymap_dispatches_catalog_and_matches_help_footer() {
        let function_inputs = [
            Input::F1,
            Input::F2,
            Input::F3,
            Input::F4,
            Input::F5,
            Input::F6,
            Input::F7,
            Input::F8,
            Input::F9,
            Input::F10,
        ];
        let mut labels = std::collections::BTreeSet::new();
        for (input, shortcut) in function_inputs
            .into_iter()
            .zip(yoctui_model::FUNCTION_SHORTCUTS)
        {
            assert!(
                labels.insert(shortcut.key_label),
                "duplicate authoritative function-key label {}",
                shortcut.key_label
            );
            let expected = yoctui_model::function_shortcut_action(shortcut.key);
            assert_eq!(key_action(input), Some(expected.clone()));
            let mut app = yoctui_model::App::new(32, 8192);
            let _ = yoctui_model::update(&mut app, expected);
            match shortcut.route {
                FunctionShortcutRoute::Open(screen) => assert_eq!(app.screen, screen),
                FunctionShortcutRoute::CommandPalette => {
                    assert!(app.command_palette_open);
                    assert_eq!(app.focus, FocusTarget::CommandPalette);
                }
                FunctionShortcutRoute::ApplicationMenu => {
                    assert!(app.menu.is_open());
                    assert_eq!(app.focus, FocusTarget::Dialog);
                }
            }
        }

        let global = [
            ("?", Input::Char('?'), Action::Open(AppScreen::Help)),
            ("Ctrl+P", Input::CtrlP, Action::OpenCommandPalette),
            ("Tab", Input::Tab, Action::CycleFocus { backwards: false }),
            (
                "Shift+Tab",
                Input::BackTab,
                Action::CycleFocus { backwards: true },
            ),
            ("Esc", Input::Esc, Action::Open(AppScreen::Dashboard)),
            ("B", Input::Char('B'), Action::OpenBuildOptions),
            ("q", Input::Char('q'), Action::Quit),
            ("Ctrl+C", Input::CtrlC, Action::Quit),
        ];
        let mut global_inputs = Vec::new();
        for (label, input, expected) in global {
            assert!(
                !global_inputs.contains(&input),
                "documented global binding {label} is duplicated or shadowed"
            );
            global_inputs.push(input);
            assert_eq!(key_action(input), Some(expected), "global {label}");
        }

        for (label, input, expected) in [
            (
                "Navigator Up",
                Input::Up,
                Action::SelectNavigator { delta: -1 },
            ),
            (
                "Navigator Down",
                Input::Down,
                Action::SelectNavigator { delta: 1 },
            ),
            ("Navigator Enter", Input::Enter, Action::ActivateNavigator),
            (
                "Navigator Left",
                Input::Left,
                Action::CollapseNavigatorGroup,
            ),
            (
                "Navigator Right",
                Input::Right,
                Action::ExpandNavigatorGroup,
            ),
            (
                "Navigator h",
                Input::Char('h'),
                Action::CollapseNavigatorGroup,
            ),
            (
                "Navigator l",
                Input::Char('l'),
                Action::ExpandNavigatorGroup,
            ),
        ] {
            assert_eq!(
                focus_action(FocusTarget::Navigator, input),
                Some(expected),
                "{label}"
            );
        }
        for (label, input, expected) in [
            (
                "Tasks Up",
                Input::Up,
                Action::ScrollBuildTasks { delta: -1 },
            ),
            (
                "Tasks Down",
                Input::Down,
                Action::ScrollBuildTasks { delta: 1 },
            ),
            ("Tasks f", Input::Char('f'), Action::CycleTaskStateFilter),
            ("Tasks F", Input::Char('F'), Action::CycleTaskFilterField),
            ("Tasks /", Input::Char('/'), Action::BeginTaskFilterEdit),
            ("Tasks d", Input::Char('d'), Action::CycleTaskDurationFilter),
        ] {
            assert_eq!(tasks_action(false, input), Some(expected), "{label}");
        }
        for (label, input, expected) in [
            ("Logs f", Input::Char('f'), Action::ToggleLogFollow),
            ("Logs w", Input::Char('w'), Action::ToggleLogWrap),
            ("Logs s", Input::Char('s'), Action::CycleLogSeverity),
            ("Logs /", Input::Char('/'), Action::BeginLogSearch),
            ("Logs n", Input::Char('n'), Action::NextLogMatch),
            ("Logs N", Input::Char('N'), Action::PreviousLogMatch),
            ("Logs Ctrl+U", Input::CtrlU, Action::ClearLogQuery),
            (
                "Logs Left",
                Input::Left,
                Action::ScrollLogsHorizontally { delta: -8 },
            ),
            (
                "Logs Right",
                Input::Right,
                Action::ScrollLogsHorizontally { delta: 8 },
            ),
        ] {
            assert_eq!(logs_action(false, input), Some(expected), "{label}");
        }

        let now = Instant::now();
        for (key, command) in [
            (Input::Char('c'), PrefixCommand::CreateSession),
            (Input::Char('n'), PrefixCommand::NextSession),
            (Input::Char('p'), PrefixCommand::PreviousSession),
            (Input::Char('%'), PrefixCommand::SplitHorizontal),
            (Input::Char('"'), PrefixCommand::SplitVertical),
            (Input::Char('x'), PrefixCommand::ClosePane),
            (Input::Char('d'), PrefixCommand::Detach),
            (Input::Char(':'), PrefixCommand::CommandPalette),
            (Input::Char('?'), PrefixCommand::Help),
            (Input::Char('o'), PrefixCommand::TakeControl),
        ] {
            let mut prefix = PrefixState::new(Duration::from_secs(1));
            assert_eq!(prefix.feed(Input::CtrlB, now), PrefixEvent::Awaiting);
            assert_eq!(prefix.feed(key, now), PrefixEvent::Command(command));
            assert!(!prefix.pending());
        }

        assert_eq!(
            devtool_reset_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolReset)
        );
        assert_eq!(
            devtool_reset_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolReset)
        );
        assert_eq!(
            devtool_reset_confirmation_action(Input::Char('q')),
            None,
            "modal bindings trap unmatched global shortcuts"
        );

        let rendered = |screen| {
            let mut app = yoctui_model::App::new(32, 8192);
            app.screen = screen;
            app.focus = yoctui_model::FocusTarget::Workspace;
            let mut terminal = Terminal::new(TestBackend::new(300, 40)).unwrap();
            terminal
                .draw(|frame| yoctui_ui::render_at(frame, &app, UNIX_EPOCH))
                .unwrap();
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>()
        };
        let help = rendered(AppScreen::Help);
        for shortcut in yoctui_model::FUNCTION_SHORTCUTS {
            let documented = format!("{} {}", shortcut.key_label, shortcut.action_label);
            assert!(help.contains(&documented), "Help lost {documented}: {help}");
        }
        for screen in [
            AppScreen::Dashboard,
            AppScreen::Tasks,
            AppScreen::BuildHistory,
            AppScreen::Logs,
            AppScreen::Layers,
            AppScreen::Recipes,
            AppScreen::Images,
        ] {
            let footer = rendered(screen);
            for shortcut in yoctui_model::FUNCTION_SHORTCUTS {
                if footer.contains(shortcut.key_label) {
                    let documented = format!("{} {}", shortcut.key_label, shortcut.action_label);
                    assert!(
                        footer.contains(&documented),
                        "{screen:?} footer disagrees with keymap for {documented}: {footer}"
                    );
                }
            }
        }
    }
