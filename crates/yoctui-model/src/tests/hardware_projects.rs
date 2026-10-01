use super::*;

#[test]
fn hardware_project_name_progress_and_preview_policy_are_bounded() {
    for name in [
        "../a",
        ".",
        "..",
        "",
        "trailing ",
        "a\\b",
        ".yoctui-project.toml",
        "bad\nname",
    ] {
        assert!(validate_hardware_project_name(name).is_err());
    }
    assert!(validate_hardware_project_name("Main board α").is_ok());
    let mut project = HardwareProject {
        name: "Main board".into(),
        root: "/data/Main board".into(),
        progress: [100, 50, 25, 0, 100, 25],
    };
    assert_eq!(project.percent(), 50);
    project.progress[0] = 101;
    assert!(project.validate().is_err());
    assert_eq!(
        HardwareDocumentKind::project_kind(Path::new("notes.TXT")),
        Some(HardwareDocumentKind::Text)
    );
    assert_eq!(
        HardwareDocumentKind::project_kind(Path::new("board.png")),
        None
    );
    assert_eq!(
        HardwareDocumentKind::library_kind(Path::new("notes.txt")),
        None
    );
}

#[test]
fn hardware_project_results_are_correlated_and_failed_save_retains_progress() {
    let mut app = App::new(32, 4096);
    let old = HardwareProject {
        name: "board".into(),
        root: "/data/board".into(),
        progress: [0; 6],
    };
    app.hardware.projects.project = Some(old.clone());
    let effect = update(
        &mut app,
        Action::Hardware(HardwareAction::Project(HardwareProjectAction::Request(
            HardwareProjectOperation::SaveProgress {
                name: "board".into(),
                progress: [100; 6],
            },
        ))),
    );
    let Some(Effect::Hardware(HardwareEffect::Project(request))) = effect else {
        panic!()
    };
    let _ = update(
        &mut app,
        Action::Hardware(HardwareAction::Project(HardwareProjectAction::Finished {
            generation: request.generation + 1,
            result: Ok(HardwareProjectResult::Progress(HardwareProject {
                progress: [100; 6],
                ..old.clone()
            })),
        })),
    );
    assert!(app.hardware.projects.loading);
    assert_eq!(
        app.hardware.projects.project.as_ref().unwrap().progress,
        [0; 6]
    );
    let _ = update(
        &mut app,
        Action::Hardware(HardwareAction::Project(HardwareProjectAction::Finished {
            generation: request.generation,
            result: Err("permission denied".into()),
        })),
    );
    assert_eq!(app.hardware.projects.project.unwrap(), old);
    assert_eq!(
        app.hardware.projects.error.as_deref(),
        Some("permission denied")
    );
}
