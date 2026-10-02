//! Exact, bounded local artifact observations; never mount or extract an image.
use super::*;
use yoctui_model::{ImageArtifactView, PlatformFileKind, TEXTAREA_MAX_BYTES};

#[cfg(unix)]
fn open_image_file(root: &Path, path: &Path) -> Result<fs::File> {
    use std::os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    };
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC)
        .open(root)?;
    let parts = path.strip_prefix(root)?.components().collect::<Vec<_>>();
    for (index, part) in parts.iter().enumerate() {
        let name = std::ffi::CString::new(part.as_os_str().as_bytes())?;
        let leaf = index + 1 == parts.len();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if leaf {
                libc::O_NONBLOCK
            } else {
                libc::O_DIRECTORY
            };
        // Owned directory descriptor anchors each relative lookup; no symlink
        // ancestor race can redirect the file open outside the deploy tree.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(io::Error::last_os_error().into());
        }
        // openat returned a new descriptor whose sole owner is this File.
        let opened = unsafe { fs::File::from_raw_fd(fd) };
        if leaf {
            return Ok(opened);
        }
        directory = opened;
    }
    anyhow::bail!("Select an artifact file, not the deploy directory.")
}

#[cfg(not(unix))]
fn open_image_file(root: &Path, path: &Path) -> Result<fs::File> {
    if fs::symlink_metadata(path)?.file_type().is_symlink()
        || !path.canonicalize()?.starts_with(root.canonicalize()?)
    {
        anyhow::bail!("Artifact viewing does not follow symlinks or escape the deploy directory.");
    }
    Ok(fs::File::open(path)?)
}

fn inspect_image_artifact(
    root: &Path,
    path: &Path,
    dtc: Option<PathBuf>,
) -> Result<ImageArtifactView> {
    if !yoctui_utils::is_absolute_normal_path(path)
        || !yoctui_utils::is_absolute_normal_path(root)
        || !path.starts_with(root)
        || path == root
    {
        anyhow::bail!("Artifact path is outside the authoritative deploy directory.");
    }
    let root_metadata = fs::symlink_metadata(root)?;
    if !root_metadata.is_dir() || root_metadata.file_type().is_symlink() {
        anyhow::bail!("Deploy directory must be a real directory, not a symlink.");
    }
    // Reject symlink ancestors before opening, including nested associated files.
    let mut ancestor = root.to_path_buf();
    for part in path.strip_prefix(root)?.components() {
        ancestor.push(part);
        if fs::symlink_metadata(&ancestor)?.file_type().is_symlink() {
            anyhow::bail!("Artifact viewing does not follow symlinks.");
        }
    }
    let canonical_root = root.canonicalize()?;
    if !path.canonicalize()?.starts_with(&canonical_root) {
        anyhow::bail!("Artifact escapes the authoritative deploy directory.");
    }
    let file = open_image_file(root, path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        anyhow::bail!("Only regular artifact files can be viewed.");
    }
    let kind = match path.extension().and_then(|value| value.to_str()) {
        Some("dtb") => Some(PlatformFileKind::Dtb),
        Some("dtbo") => Some(PlatformFileKind::Dtbo),
        _ => None,
    };
    if let Some(kind) = kind {
        let program = dtc.context("dtc is unavailable. Install device-tree-compiler or add the native dtc tool to the initialized PATH.")?;
        return Ok(ImageArtifactView::DeviceTree {
            kind,
            program,
            size_bytes: metadata.len(),
        });
    }
    if metadata.len() > TEXTAREA_MAX_BYTES as u64 {
        anyhow::bail!(
            "Artifact exceeds the {TEXTAREA_MAX_BYTES}-byte text viewer limit. Use v for RootFS files or an explicit external tool."
        );
    }
    let mut bytes = Vec::new();
    file.take(TEXTAREA_MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > TEXTAREA_MAX_BYTES {
        anyhow::bail!("Artifact grew beyond the text viewer limit.");
    }
    let text = String::from_utf8(bytes).context("Binary artifact cannot be viewed as text. Use v to browse available IMAGE_ROOTFS files; disk images are not mounted.")?;
    if text
        .chars()
        .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        anyhow::bail!(
            "Binary/control-byte artifact cannot be viewed as text. Use v for available IMAGE_ROOTFS files."
        );
    }
    Ok(ImageArtifactView::Text(text))
}

pub(crate) async fn open_image_artifact(app: &mut App, root: PathBuf, path: PathBuf) {
    let checked_root = root.clone();
    let checked_path = path.clone();
    let result = tokio::task::spawn_blocking(move || {
        let dtc = terminal_launcher::executable_on_initialized_path("dtc")
            .and_then(|path| path.canonicalize().ok());
        inspect_image_artifact(&checked_root, &checked_path, dtc)
    })
    .await
    .map_err(|error| format!("Artifact viewer failed: {error}"))
    .and_then(|result| result.map_err(|error| format!("Could not view artifact: {error:#}")));
    let _ = update(app, Action::ImageArtifactViewed { root, path, result });
}

#[cfg(test)]
mod tests {
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
            ImageArtifact, ImageArtifactField as Field, ImageArtifactIdentity,
            ImageArtifactInventory, ImageArtifactInventoryState, ImageArtifactKind,
            ImageArtifactRequest,
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
}
