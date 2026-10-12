use super::*;
use yoctui_model::{FileDiffAction as Diff, FileDiffRequest};

fn browser_app() -> App {
    let mut app = App::new(16, 4096);
    update(&mut app, Action::Open(Screen::DiffTool));
    app.onboarding.open = false;
    app.preferences.mouse_enabled = true;
    for pane in 0..2 {
        let Some(yoctui_model::Effect::FileDiff(FileDiffRequest::Directory {
            request,
            directory,
            ..
        })) = yoctui_model::file_diff_initial_request(&mut app, "/tmp".into())
        else {
            panic!("scan")
        };
        update(
            &mut app,
            Action::FileDiff(Diff::DirectoryLoaded {
                pane,
                request,
                directory,
                result: Ok((0..150)
                    .map(|n| yoctui_model::LayerBrowserEntry {
                        path: format!("/tmp/file {n:03}.txt").into(),
                        is_dir: false,
                        ..Default::default()
                    })
                    .collect()),
            }),
        );
    }
    app
}

#[test]
fn file_diff_browsers_scroll_marks_are_colored_and_mouse_hits_current_rows() {
    use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};
    for (w, h) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let mut app = browser_app();
        app.file_diff
            .reconcile_viewport(usize::from(h - if w < 100 { 17 } else { 16 }));
        update(
            &mut app,
            Action::FileDiff(Diff::Select { delta: isize::MAX }),
        );
        update(&mut app, Action::FileDiff(Diff::Mark));
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let row = (0..h)
            .find(|&r| {
                (0..w / 2)
                    .map(|c| buffer[(c, r)].symbol())
                    .collect::<String>()
                    .contains("file 149")
            })
            .unwrap();
        let col = (0..w / 2)
            .find(|&c| buffer[(c, row)].symbol() == "*")
            .unwrap();
        assert_eq!(buffer[(col, row)].fg, ThemePalette::for_app(&app).success);
        let action = mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: col,
                row,
            },
            &app,
            w,
            h,
        )
        .unwrap();
        assert_eq!(
            action,
            Action::FileDiff(Diff::SelectAt {
                pane: 0,
                index: 149
            }),
            "{w}x{h}"
        );
        update(&mut app, Action::FileDiff(Diff::Select { delta: -1 }));
        let text = rendered_text_at(&app, w, h, literal_now());
        assert!(text.contains("file 148"));
        assert!(text.contains("F5"));
        assert!(text.contains("F5 compare"));
        assert!(!text.contains("F5 Logs"));
        assert!(!text.contains("Tab Focus"));
        assert!(!text.contains("Recipe Editor"));
    }
}

#[test]
fn file_diff_comparison_is_responsive_colored_and_escape_preserves_browsers() {
    let mut app = browser_app();
    app.file_diff.comparison = Some(
        yoctui_model::compare_file_text(
            ["left.txt".into(), "right.txt".into()],
            "same\nold\n",
            "same\nnew\n",
        )
        .unwrap(),
    );
    for (w, h) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = rendered_text_at(&app, w, h, literal_now());
        assert!(text.contains("Files differ"));
        assert!(text.contains("old") && text.contains("new"));
        assert!(text.contains("left.txt") && text.contains("right.txt"));
    }
    update(&mut app, Action::FileDiff(Diff::Back));
    assert!(app.file_diff.comparison.is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
    update(&mut app, Action::FileDiff(Diff::Back));
    assert_eq!(app.focus, FocusTarget::Navigator);
}

#[test]
fn file_diff_navigator_and_menu_open_same_workspace_without_processes() {
    use yoctui_model::{CommandId, WorkspaceDestination};
    let mut app = browser_app();
    let index = App::navigator_selection_for_destination(WorkspaceDestination::DiffTool).unwrap();
    update(&mut app, Action::SelectNavigatorAt { index });
    update(&mut app, Action::ActivateNavigator);
    assert_eq!(app.screen, Screen::DiffTool);
    assert_eq!(
        yoctui_model::command_action(&app, CommandId::OpenDiffTool),
        Action::Open(Screen::DiffTool)
    );
    assert!(app.daemon.pty_sessions.is_empty());
}

#[test]
fn help_and_operator_guide_are_distinct_and_shortcuts_can_scroll() {
    let mut app = App::new(16, 4096);
    update(&mut app, Action::Open(Screen::Help));
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(
        rendered_text_at(&app, 80, 24, literal_now()).contains("Keyboard and action reference")
    );
    let help = rendered_text_at(&app, 160, 50, literal_now());
    assert!(help.contains("Keyboard and action reference"));
    assert!(!help.contains("Six-step workbench path"));
    app.help_visible_rows = 34;
    update(&mut app, Action::ScrollHelp { delta: isize::MAX });
    let end = app.help_scroll;
    let bottom = rendered_text_at(&app, 160, 50, literal_now());
    assert!(!bottom.contains("Keyboard and action reference"));
    update(&mut app, Action::ScrollHelp { delta: -1 });
    assert_eq!(app.help_scroll + 1, end);
    update(&mut app, Action::OpenOnboarding);
    let guide = rendered_text_at(&app, 160, 50, literal_now());
    assert!(guide.contains("Six-step workbench path"));
    assert!(guide.contains("Operator guide"));
    assert!(guide.contains("Verify environment"));
}
