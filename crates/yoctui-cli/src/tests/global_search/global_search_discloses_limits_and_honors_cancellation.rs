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
