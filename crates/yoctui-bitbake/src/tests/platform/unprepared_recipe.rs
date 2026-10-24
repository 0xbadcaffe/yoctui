use super::*;

struct Directory(PathBuf);

impl Directory {
    fn new() -> Self {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            env::temp_dir().join(format!("yoctui-unprepared-{}-{nonce}", std::process::id()));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn missing_absolute_recipe_directories_remain_inspectable_without_creating_them() {
    let root = Directory::new();
    let source = root.0.join("unpacked-source");
    let build = root.0.join("build");
    let scan = PlatformArtifactAdapter
        .scan(vec![source.clone(), build.clone()])
        .unwrap();
    assert!(scan.roots.is_empty());
    assert!(scan.files.is_empty());
    assert!(
        scan.limitations
            .iter()
            .any(|line| line.contains("do not exist yet"))
    );
    assert!(!source.exists());
    assert!(!build.exists());
}

#[test]
fn absent_metadata_and_invalid_roots_are_not_treated_as_unprepared_recipes() {
    let root = Directory::new();
    let file = root.0.join("not-a-directory");
    fs::write(&file, "invalid root").unwrap();
    for candidates in [Vec::new(), vec!["relative/source".into()], vec![file]] {
        assert!(matches!(
            PlatformArtifactAdapter.scan(candidates),
            Err(PlatformArtifactAdapterError::NoRoots)
        ));
    }
}
