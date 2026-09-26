use super::*;

#[test]
fn hardware_library_persists_atomically_across_restart() {
    let root = std::env::temp_dir().join(format!("yoctui-hardware-session-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let session_path = root.join("session.toml");
    let document_path = root.join("board.pdf");
    fs::write(&document_path, b"fixture").unwrap();

    let mut app = App::new(100, 100_000);
    app.hardware.documents.push(yoctui_model::HardwareDocument {
        path: document_path,
        category: yoctui_model::HardwareCategory::Board,
        kind: yoctui_model::HardwareDocumentKind::Pdf,
    });
    app.hardware.last_directory = Some(root.clone());
    let mut session = Session::default();
    persist_hardware(Some(&session_path), &mut session, &app).unwrap();

    let restored = read_session(Some(&session_path)).unwrap();
    let mut restarted = App::new(100, 100_000);
    install_session_hardware(&restored, &mut restarted).unwrap();
    assert_eq!(restarted.hardware.documents, app.hardware.documents);
    assert_eq!(restarted.hardware.last_directory, Some(root.clone()));
    assert!(
        !fs::read_to_string(&session_path)
            .unwrap()
            .contains("fixture")
    );
    fs::remove_dir_all(root).unwrap();
}
