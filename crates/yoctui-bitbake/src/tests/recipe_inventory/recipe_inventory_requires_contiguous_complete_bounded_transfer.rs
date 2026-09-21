use super::*;

#[test]
fn recipe_inventory_requires_contiguous_complete_bounded_transfer() {
    let mut inventory = RecipeInventory::default();
    assert!(!inventory.push(0, 2, false, vec![recipe("α")]).unwrap());
    assert!(inventory.push(1, 2, true, vec![recipe("β")]).unwrap());
    assert!(inventory.push(2, 2, true, vec![]).is_err());
    assert_eq!(inventory.finish().unwrap(), vec![recipe("α"), recipe("β")]);
    assert!(RecipeInventory::default().finish().is_err());
    let mut empty = RecipeInventory::default();
    assert!(empty.push(0, 0, true, vec![]).unwrap());
    assert!(empty.finish().unwrap().is_empty());
}
