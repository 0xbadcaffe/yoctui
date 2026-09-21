use super::*;

#[test]
fn recipe_inventory_rejects_bad_offsets_totals_final_flags_and_bounds() {
    for (offset, total, complete, rows) in [
        (1, 2, true, vec![recipe("x")]),
        (0, 2, true, vec![recipe("x")]),
        (0, 1, false, vec![recipe("x")]),
        (0, 1, false, vec![]),
        (
            0,
            MAX_RECIPE_INVENTORY_RECORDS + 1,
            false,
            vec![recipe("x")],
        ),
        (
            0,
            1,
            true,
            vec![recipe(&"x".repeat(MAX_RECIPE_INVENTORY_BYTES))],
        ),
    ] {
        assert!(
            RecipeInventory::default()
                .push(offset, total, complete, rows)
                .is_err()
        );
    }
    let mut changed = RecipeInventory::default();
    changed.push(0, 3, false, vec![recipe("x")]).unwrap();
    assert!(changed.push(1, 2, true, vec![recipe("y")]).is_err());
    let mut duplicate = RecipeInventory::default();
    duplicate.push(0, 2, false, vec![recipe("x")]).unwrap();
    assert!(duplicate.push(0, 2, false, vec![recipe("x")]).is_err());
}
