use super::*;

#[test]
fn utility_menu_catalog_exposes_typed_common_operations() {
    let catalog = utility_menu_catalog();
    assert!(catalog.iter().any(|entry| entry.operation == "status"));
    assert!(catalog.iter().any(|entry| entry.operation == "lookup-pkg"));
    assert!(catalog.iter().any(|entry| entry.destructive));
}
