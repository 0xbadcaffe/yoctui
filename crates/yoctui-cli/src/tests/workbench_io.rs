use super::*;
use yoctui_model::TextAreaRevision;

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let path = root.join("recipe.bb");
    fs::write(&path, "SUMMARY = \"old\"\n").unwrap();
    (temp, root, path)
}

#[test]
fn workbench_reads_extensionless_text_and_rejects_binary_and_oversize() {
    let (_temp, root, _) = fixture();
    for (name, bytes, valid) in [
        ("Makefile", b"all:\n\techo hi\n".to_vec(), true),
        ("binary", vec![0, 1, 2], false),
        (
            "large",
            vec![b'x'; yoctui_model::TEXTAREA_MAX_BYTES + 1],
            false,
        ),
    ] {
        let path = root.join(name);
        fs::write(&path, bytes).unwrap();
        let W::FileLoaded { result, .. } = execute(R::Read {
            session: 1,
            root: root.clone(),
            path,
            generation: 1,
        }) else {
            panic!()
        };
        assert_eq!(result.is_ok(), valid, "{name}");
    }
}

#[test]
fn workbench_save_is_atomic_and_detects_external_changes() {
    let (_temp, root, path) = fixture();
    let original = fs::read_to_string(&path).unwrap();
    let save = |text: &str, expected| {
        execute(R::Save {
            session: 1,
            root: root.clone(),
            path: path.clone(),
            content: text.into(),
            expected,
        })
    };
    assert!(matches!(
        save("new\n", TextAreaRevision::of(&original)),
        W::Saved { result: Ok(()), .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
    assert!(matches!(
        save("overwrite\n", TextAreaRevision::of(&original)),
        W::Saved { result: Err(_), .. }
    ));
    assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
}

#[cfg(unix)]
#[test]
fn workbench_rejects_symlink_escape_and_preserves_file_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (_temp, root, path) = fixture();
    let outside = tempfile::tempdir().unwrap();
    let external = outside.path().join("private");
    fs::write(&external, "secret").unwrap();
    let link = root.join("escape");
    symlink(&external, &link).unwrap();
    assert!(matches!(
        execute(R::Read {
            session: 1,
            root: root.clone(),
            path: link,
            generation: 1
        }),
        W::FileLoaded { result: Err(_), .. }
    ));
    fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
    let old = fs::read_to_string(&path).unwrap();
    assert!(matches!(
        execute(R::Save {
            session: 1,
            root,
            path: path.clone(),
            content: "updated".into(),
            expected: TextAreaRevision::of(&old)
        }),
        W::Saved { result: Ok(()), .. }
    ));
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o640
    );
}

#[test]
fn workbench_create_never_overwrites_and_scan_is_one_directory() {
    let (_temp, root, path) = fixture();
    assert!(matches!(
        execute(R::NewFile {
            session: 1,
            root: root.clone(),
            path
        }),
        W::NewFileCreated { result: Err(_), .. }
    ));
    fs::create_dir(root.join("src")).unwrap();
    fs::write(root.join("src/hidden.c"), "int x;").unwrap();
    let W::DirectoryLoaded {
        result: Ok(entries),
        ..
    } = execute(R::Directory {
        session: 1,
        root: root.clone(),
        directory: root.clone(),
        generation: 1,
    })
    else {
        panic!()
    };
    assert!(
        entries
            .iter()
            .any(|e| e.path == root.join("src") && e.is_dir)
    );
    assert!(!entries.iter().any(|e| e.path.ends_with("hidden.c")));
}

#[test]
fn workbench_store_roundtrips_and_corrupt_store_is_retained() {
    let (_temp, root, _) = fixture();
    let path = root.join("state/workbenches.json");
    save(&path, &SavedWorkbenches::default()).unwrap();
    assert!(load(&path).unwrap().sessions.is_empty());
    fs::write(&path, "not json").unwrap();
    assert!(load(&path).is_err());
    assert_eq!(fs::read_to_string(path).unwrap(), "not json");
}
