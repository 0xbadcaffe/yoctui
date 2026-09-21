use super::*;

#[test]
fn maintenance_optional_rejects_symlinked_helpers_and_escaped_manifests() {
    let fixture = TestDirectory::new("unsafe");
    let input = complete_fixture(&fixture);
    fs::rename(
        fixture.join("tools/create-pull-request"),
        fixture.join("real-create"),
    )
    .unwrap();
    symlink(
        fixture.join("real-create"),
        fixture.join("tools/create-pull-request"),
    )
    .unwrap();
    fs::remove_file(fixture.join("repo/.repo/manifest.xml")).unwrap();
    fs::write(fixture.join("outside.xml"), "<manifest/>\n").unwrap();
    symlink(
        fixture.join("outside.xml"),
        fixture.join("repo/.repo/manifest.xml"),
    )
    .unwrap();
    let inspection = MaintenanceOptionalCapabilityInspector::inspect(input).unwrap();
    assert!(
        !inspection
            .capability
            .supports(MaintenanceTool::CreatePullRequest)
    );
    assert_eq!(
        inspection.pull_request.state,
        OptionalIntegrationState::Partial
    );
    assert_eq!(
        inspection.repo_manifest.state,
        OptionalIntegrationState::Partial
    );
    assert!(
        inspection
            .limitations
            .iter()
            .any(|value| value.contains("unsafe executable"))
    );
    assert!(
        inspection
            .limitations
            .iter()
            .any(|value| value.contains("unsafe repo workspace"))
    );
}
