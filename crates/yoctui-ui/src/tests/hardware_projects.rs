use super::*;
use yoctui_model::{HardwareProject, HardwareProjectEntry, HardwareProjectForm};

#[test]
fn hardware_project_files_progress_and_forms_render_on_small_terminals() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Workspace;
    app.hardware.projects.visible = true;
    app.hardware.projects.project = Some(HardwareProject {
        name: "Board Alpha".into(),
        root: "/data/Board Alpha".into(),
        progress: [100, 50, 25, 0, 100, 25],
    });
    app.hardware.projects.entries = vec![HardwareProjectEntry {
        name: "firmware.bin".into(),
        path: "/data/Board Alpha/firmware.bin".into(),
        is_directory: false,
        size: 12345,
        kind: None,
    }];
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("Board Alpha"));
    assert!(output.contains("User-reported bring-up 50%"), "{output}");
    assert!(output.contains("Stored only"));
    assert!(output.contains("firmware.bin"));
    assert!(output.contains("12345"));
    app.hardware.projects.form = Some(HardwareProjectForm::Progress {
        values: [100, 50, 25, 0, 100, 25],
        stage: 2,
        digits: "25".into(),
    });
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("Manual project bring-up"));
    assert!(output.contains("Device tree"));
    assert!(output.contains("Packages"));
    for (width, height) in [(80, 24), (60, 15), (20, 5), (1, 1)] {
        let _ = rendered_text(&app, width, height);
    }
    app.hardware.projects.form = Some(HardwareProjectForm::Name {
        value: "power rails".into(),
    });
    assert!(rendered_text(&app, 160, 42).contains("power rails_"));
}

#[test]
fn hardware_project_catalog_and_any_file_import_are_visible() {
    let mut app = App::new(32, 4096);
    app.screen = Screen::Hardware;
    app.focus = FocusTarget::Workspace;
    app.hardware.projects.visible = true;
    app.hardware.projects.catalog = vec![HardwareProject {
        name: "Main board".into(),
        root: "/data/Main board".into(),
        progress: [100; 6],
    }];
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("Hardware · Projects"));
    assert!(output.contains("100%"));
    app.hardware.projects.import_browser = Some((
        "/source".into(),
        vec![HardwareProjectEntry {
            name: "arbitrary.bin".into(),
            path: "/source/arbitrary.bin".into(),
            is_directory: false,
            size: 1,
            kind: None,
        }],
        0,
    ));
    let output = rendered_text(&app, 160, 42);
    assert!(output.contains("Import any file"));
    assert!(output.contains("arbitrary.bin"));
}
