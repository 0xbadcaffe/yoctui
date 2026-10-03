use super::*;

#[test]
fn platform_inspection_scope_allows_only_read_only_metadata() {
    let scope = BridgeLocalApiScope::PlatformInspection;
    assert!(scope.allows(BitBakeApiOperation::Variable));
    assert!(scope.allows(BitBakeApiOperation::RecipeMetadata));
    assert!(!scope.allows(BitBakeApiOperation::Build));
    assert!(!scope.allows(BitBakeApiOperation::Cancel));
    assert!(!scope.allows(BitBakeApiOperation::ServerSocket));
}
