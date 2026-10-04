use super::*;

fn send(app: &mut App, action: HardwareProjectAction) -> Option<Effect> {
    update(app, Action::Hardware(HardwareAction::Project(action)))
}

#[test]
fn hardware_project_forms_cancel_and_manual_progress_never_comes_from_build_state() {
    let mut app = App::new(32, 4096);
    let original = HardwareProject {
        name: "board".into(),
        root: "/data/board".into(),
        progress: [0; 6],
    };
    app.hardware.projects.project = Some(original.clone());
    send(&mut app, HardwareProjectAction::NewName);
    for c in "Kernel files".chars() {
        send(&mut app, HardwareProjectAction::EditName(c));
    }
    assert!(
        matches!(send(&mut app, HardwareProjectAction::Confirm), Some(Effect::Hardware(HardwareEffect::Project(HardwareProjectRequest { operation: HardwareProjectOperation::CreateFolder { folder, .. }, .. }))) if folder == "Kernel files")
    );
    app.hardware.projects.loading = false;
    send(&mut app, HardwareProjectAction::Cancel);
    send(&mut app, HardwareProjectAction::BeginProgress);
    for c in "100".chars() {
        send(&mut app, HardwareProjectAction::ProgressDigit(c));
    }
    assert_eq!(app.hardware.projects.project, Some(original.clone()));
    send(&mut app, HardwareProjectAction::Cancel);
    assert_eq!(app.hardware.projects.project, Some(original));
    send(&mut app, HardwareProjectAction::BeginProgress);
    for c in "101".chars() {
        send(&mut app, HardwareProjectAction::ProgressDigit(c));
    }
    assert!(send(&mut app, HardwareProjectAction::Confirm).is_none());
    assert!(app.hardware.projects.error.is_some());
    send(&mut app, HardwareProjectAction::Backspace);
    assert!(matches!(
        send(&mut app, HardwareProjectAction::Confirm),
        Some(Effect::Hardware(HardwareEffect::Project(
            HardwareProjectRequest {
                operation: HardwareProjectOperation::SaveProgress {
                    progress: [10, 0, 0, 0, 0, 0],
                    ..
                },
                ..
            }
        )))
    ));
}

#[test]
fn hardware_project_navigation_restricts_previews_and_preserves_the_library() {
    let mut app = App::new(32, 4096);
    let legacy = HardwareDocument {
        path: "/tmp/library.pdf".into(),
        category: HardwareCategory::Board,
        kind: HardwareDocumentKind::Pdf,
    };
    app.hardware.documents.push(legacy.clone());
    app.hardware.projects.visible = true;
    app.hardware.projects.project = Some(HardwareProject {
        name: "board".into(),
        root: "/data/board".into(),
        progress: [0; 6],
    });
    app.hardware.projects.entries = vec![HardwareProjectEntry {
        name: "notes.txt".into(),
        path: "/data/board/notes.txt".into(),
        is_directory: false,
        size: 1,
        kind: Some(HardwareDocumentKind::Text),
    }];
    let Some(Effect::Hardware(HardwareEffect::LoadProject { root, request })) =
        send(&mut app, HardwareProjectAction::Open)
    else {
        panic!()
    };
    assert_eq!(root, PathBuf::from("/data/board"));
    let _ = update(&mut app, Action::Hardware(HardwareAction::CloseViewer));
    let Some(Effect::Hardware(HardwareEffect::LoadProject {
        request: reopened, ..
    })) = send(&mut app, HardwareProjectAction::Open)
    else {
        panic!()
    };
    assert!(reopened.generation > request.generation);
    let _ = update(&mut app, Action::Hardware(HardwareAction::CloseViewer));
    app.hardware.projects.entries[0].path = "/data/board/data.bin".into();
    // The filesystem adapter reports binary content as Stored only; readable
    // .bin files are now text candidates and must be revalidated on load.
    app.hardware.projects.entries[0].kind = None;
    assert!(send(&mut app, HardwareProjectAction::Open).is_none());
    assert!(
        app.hardware
            .projects
            .error
            .as_deref()
            .unwrap()
            .contains("Stored only")
    );
    assert_eq!(app.hardware.documents, vec![legacy]);
    assert!(matches!(
        send(&mut app, HardwareProjectAction::Parent),
        Some(Effect::Hardware(HardwareEffect::Project(
            HardwareProjectRequest {
                operation: HardwareProjectOperation::List,
                ..
            }
        )))
    ));
}

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
        Some(HardwareDocumentKind::Text)
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
