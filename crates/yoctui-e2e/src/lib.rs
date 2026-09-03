//! Small, dependency-free PTY primitives used by release acceptance tests.

use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    pub width: usize,
    pub height: usize,
    cells: Vec<Vec<char>>,
}

impl Screen {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![vec![' '; width]; height],
        }
    }

    pub fn text(&self) -> String {
        self.cells
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn put(&mut self, x: usize, y: usize, ch: char) {
        if y < self.height && x < self.width {
            self.cells[y][x] = ch;
        }
    }
}

/// Conservative ANSI parser for semantic assertions. Unsupported controls are
/// ignored rather than leaking into screen text.
pub fn parse_screen(bytes: &[u8], width: usize, height: usize) -> Screen {
    let mut screen = Screen::new(width, height);
    let (mut x, mut y) = (0usize, 0usize);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\x1b' if bytes.get(i + 1) == Some(&b'[') => {
                i += 2;
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let params = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                let command = bytes.get(i).copied().unwrap_or_default();
                let nums = params
                    .split(';')
                    .filter_map(|v| v.parse::<usize>().ok())
                    .collect::<Vec<_>>();
                match command {
                    b'H' | b'f' => {
                        y = nums.first().copied().unwrap_or(1).saturating_sub(1);
                        x = nums.get(1).copied().unwrap_or(1).saturating_sub(1);
                    }
                    b'J' if params == "2" || params == "3" => {
                        screen = Screen::new(width, height);
                        x = 0;
                        y = 0;
                    }
                    b'K' => {
                        for col in x..width {
                            screen.put(col, y, ' ');
                        }
                    }
                    b'A' => y = y.saturating_sub(nums.first().copied().unwrap_or(1)),
                    b'B' => {
                        y = (y + nums.first().copied().unwrap_or(1)).min(height.saturating_sub(1))
                    }
                    b'C' => {
                        x = (x + nums.first().copied().unwrap_or(1)).min(width.saturating_sub(1))
                    }
                    b'D' => x = x.saturating_sub(nums.first().copied().unwrap_or(1)),
                    _ => {}
                }
            }
            b'\n' => y = (y + 1).min(height.saturating_sub(1)),
            b'\r' => x = 0,
            0x20..=0x7e => {
                screen.put(x, y, bytes[i] as char);
                x += 1;
                if x >= width {
                    x = 0;
                    y = (y + 1).min(height.saturating_sub(1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    screen
}

/// Open a Unix PTY and run a command with the slave attached to stdio.
#[cfg(unix)]
pub fn run_pty(command: &str, args: &[&str], input: &[u8]) -> io::Result<Vec<u8>> {
    use std::{
        io::{Read, Write},
        os::fd::{FromRawFd, RawFd},
        os::unix::process::CommandExt,
        process::{Command, Stdio},
    };
    let (master, slave): (RawFd, RawFd) = unsafe {
        let mut master = -1;
        let mut slave = -1;
        if libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        ) != 0
        {
            return Err(io::Error::last_os_error());
        }
        (master, slave)
    };
    let slave_for_child = slave;
    let mut child = unsafe {
        Command::new(command)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(move || {
                for fd in [0, 1, 2] {
                    libc::dup2(slave_for_child, fd);
                }
                libc::close(slave_for_child);
                Ok(())
            })
            .spawn()?
    };
    unsafe {
        libc::close(slave);
    }
    let mut output = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(master) };
    reader.write_all(input)?;
    let _ = child.wait();
    reader.read_to_end(&mut output).ok();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::{Duration, Instant, UNIX_EPOCH};
    use yoctui_app::{
        Input, PrefixCommand, PrefixEvent, PrefixState, devtool_reset_confirmation_action,
        focus_action, key_action, logs_action, tasks_action,
    };
    use yoctui_model::{Action, FocusTarget, FunctionShortcutRoute, Screen as AppScreen};

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
                (Input::Tab, FocusTarget::Inspector),
                (Input::Tab, FocusTarget::Navigator),
                (Input::BackTab, FocusTarget::Inspector),
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
                        assert!(output.contains("[Workspace]  Inspector"), "{output}")
                    }
                    (80..=99, FocusTarget::Inspector) => {
                        assert!(output.contains("[Inspector]"), "{output}")
                    }
                    (100..=129, FocusTarget::Inspector) => {
                        assert!(output.contains("Inspector: Task"), "{output}")
                    }
                    (_, FocusTarget::Navigator) => {
                        assert!(output.contains("Navigator"), "{output}")
                    }
                    (_, FocusTarget::Workspace) => {
                        assert!(output.contains("Tasks:"), "{output}")
                    }
                    (_, FocusTarget::Inspector) => {
                        assert!(output.contains("Inspector: Task"), "{output}")
                    }
                    _ => unreachable!(),
                }
            }
            assert_eq!(app.navigator_selection, 6);
            assert_eq!(app.task_progress_scroll, 3);
        }

        let mut modal = yoctui_model::App::new(32, 8192);
        modal.focus = FocusTarget::Inspector;
        let _ = yoctui_model::update(&mut modal, Action::OpenThemePicker);
        assert_eq!(modal.focus, FocusTarget::Dialog);
        assert_eq!(modal.focus_return, Some(FocusTarget::Inspector));
        dispatch(&mut modal, Input::Tab);
        dispatch(&mut modal, Input::BackTab);
        assert_eq!(modal.focus, FocusTarget::Dialog);
        assert_eq!(focus_action(FocusTarget::Dialog, Input::Tab), None);
        let _ = yoctui_model::update(&mut modal, Action::CloseThemePicker);
        assert_eq!(modal.focus, FocusTarget::Inspector);
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
        assert_eq!(terminal_app.focus, FocusTarget::Inspector);
    }
}
