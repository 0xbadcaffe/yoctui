use super::*;
use ratatui::{Terminal, backend::TestBackend};
use yoctui_model::{TaskId, TaskInfo};

fn text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn progress_layout_bars_are_contiguous_bounded_and_ascii_safe() {
    let mut app = App::new(16, 4096);
    assert_eq!(task_progress_bar(&app, 42), "████▏░░░░░ 42%");
    for symbols in [SymbolPreference::Unicode, SymbolPreference::Ascii] {
        app.preferences.symbols = symbols;
        for value in [0, 1, 42, 100, 255] {
            let bar = task_progress_bar(&app, value);
            assert!(bar.ends_with(&format!(" {}%", value.min(100))));
            assert_eq!(bar.split_once(' ').unwrap().0.chars().count(), 10);
            assert!(!bar.contains(['▪', '▫']));
            if symbols == SymbolPreference::Ascii {
                assert!(bar.is_ascii());
            }
        }
    }
}

#[test]
fn progress_layout_reserves_task_progress_before_long_identity_columns() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Tasks;
    app.build.status = BuildStatus::Running;
    app.build.completed = 4;
    app.build.total = Some(10);
    for (id, progress) in [("known", Some(42)), ("unknown", None)] {
        app.tasks.insert(
            TaskId(id.into()),
            TaskInfo {
                id: TaskId(id.into()),
                recipe: "long-recipe-name-".repeat(20),
                task: format!("do_{id}_{}", "long_task_".repeat(20)),
                state: TaskState::Active,
                progress,
                worker: Some("worker-1".into()),
                pid: Some(123),
                ..TaskInfo::default()
            },
        );
    }
    for symbols in [SymbolPreference::Unicode, SymbolPreference::Ascii] {
        app.preferences.symbols = symbols;
        for width in [54, 64, 80, 84, 89, 100, 110, 160] {
            let rows = app.visible_task_row_refs_at(UNIX_EPOCH);
            let mut terminal = Terminal::new(TestBackend::new(width, 17)).unwrap();
            terminal
                .draw(|frame| render_task_table(frame, &app, frame.area(), &rows, UNIX_EPOCH))
                .unwrap();
            let output = text(&terminal);
            assert!(output.contains("42%"), "{width}: {output}");
            assert!(output.contains("progress unknown"), "{width}: {output}");
            assert!(output.contains("Overall"), "{width}: {output}");
        }
    }
    for width in [32, 48, 80] {
        let mut terminal = Terminal::new(TestBackend::new(width, 5)).unwrap();
        terminal
            .draw(|frame| dashboard_render::render_dashboard_tasks(frame, &app, frame.area()))
            .unwrap();
        let output = text(&terminal);
        assert!(output.contains("42%"), "{width}: {output}");
        assert!(output.contains("active"), "{width}: {output}");
    }
}

#[test]
fn progress_layout_insights_wrap_every_whole_numbered_shortcut() {
    let mut app = App::new(16, 4096);
    app.screen = Screen::Insights;
    for selected in yoctui_model::OverviewView::ALL {
        app.overview_view = selected;
        for width in [32, 48, 64, 80, 100, 160] {
            let mut terminal = Terminal::new(TestBackend::new(width, 24)).unwrap();
            terminal
                .draw(|frame| overview_workspace(frame, &app, frame.area(), UNIX_EPOCH))
                .unwrap();
            let output = text(&terminal);
            for (index, view) in yoctui_model::OverviewView::ALL.iter().enumerate() {
                assert!(
                    output.contains(&format!("{} {}", index + 1, view.label())),
                    "{width}: {output}"
                );
            }
        }
    }
}
