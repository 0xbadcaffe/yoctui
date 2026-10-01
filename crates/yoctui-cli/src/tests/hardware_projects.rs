use super::*;

#[test]
fn hardware_project_preview_revalidates_registration_and_format_on_disk() {
    let temporary = fixture("preview-root");
    let root = temporary.join("store");
    let created = project(
        execute(
            &root,
            HardwareProjectOperation::Create {
                name: "board".into(),
            },
        )
        .unwrap(),
    );
    let txt = created.root.join("notes.txt");
    fs::write(&txt, "notes").unwrap();
    assert!(validate_preview_in_store(&root, &created.root, &txt).is_ok());
    let binary = created.root.join("firmware.bin");
    fs::write(&binary, "stored only").unwrap();
    assert!(validate_preview_in_store(&root, &created.root, &binary).is_err());
    let foreign = temporary.join("foreign.txt");
    fs::write(&foreign, "outside").unwrap();
    assert!(validate_preview_in_store(&root, &created.root, &foreign).is_err());
    fs::remove_dir_all(temporary).unwrap();
}

fn fixture(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "yoctui-hardware-project-{label}-{}-{}",
        std::process::id(),
        NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir(&path).unwrap();
    path
}

fn project(result: HardwareProjectResult) -> HardwareProject {
    match result {
        HardwareProjectResult::Directory { project, .. }
        | HardwareProjectResult::Progress(project) => project,
        _ => panic!("expected project"),
    }
}

#[test]
fn hardware_project_real_folders_arbitrary_files_and_progress_survive_restart() {
    let temporary = fixture("roundtrip");
    let root = temporary.join("store");
    let created = project(
        execute(
            &root,
            HardwareProjectOperation::Create {
                name: "Board Alpha".into(),
            },
        )
        .unwrap(),
    );
    assert!(created.root.is_dir());
    execute(
        &root,
        HardwareProjectOperation::CreateFolder {
            name: created.name.clone(),
            relative: PathBuf::new(),
            folder: "Schematics".into(),
        },
    )
    .unwrap();
    let source = temporary.join("firmware.bin");
    fs::write(&source, [0, 1, 255]).unwrap();
    let imported = execute(
        &root,
        HardwareProjectOperation::Import {
            name: created.name.clone(),
            relative: "Schematics".into(),
            source: source.clone(),
        },
    )
    .unwrap();
    let HardwareProjectResult::Directory { entries, .. } = imported else {
        panic!()
    };
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].kind, None);
    assert_eq!(entries[0].size, 3);
    assert_eq!(fs::read(&entries[0].path).unwrap(), [0, 1, 255]);
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Import {
                name: created.name.clone(),
                relative: "Schematics".into(),
                source
            }
        )
        .is_err()
    );
    let updated = project(
        execute(
            &root,
            HardwareProjectOperation::SaveProgress {
                name: created.name.clone(),
                progress: [100, 50, 25, 0, 100, 25],
            },
        )
        .unwrap(),
    );
    assert_eq!(updated.percent(), 50);
    let HardwareProjectResult::Catalog(catalog) =
        execute(&root, HardwareProjectOperation::List).unwrap()
    else {
        panic!()
    };
    assert_eq!(catalog, vec![updated]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(created.root.join(MANIFEST))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
    assert!(fs::read_dir(&created.root).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".yoctui-save-")
    }));
    fs::remove_dir_all(temporary).unwrap();
}

