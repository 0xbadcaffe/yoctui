use super::*;

#[test]
fn security_capability_preserves_current_and_legacy_reported_tasks() {
    let (_directory, current) = fixture(&["do_cve_check", "do_create_recipe_sbom"]);
    let current = SecurityCapabilityInspector::new(current).inspect().unwrap();
    assert_eq!(current.cve_task.as_deref(), Some("cve_check"));
    assert_eq!(
        current.recipe_sbom_task.as_deref(),
        Some("create_recipe_sbom")
    );
    assert!(current.mapper.is_some());
    assert_eq!(current.cve_roots.len(), 1);

    let (_directory, legacy) = fixture(&["do_cve_check", "do_create_spdx"]);
    let legacy = SecurityCapabilityInspector::new(legacy).inspect().unwrap();
    assert_eq!(legacy.recipe_sbom_task.as_deref(), Some("create_spdx"));
    assert_eq!(legacy.image_sbom_task.as_deref(), Some("create_spdx"));
}
