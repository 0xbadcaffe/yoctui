use super::super::scan_global_content_streaming;
use super::*;
use std::sync::Mutex;

#[test]
fn global_search_streams_before_completion_preserves_order_and_cancels_in_callback() {
    let root = fixture_root();
    fs::write(root.join("first.txt"), "token\n".repeat(50)).unwrap();
    let plan = GlobalSearchPlan::for_app(&App::new(10, 1000), &root, "token".into());
    let cancellation = GlobalSearchCancellation::default();
    let received = Mutex::new(Vec::new());
    let result = scan_global_content_streaming(&plan, &cancellation, &|hit| {
        let mut received = received.lock().unwrap();
        received.push(hit.clone());
        if received.len() == 3 {
            cancellation.cancel();
        }
    })
    .unwrap();
    assert!(
        cancellation.cancelled(),
        "callback must run during the scan, not after it"
    );
    assert_eq!(result.hits.len(), 3);
    assert_eq!(result.hits, *received.lock().unwrap());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn global_search_file_names_find_empty_and_binary_files_without_content_matches() {
    let root = fixture_root();
    fs::write(root.join("BOARD.dtb"), b"\0\xffbinary").unwrap();
    fs::write(root.join("board.txt"), "").unwrap();
    fs::write(root.join("different.txt"), "board").unwrap();
    fs::create_dir(root.join("downloads")).unwrap();
    fs::write(root.join("downloads/board.txt"), "").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(root.join("BOARD.dtb"), root.join("board-link.dtb")).unwrap();
    let mut app = App::new(10, 1000);
    app.global_search_target = yoctui_model::GlobalSearchTarget::FileNames;
    let plan = GlobalSearchPlan::for_app(&app, &root, "^board\\.(dtb|txt)$".into());
    let result = scan_global_content(&plan, &GlobalSearchCancellation::default()).unwrap();
    assert_eq!(result.hits.len(), 2);
    assert!(
        result
            .hits
            .iter()
            .all(|hit| hit.kind == GlobalSearchContentKind::FileName
                && hit.line == 1
                && hit.column == 1)
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn decompiled_dts_search_does_not_scan_neighbor_files_or_symlinks() {
    let root = fixture_root();
    fs::write(root.join("board.dts"), "model = \"needle\";\n").unwrap();
    fs::write(root.join("neighbor.dts"), "needle\n").unwrap();
    let mut plan = GlobalSearchPlan::for_app(&App::new(10, 1000), &root, "needle".into());
    plan.file = Some(root.join("board.dts"));
    let result = scan_global_content(&plan, &GlobalSearchCancellation::default()).unwrap();
    assert_eq!(result.hits.len(), 1);
    assert_eq!(result.hits[0].path, root.join("board.dts"));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("board.dts"), root.join("link.dts")).unwrap();
        plan.file = Some(root.join("link.dts"));
        assert!(
            scan_global_content(&plan, &GlobalSearchCancellation::default())
                .unwrap()
                .hits
                .is_empty()
        );
    }
    plan.file = Some(root.join("../outside.dts"));
    assert!(
        scan_global_content(&plan, &GlobalSearchCancellation::default())
            .unwrap()
            .hits
            .is_empty()
    );
    fs::remove_dir_all(root).unwrap();
}