#[test]
fn hardware_project_refuses_escape_collisions_bad_progress_and_large_imports() {
    let temporary = fixture("limits");
    let root = temporary.join("store");
    for name in [
        "../escape",
        "/absolute",
        ".",
        "..",
        ".yoctui-project.toml",
        "",
    ] {
        assert!(
            execute(
                &root,
                HardwareProjectOperation::Create { name: name.into() }
            )
            .is_err()
        );
    }
    let created = project(
        execute(
            &root,
            HardwareProjectOperation::Create {
                name: "board".into(),
            },
        )
        .unwrap(),
    );
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Create {
                name: "board".into()
            }
        )
        .is_err()
    );
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Directory {
                name: "board".into(),
                relative: "..".into()
            }
        )
        .is_err()
    );
    assert!(
        execute(
            &root,
            HardwareProjectOperation::SaveProgress {
                name: "board".into(),
                progress: [101; 6]
            }
        )
        .is_err()
    );
    assert_eq!(read_project(&root, "board").unwrap().progress, [0; 6]);
    let large = temporary.join("large.bin");
    fs::File::create(&large)
        .unwrap()
        .set_len(MAX_HARDWARE_IMPORT_BYTES + 1)
        .unwrap();
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Import {
                name: "board".into(),
                relative: PathBuf::new(),
                source: large
            }
        )
        .is_err()
    );
    assert!(!created.root.join("large.bin").exists());
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Import {
                name: "board".into(),
                relative: PathBuf::new(),
                source: temporary.join("missing")
            }
        )
        .is_err()
    );
    assert!(!created.root.join("missing").exists());
    fs::remove_dir_all(temporary).unwrap();
}

#[test]
#[cfg(unix)]
fn hardware_project_rejects_symlink_roots_folders_files_and_manifests() {
    use std::os::unix::fs::symlink;
    let temporary = fixture("links");
    let root = temporary.join("store");
    let created = project(
        execute(
            &root,
            HardwareProjectOperation::Create {
                name: "board".into(),
            },
        )
        .unwrap(),
    );
    let outside = temporary.join("outside");
    fs::create_dir(&outside).unwrap();
    symlink(&outside, created.root.join("escape")).unwrap();
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Directory {
                name: "board".into(),
                relative: "escape".into()
            }
        )
        .is_err()
    );
    let source = temporary.join("note.txt");
    fs::write(&source, "note").unwrap();
    let link = temporary.join("link.txt");
    symlink(&source, &link).unwrap();
    assert!(
        execute(
            &root,
            HardwareProjectOperation::Import {
                name: "board".into(),
                relative: PathBuf::new(),
                source: link
            }
        )
        .is_err()
    );
    fs::remove_file(created.root.join(MANIFEST)).unwrap();
    symlink(&source, created.root.join(MANIFEST)).unwrap();
    assert!(read_project(&root, "board").is_err());
    symlink(&root, temporary.join("linked-store")).unwrap();
    assert!(
        execute(
            &temporary.join("linked-store"),
            HardwareProjectOperation::List
        )
        .is_err()
    );
    fs::remove_dir_all(temporary).unwrap();
}

#[test]
fn hardware_project_import_browser_lists_any_regular_file_and_view_policy_is_restricted() {
    let temporary = fixture("browser");
    for name in [
        "notes.TXT",
        "board.SchDoc",
        "sheet.1",
        "board.pdf",
        "board.kicad_sch",
        "data.bin",
        "image.png",
    ] {
        fs::write(temporary.join(name), "test").unwrap();
    }
    let HardwareProjectResult::ImportBrowser { entries, .. } = execute(
        &temporary,
        HardwareProjectOperation::ImportBrowse {
            directory: temporary.clone(),
        },
    )
    .unwrap() else {
        panic!()
    };
    assert_eq!(entries.len(), 7);
    assert!(
        entries
            .iter()
            .find(|entry| entry.name == "data.bin")
            .unwrap()
            .kind
            .is_none()
    );
    assert!(
        entries
            .iter()
            .find(|entry| entry.name == "image.png")
            .unwrap()
            .kind
            .is_none()
    );
    assert_eq!(
        entries
            .iter()
            .find(|entry| entry.name == "board.SchDoc")
            .unwrap()
            .kind,
        Some(HardwareDocumentKind::Altium)
    );
    fs::remove_dir_all(temporary).unwrap();
}
