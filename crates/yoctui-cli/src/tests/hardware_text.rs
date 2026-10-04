use super::*;
use yoctui_model::{
    Dialog, HardwareDocument, HardwareProjectAction, HardwareProjectOperation, SourceEditorContext,
    TextAreaRevision,
};

fn fixture() -> PathBuf {
    let path = temporary_path("hardware-text-test");
    fs::create_dir(&path).unwrap();
    path
}

#[tokio::test]
async fn hardware_text_browser_and_loader_support_any_suffix_and_extensionless_text() {
    let root = fixture();
    for name in [
        "README",
        "Makefile",
        "driver.c",
        "board.dts",
        "requirements.md",
        "metadata.xyz",
        "data.bin",
        "empty",
    ] {
        let source = if name == "empty" {
            ""
        } else {
            "# notes\nUTF-8 α\n\n"
        };
        let path = root.join(name);
        fs::write(&path, source).unwrap();
        assert_eq!(
            text::file_kind(&path, false),
            Some(HardwareDocumentKind::Text)
        );
        let (_, preview, _) = load_document(HardwareLoadRequest {
            generation: 1,
            document: HardwareDocument {
                path: path.clone(),
                category: yoctui_model::HardwareCategory::Other,
                kind: HardwareDocumentKind::Text,
            },
            page: 1,
        })
        .await
        .unwrap();
        assert_eq!(preview, HardwarePreview::Source(source.into()));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            source,
            "opening must not rewrite"
        );
    }
    fs::write(root.join("binary.unknown"), [0, 255, 1]).unwrap();
    fs::write(root.join("binary.txt"), [0, 1]).unwrap();
    let (_, entries) = browse_directory(&root).unwrap();
    assert_eq!(entries.len(), 8);
    assert!(
        entries
            .iter()
            .all(|entry| entry.kind == Some(HardwareDocumentKind::Text))
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn hardware_text_load_revalidates_binary_size_symlinks_and_special_files() {
    let root = fixture();
    let path = root.join("source.anything");
    fs::write(&path, "readable").unwrap();
    assert_eq!(
        text::file_kind(&path, true),
        Some(HardwareDocumentKind::Text)
    );
    for bytes in [
        vec![0, 1],
        vec![255, 254],
        b"escape\x1b[31m".to_vec(),
        vec![b'x'; yoctui_model::TEXTAREA_MAX_BYTES + 1],
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(text::source_editor_content(&path).is_err());
    }
    fs::remove_file(&path).unwrap();
    assert!(text::source_editor_content(&path).is_err());
    #[cfg(unix)]
    {
        let target = root.join("original");
        fs::write(&target, "original").unwrap();
        std::os::unix::fs::symlink(&target, &path).unwrap();
        assert!(text::source_editor_content(&path).is_err());
        fs::remove_file(&path).unwrap();
        let fifo = std::ffi::CString::new(path.to_string_lossy().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(text::source_editor_content(&path).is_err());
    }
    fs::remove_dir_all(root).unwrap();
}

async fn poll(io: &mut HardwareIo, app: &mut App) {
    tokio::time::timeout(Duration::from_secs(5), async {
        while !io.poll(app).await {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn hardware_text_library_edits_readable_graphical_source_and_saves_exact_content() {
    let root = fixture();
    let path = root.join("board.svg");
    let original = "<svg>\r\n  <!-- original -->\r\n</svg>\r\n";
    fs::write(&path, original).unwrap();
    let mut app = App::new(32, 4096);
    app.screen = yoctui_model::Screen::Hardware;
    app.hardware.documents.push(HardwareDocument {
        path: path.clone(),
        category: app.hardware.category,
        kind: HardwareDocumentKind::Svg,
    });
    let mut io = HardwareIo::default();
    io.submit(
        yoctui_model::update(&mut app, Action::Hardware(HardwareAction::EditSelected)).unwrap(),
    );
    poll(&mut io, &mut app).await;
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("library source not opened");
    };
    assert_eq!(editor.context, SourceEditorContext::HardwareLibrary);
    assert_eq!(editor.document.text, original);
    let editor = editor.clone();
    assert_eq!(
        crate::recipe_editor::read_source_editor_file(&editor, &path).unwrap(),
        original
    );
    assert_eq!(app.hardware.documents[0].kind, HardwareDocumentKind::Svg);
    yoctui_model::update(&mut app, Action::ToggleRecipeEditorEditing);
    yoctui_model::update(&mut app, Action::AppendRecipeEditor(' '));
    let Some(Effect::SaveRecipeEditorFile {
        root: saved_root,
        path: saved_path,
        content,
        expected,
    }) = yoctui_model::update(&mut app, Action::SaveRecipeEditor)
    else {
        panic!("no library save");
    };
    crate::save_recipe_editor_file(&mut app, saved_root, saved_path, content.clone(), expected)
        .await;
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    assert!(content.ends_with("\r\n"));
    yoctui_model::update(&mut app, Action::CloseRecipeEditor);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.hardware.documents[0].path, path);
    for unsafe_source in [
        "binary\0".into(),
        "x".repeat(yoctui_model::TEXTAREA_MAX_BYTES + 1),
    ] {
        fs::write(&path, unsafe_source).unwrap();
        assert!(crate::recipe_editor::read_source_editor_file(&editor, &path).is_err());
    }
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn hardware_text_project_worker_edit_save_conflict_and_path_guards_round_trip() {
    // Isolate the real XDG-backed project adapter in a child test process, without
    // changing the parent test runner's environment or touching user projects.
    if std::env::var_os("YOCTUI_HARDWARE_TEXT_TEST_CHILD").is_none() {
        let data = fixture();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "hardware_text_project_worker_edit_save_conflict_and_path_guards_round_trip",
                "--nocapture",
            ])
            .env("YOCTUI_HARDWARE_TEXT_TEST_CHILD", "1")
            .env("XDG_DATA_HOME", &data)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        fs::remove_dir_all(data).unwrap();
        return;
    }
    let mut app = App::new(32, 4096);
    app.screen = yoctui_model::Screen::Hardware;
    app.hardware.projects.visible = true;
    let mut io = HardwareIo::default();
    io.submit(
        yoctui_model::update(
            &mut app,
            Action::Hardware(HardwareAction::Project(HardwareProjectAction::Request(
                HardwareProjectOperation::Create {
                    name: "board".into(),
                },
            ))),
        )
        .unwrap(),
    );
    poll(&mut io, &mut app).await;
    let root = app.hardware.projects.project.as_ref().unwrap().root.clone();
    let path = root.join("startup");
    let original = "#!/bin/sh\necho original\n\n";
    fs::write(&path, original).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o750)).unwrap();
    }
    io.submit(
        yoctui_model::update(
            &mut app,
            Action::Hardware(HardwareAction::Project(HardwareProjectAction::Reload)),
        )
        .unwrap(),
    );
    poll(&mut io, &mut app).await;
    assert_eq!(
        app.hardware.projects.entries[0].kind,
        Some(HardwareDocumentKind::Text)
    );
    io.submit(
        yoctui_model::update(
            &mut app,
            Action::Hardware(HardwareAction::Project(HardwareProjectAction::Open)),
        )
        .unwrap(),
    );
    poll(&mut io, &mut app).await;
    let Some(Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("source editor not opened");
    };
    assert_eq!(editor.context, SourceEditorContext::HardwareProject);
    assert_eq!(editor.document.text, original);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    yoctui_model::update(&mut app, Action::ToggleRecipeEditorEditing);
    yoctui_model::update(&mut app, Action::AppendRecipeEditor('#'));
    let Some(Effect::SaveRecipeEditorFile {
        root,
        path,
        content,
        expected,
    }) = yoctui_model::update(&mut app, Action::SaveRecipeEditor)
    else {
        panic!("no save effect");
    };
    crate::save_recipe_editor_file(
        &mut app,
        root.clone(),
        path.clone(),
        content.clone(),
        expected,
    )
    .await;
    assert_eq!(fs::read_to_string(&path).unwrap(), content);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o750
        );
    }
    fs::write(&path, "external update\n").unwrap();
    crate::save_recipe_editor_file(
        &mut app,
        root.clone(),
        path.clone(),
        "replacement".into(),
        TextAreaRevision::of(&content),
    )
    .await;
    assert_eq!(fs::read_to_string(&path).unwrap(), "external update\n");
    assert!(
        app.notification
            .as_ref()
            .unwrap()
            .contains("changed on disk")
    );
    let outside = root.parent().unwrap().join("outside.txt");
    fs::write(&outside, "outside").unwrap();
    crate::save_recipe_editor_file(
        &mut app,
        root.clone(),
        outside.clone(),
        "replacement".into(),
        TextAreaRevision::of("outside"),
    )
    .await;
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside");
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        std::os::unix::fs::symlink(&outside, &path).unwrap();
        crate::save_recipe_editor_file(
            &mut app,
            root,
            path,
            "replacement".into(),
            TextAreaRevision::of("outside"),
        )
        .await;
        assert_eq!(fs::read_to_string(&outside).unwrap(), "outside");
    }
}
