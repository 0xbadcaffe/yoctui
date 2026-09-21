use super::*;

#[test]
fn global_search_only_matches_build_text_contents_including_rootfs_and_artifacts() {
    let root = fixture_root();
    let build = root.join("build");
    let files = [
        "notes.txt",
        "conf/local.conf",
        "tmp/work/x/image/1/rootfs/etc/os-release",
        "tmp/deploy/images/x/image.manifest",
        "tmp-glibc/work/x/recipe/1/source/code.c",
    ];
    for file in files {
        let path = build.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "first line\ninside TOKEN text\n").unwrap();
    }
    fs::write(root.join("outside.bb"), "TOKEN").unwrap();
    fs::write(build.join("TOKEN-filename.txt"), "no match").unwrap();
    fs::write(build.join("binary.img"), b"TOKEN\0image").unwrap();
    fs::write(build.join("invalid.bin"), b"TOKEN\xff").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&root, build.join("escape")).unwrap();
    let mut app = App::new(10, 1000);
    app.workspace.source_dir = Some(root.clone());
    app.workspace.build_dir = Some(build.clone());
    let plan = GlobalSearchPlan::for_app(&app, &root, "token".into());
    let result = scan_global_content(&plan, &GlobalSearchCancellation::default()).unwrap();
    assert_eq!(result.hits.len(), files.len());
    assert!(
        result
            .hits
            .iter()
            .all(|hit| hit.line == 2 && hit.column == 8 && hit.path.starts_with(&build))
    );
    assert!(
        result
            .hits
            .iter()
            .any(|hit| hit.kind == GlobalSearchContentKind::ImageRootfs)
    );
    let empty = GlobalSearchPlan::for_app(&app, &build, "  ".into());
    let result = scan_global_content(&empty, &GlobalSearchCancellation::default()).unwrap();
    assert!(result.hits.is_empty() && result.searched_scopes.is_empty());
    let invalid = GlobalSearchPlan::for_app(&app, &build, "[".into());
    assert!(scan_global_content(&invalid, &GlobalSearchCancellation::default()).is_err());
    assert!(!safe_search_root(Path::new("/")));
    fs::remove_dir_all(root).unwrap();
}
