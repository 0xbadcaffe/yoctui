use ratatui::{Terminal, backend::TestBackend};
use yoctui_app::*;
use yoctui_model::*;

fn fixture() -> App {
    let mut app = App::new(32, 8192);
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::OpenRecipe {
            recipe: "demo".into(),
            root: "/source".into(),
        }),
    );
    app.workbench.file_request = 1;
    let id = app.workbench.active.unwrap();
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::FileLoaded {
            session: id,
            generation: 1,
            root: "/source".into(),
            path: "/source/main.c".into(),
            result: Ok("int main(void) {\n\treturn 0;  \n}\n".into()),
        }),
    );
    app
}
fn render(app: &App, w: u16, h: u16) -> String {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|frame| yoctui_ui::render(frame, app)).unwrap();
    t.backend()
        .buffer()
        .content
        .chunks(w as usize)
        .map(|row| row.iter().map(|c| c.symbol()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn workbench_whitespace_syntax_and_gutter_mouse_agree_at_all_terminal_sizes() {
    let mut app = fixture();
    update(
        &mut app,
        Action::Workbench(WorkbenchAction::ToggleWhitespace),
    );
    for (w, h) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = render(&app, w, h);
        assert!(text.contains('→') && text.contains('·'), "{text}");
        assert!(text.contains("return"));
        let (row, line) = text
            .lines()
            .enumerate()
            .find(|(_, line)| line.contains("return"))
            .unwrap();
        // Use the actual rendered substring, not a guessed layout coordinate.
        let col = line[..line.find("return").unwrap()].chars().count();
        assert_eq!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: col as u16,
                    row: row as u16
                },
                &app,
                w,
                h
            ),
            Some(Action::Workbench(WorkbenchAction::Edit(
                PopupEditorCommand::SelectPosition {
                    line: 1,
                    column: 1,
                    extend: false
                }
            ))),
            "{text}"
        );
        let [x, y, width, height] = workbench_editor_rect(&app, w, h).unwrap();
        assert_eq!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::ScrollDown,
                    column: x + width / 2,
                    row: y + height / 2
                },
                &app,
                w,
                h
            ),
            Some(Action::Workbench(WorkbenchAction::ScrollEditor(3)))
        );
    }
    assert_eq!(
        app.workbench.current().unwrap().tabs[0].document.text,
        "int main(void) {\n\treturn 0;  \n}\n"
    );
}
#[test]
fn workbench_debug_and_tab_picker_render_without_losing_the_editor() {
    let mut app = fixture();
    for (w, h) in [(80, 24), (160, 48)] {
        update(&mut app, Action::Workbench(WorkbenchAction::Debug));
        app.workbench.debug_dialog.as_mut().unwrap().error = Some("TEST_ERROR_VISIBLE".into());
        assert!(render(&app, w, h).contains("Workbench GDB"));
        assert!(render(&app, w, h).contains("TEST_ERROR_VISIBLE"));
        update(&mut app, Action::Workbench(WorkbenchAction::Cancel));
        update(&mut app, Action::Workbench(WorkbenchAction::Tabs));
        assert!(render(&app, w, h).contains("Editor tabs"));
        update(&mut app, Action::Workbench(WorkbenchAction::Cancel));
    }
}

#[test]
fn workbench_unicode_and_double_digit_tab_labels_match_mouse_hit_testing() {
    let mut app = fixture();
    for index in 1..=12 {
        app.workbench.file_request += 1;
        let generation = app.workbench.file_request;
        let session = app.workbench.active.unwrap();
        update(
            &mut app,
            Action::Workbench(WorkbenchAction::FileLoaded {
                session,
                generation,
                root: "/source".into(),
                path: format!("/source/界界界界界界界界-{index}.c").into(),
                result: Ok("int x;".into()),
            }),
        );
    }
    let (w, h) = (160, 48);
    let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
    terminal
        .draw(|frame| yoctui_ui::render(frame, &app))
        .unwrap();
    let [x, y, width, _] = workbench_editor_rect(&app, w, h).unwrap();
    let count = usize::from(width / 20).max(1);
    let first = 12 - count + 1;
    for slot in 0..count {
        let column = x + slot as u16 * 20;
        let row = y - 1;
        let label = format!("{:>2}", first + slot + 1);
        for offset in 0..2 {
            assert_eq!(
                terminal.backend().buffer()[(column + offset as u16, row)].symbol(),
                &label[offset..offset + 1]
            );
        }
        assert_eq!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: column + 4,
                    row
                },
                &app,
                w,
                h
            ),
            Some(Action::Workbench(WorkbenchAction::SelectTab(first + slot)))
        );
    }
}
