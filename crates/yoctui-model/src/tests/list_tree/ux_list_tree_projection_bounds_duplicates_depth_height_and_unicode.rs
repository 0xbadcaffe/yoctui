use super::*;

#[test]
fn ux_list_tree_projection_bounds_duplicates_depth_height_and_unicode() {
    let mut rows = vec![ListTreeRow {
        id: "猫".into(),
        label: "レイヤー".into(),
        depth: usize::MAX,
        branch: true,
        expanded: true,
        height: usize::MAX,
    }];
    rows.push(rows[0].clone());
    rows.extend(
        (0..LIST_TREE_MAX_ROWS + 3)
            .map(|index| ListTreeRow::new(format!("id:{index}"), format!("row {index}"), 0)),
    );
    let tree = ListTreeProjection::new(rows);
    assert_eq!(tree.rows().len(), LIST_TREE_MAX_ROWS);
    assert_eq!(tree.limitations.duplicate_ids, 1);
    assert!(tree.limitations.omitted_rows > 0);
    assert_eq!(tree.rows()[0].depth, LIST_TREE_MAX_DEPTH);
    assert_eq!(tree.rows()[0].height, LIST_TREE_MAX_ROW_HEIGHT);
    assert!(list_tree_text(&tree.rows()[0], true, true).contains("▾ レイヤー"));
    assert!(list_tree_text(&tree.rows()[0], true, false).contains("- レイヤー"));
}
