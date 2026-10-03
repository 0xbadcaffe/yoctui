use super::*;

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = env::temp_dir().join(format!(
            "yoctui-image-artifact-view-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn image_artifact_view_reads_text_dts_and_associations_without_writes() {
    let fixture = Fixture::new();
    for name in [
        "board.dts",
        "board.dtsi",
        "image.manifest",
        "image.spdx.json",
        "image.wks",
    ] {
        let path = fixture.file(name, b"/dts-v1/;\n/ { model = \"board\"; };\n");
        let before = fs::read(&path).unwrap();
        assert_eq!(
            inspect_image_artifact(&fixture.0, &path, None).unwrap(),
            ImageArtifactView::Text(String::from_utf8(before.clone()).unwrap())
        );
        assert_eq!(fs::read(path).unwrap(), before);
    }
}

#[test]
fn image_artifact_view_dtb_and_overlay_return_only_review_observations() {
    let fixture = Fixture::new();
    for (name, kind) in [
        ("board.dtb", PlatformFileKind::Dtb),
        ("board.dtbo", PlatformFileKind::Dtbo),
    ] {
        let path = fixture.file(name, &[0xd0, 0x0d, 0xfe, 0xed]);
        let error = inspect_image_artifact(&fixture.0, &path, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("dtc is unavailable") && error.contains("device-tree-compiler"));
        assert_eq!(
            inspect_image_artifact(&fixture.0, &path, Some("/tools/dtc".into())).unwrap(),
            ImageArtifactView::DeviceTree {
                kind,
                program: "/tools/dtc".into(),
                size_bytes: 4
            }
        );
        assert!(!path.with_extension("yoctui.dts").exists());
        assert_eq!(fs::read(path).unwrap(), vec![0xd0, 0x0d, 0xfe, 0xed]);
    }
}

#[test]
fn image_artifact_view_refuses_binary_oversized_missing_and_outside_files() {
    let fixture = Fixture::new();
    for bytes in [&[0xff, 0xfe][..], &[0, 1, 2][..], b"hello\x1b[2J"] {
        let path = fixture.file("binary.ext4", bytes);
        assert!(inspect_image_artifact(&fixture.0, &path, None).is_err());
    }
    let path = fixture.file("large.txt", b"");
    OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_len(TEXTAREA_MAX_BYTES as u64 + 1)
        .unwrap();
    assert!(
        inspect_image_artifact(&fixture.0, &path, None)
            .unwrap_err()
            .to_string()
            .contains("viewer limit")
    );
    for path in [
        fixture.0.join("missing"),
        fixture.0.join("../escape"),
        PathBuf::from("/etc/passwd"),
        fixture.0.clone(),
    ] {
        assert!(inspect_image_artifact(&fixture.0, &path, None).is_err());
    }
}

#[cfg(unix)]
#[test]
fn image_artifact_view_rejects_symlink_ancestors_leaf_directory_and_fifo() {
    use std::os::unix::{ffi::OsStrExt, fs::symlink};
    let fixture = Fixture::new();
    let path = fixture.file("text", b"hello");
    symlink(&path, fixture.0.join("link")).unwrap();
    symlink(&fixture.0, fixture.0.join("nested-link")).unwrap();
    for path in [fixture.0.join("link"), fixture.0.join("nested-link/text")] {
        assert!(inspect_image_artifact(&fixture.0, &path, None).is_err());
    }
    let dir = fixture.0.join("directory");
    fs::create_dir(&dir).unwrap();
    assert!(inspect_image_artifact(&fixture.0, &dir, None).is_err());
    let fifo = fixture.0.join("fifo");
    let name = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
    assert!(inspect_image_artifact(&fixture.0, &fifo, None).is_err());
    // Direct descriptor lookup also rejects links, independent of preflight.
    assert!(open_image_file(&fixture.0, &fixture.0.join("link")).is_err());
    assert!(open_image_file(&fixture.0, &fixture.0.join("nested-link/text")).is_err());
}

#[tokio::test]
async fn image_artifact_view_runtime_opens_exact_text_and_reports_binary_failure() {
    use yoctui_model::{
        ImageArtifact, ImageArtifactField as Field, ImageArtifactIdentity, ImageArtifactInventory,
        ImageArtifactInventoryState, ImageArtifactKind, ImageArtifactRequest,
    };
    let fixture = Fixture::new();
    let path = fixture.file("board.dts", b"/dts-v1/;\n/ { model = \"test board\"; };\n");
    let mut app = App::new(20, 20_000);
    app.screen = yoctui_model::Screen::Images;
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: "core-image-minimal".into(),
            path: path.clone(),
        },
        kind: ImageArtifactKind::Other,
        size_bytes: Field::Unavailable,
        modified_unix_seconds: Field::Unavailable,
        checksums: Field::Unavailable,
        manifests: Field::Unavailable,
        licenses: Field::Unavailable,
        spdx: Field::Unavailable,
        wic_files: Field::Unavailable,
    };
    app.image_artifact_selection = Some(artifact.identity.clone());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: Field::Available(fixture.0.clone()),
            artifacts: vec![artifact],
        },
    };
    open_image_artifact(&mut app, fixture.0.clone(), path.clone()).await;
    let Some(yoctui_model::Dialog::RecipeEditor(editor)) = app.active_dialog() else {
        panic!("expected internal DTS viewer")
    };
    assert_eq!(editor.selected_path(), Some(path.clone()));
    assert_eq!(editor.language, yoctui_model::SourceLanguage::DeviceTree);
    assert!(editor.document.text.contains("test board"));
    assert!(!editor.is_dirty());
    app.dialogs.clear();
    fs::write(&path, [0xff, 0xfe]).unwrap();
    open_image_artifact(&mut app, fixture.0.clone(), path).await;
    assert!(app.active_dialog().is_none());
    assert!(
        app.notification
            .as_ref()
            .unwrap()
            .contains("Binary artifact")
    );
}

