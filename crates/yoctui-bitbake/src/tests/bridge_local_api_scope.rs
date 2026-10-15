use super::*;

#[test]
fn configuration_inspection_scope_does_not_authorize_builds_or_recipe_inventory() {
    let scope = BridgeLocalApiScope::ConfigurationInspection;
    assert!(scope.allows(BitBakeApiOperation::Workspace));
    assert!(scope.allows(BitBakeApiOperation::Variable));
    for operation in [
        BitBakeApiOperation::Build,
        BitBakeApiOperation::Cancel,
        BitBakeApiOperation::ServerSocket,
        BitBakeApiOperation::Recipes,
    ] {
        assert!(!scope.allows(operation));
    }
}

#[test]
fn platform_inspection_scope_allows_only_read_only_metadata() {
    let scope = BridgeLocalApiScope::PlatformInspection;
    assert!(scope.allows(BitBakeApiOperation::Variable));
    assert!(scope.allows(BitBakeApiOperation::RecipeMetadata));
    assert!(!scope.allows(BitBakeApiOperation::Build));
    assert!(!scope.allows(BitBakeApiOperation::Cancel));
    assert!(!scope.allows(BitBakeApiOperation::ServerSocket));
}
