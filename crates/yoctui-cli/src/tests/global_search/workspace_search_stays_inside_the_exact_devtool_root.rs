use super::*;

#[test]
fn workspace_search_stays_inside_the_exact_devtool_root() {
    let parent = fixture_root();
    let workspace = parent.join("workspace/sources/busybox");
    fs::create_dir_all(workspace.join("src")).unwrap();
    fs::create_dir_all(workspace.join(".git")).unwrap();
    fs::write(workspace.join("src/main.c"), "int workspace_needle;\n").unwrap();
    fs::write(workspace.join(".git/index.txt"), "workspace_needle\n").unwrap();
    fs::write(parent.join("outside.c"), "workspace_needle\n").unwrap();

    let plan = GlobalSearchPlan {
        query: "workspace_needle".into(),
        build_dir: Some(workspace.clone()),
        scope_label: "workspace".into(),
    };
    let result = scan_global_content(&plan, &GlobalSearchCancellation::default()).unwrap();
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].path, workspace.join("src/main.c"));
    assert_eq!(
        result.searched_scopes,
        vec![format!("workspace={}", workspace.display())]
    );
    fs::remove_dir_all(parent).unwrap();
}