#[test]
fn image_artifact_view_real_dtc_roundtrip_smoke() {
    let Some(program) = terminal_launcher::executable_on_initialized_path("dtc") else {
        eprintln!(
            "live dtc smoke unavailable: install device-tree-compiler; fake/model coverage is not a real decompile"
        );
        return;
    };
    let fixture = Fixture::new();
    let source = fixture.file(
        "board.dts",
        b"/dts-v1/;\n/ { model = \"yoctui artifact smoke\"; compatible = \"yoctui,test\"; };\n",
    );
    let dtb = fixture.0.join("board.dtb");
    let compile = ProcessCommand::new(&program)
        .args(["-I", "dts", "-O", "dtb", "-o"])
        .arg(&dtb)
        .arg(source)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let original = fs::read(&dtb).unwrap();
    let ImageArtifactView::DeviceTree {
        kind,
        program,
        size_bytes,
    } = inspect_image_artifact(&fixture.0, &dtb, Some(program)).unwrap()
    else {
        panic!("expected DTB observation")
    };
    let dialog = yoctui_model::DtcDecompileDialog::new(
        yoctui_model::PlatformComponent::Images,
        &yoctui_model::PlatformFile {
            path: dtb.clone(),
            root: fixture.0.clone(),
            kind,
            size_bytes,
        },
        program,
    );
    assert!(!dialog.output_path().exists());
    let request = dialog.terminal_request();
    let decompile = ProcessCommand::new(request.program)
        .args(request.arguments)
        .current_dir(request.cwd)
        .output()
        .unwrap();
    assert!(
        decompile.status.success(),
        "{}",
        String::from_utf8_lossy(&decompile.stderr)
    );
    let ImageArtifactView::Text(text) =
        inspect_image_artifact(&fixture.0, &dialog.output_path(), None).unwrap()
    else {
        panic!("expected decompiled DTS")
    };
    assert!(text.contains("yoctui artifact smoke") && text.contains("yoctui,test"));
    assert_eq!(fs::read(dtb).unwrap(), original);
}
