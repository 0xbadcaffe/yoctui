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
