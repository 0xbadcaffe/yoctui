use super::*;

#[test]
fn hardware_desktop_reader_rejects_links_non_pdf_and_unregistered_project_roots() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    fs::create_dir(&root).unwrap();
    let path = root.join("board with spaces.pdf");
    fs::write(&path, b"%PDF-1.4\n").unwrap();
    let mut document = yoctui_model::HardwareDocument {
        path: path.clone(),
        category: yoctui_model::HardwareCategory::Board,
        kind: HardwareDocumentKind::Pdf,
    };
    assert_eq!(
        validated_pdf(&document, None).unwrap(),
        fs::canonicalize(&path).unwrap()
    );
    assert!(validated_pdf(&document, Some(&root)).is_err());
    fs::write(&path, b"#!/bin/sh\necho not-a-PDF\n").unwrap();
    assert!(validated_pdf(&document, None).is_err());
    fs::write(&path, b"%PDF-1.4\n").unwrap();
    document.kind = HardwareDocumentKind::Text;
    assert!(validated_pdf(&document, None).is_err());
    document.kind = HardwareDocumentKind::Pdf;
    let outside = temporary.path().join("outside.pdf");
    fs::write(&outside, b"%PDF-1.4\n").unwrap();
    document.path = outside;
    assert!(validated_pdf(&document, Some(&root)).is_err());
    #[cfg(unix)]
    {
        let link = root.join("link.pdf");
        std::os::unix::fs::symlink(path, &link).unwrap();
        document.path = link;
        assert!(validated_pdf(&document, None).is_err());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn hardware_desktop_launcher_does_not_wait_for_gui_owned_pipe_eof() {
    use std::os::unix::fs::PermissionsExt;
    let temporary = tempfile::tempdir().unwrap();
    let program = temporary.path().join("gio-fixture");
    let gui_pid = temporary.path().join("gui-pid");
    fs::write(
        &program,
        format!(
            "#!/bin/sh\nsleep 10 &\necho $! > '{}'\nexit 0\n",
            gui_pid.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755)).unwrap();
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        launch_reader(
            program.to_str().unwrap(),
            Path::new("/tmp/board with spaces.pdf"),
        ),
    )
    .await;
    // Stop only the fixture's child, including when the EOF regression returns.
    if let Ok(pid) = fs::read_to_string(&gui_pid) {
        let pid: i32 = pid.trim().parse().unwrap();
        unsafe {
            libc::kill(pid, libc::SIGTERM);
        }
    }
    result
        .expect("launch must complete without waiting for GUI pipe EOF")
        .unwrap();
}
