use super::*;

#[test]
fn global_search_discloses_limits_and_honors_cancellation() {
    let root = fixture_root();
    fs::write(root.join("large.txt"), "token\n".repeat(100)).unwrap();
    let plan = GlobalSearchPlan::for_app(&App::new(10, 1000), &root, "token".into());
    let cancellation = GlobalSearchCancellation::default();
    let result = scan_global_content(&plan, &cancellation).unwrap();
    assert_eq!(result.hits.len(), MAX_HITS_PER_CONTENT_KIND);
    assert!(result.truncated);
    cancellation.cancel();
    assert!(
        scan_global_content(&plan, &cancellation)
            .unwrap()
            .hits
            .is_empty()
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn global_search_parallel_workers_preserve_per_kind_hit_bounds() {
    let root = fixture_root();
    let files = [
        "rootfs/etc/data",
        "log.compile",
        "local.conf",
        "recipe.bb",
        "class.bbclass",
        "generated.txt",
    ];
    for file in files {
        let path = root.join(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "token\n".repeat(MAX_HITS_PER_CONTENT_KIND + 1)).unwrap();
    }
    let plan = GlobalSearchPlan::for_app(&App::new(10, 1000), &root, "token".into());
    let result = scan_global_content(&plan, &GlobalSearchCancellation::default()).unwrap();
    assert_eq!(result.hits.len(), MAX_HITS_PER_CONTENT_KIND * files.len());
    assert!(result.hits.len() <= yoctui_model::MAX_GLOBAL_SEARCH_HITS);
    assert!(result.truncated);
    fs::remove_dir_all(root).unwrap();
}
