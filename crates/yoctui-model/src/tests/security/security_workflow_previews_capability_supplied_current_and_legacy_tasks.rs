use super::*;

#[test]
fn security_workflow_previews_capability_supplied_current_and_legacy_tasks() {
    let mut state = SecurityState::default();
    let _ = update_security(&mut state, SecurityAction::CapabilityLoaded(capability()));
    let transition = update_security(&mut state, SecurityAction::BeginCveCheck);
    let SecurityDialogUpdate::Open(SecurityDialog::Operation(cve)) = transition.dialog else {
        panic!("expected CVE preview");
    };
    assert!(matches!(
        &cve.operation,
        SecurityOperation::CveCheck(BuildRequest { task: Some(task), .. })
            if task == "cve_check"
    ));
    assert_eq!(cve.indexed_arguments[0], "0: bitbake");

    let transition = update_security(&mut state, SecurityAction::BeginSbomGeneration);
    let SecurityDialogUpdate::Open(SecurityDialog::Operation(sbom)) = transition.dialog else {
        panic!("expected SBOM preview");
    };
    assert!(matches!(
        &sbom.operation,
        SecurityOperation::SbomBuild(BuildRequest { task: Some(task), .. })
            if task == "create_recipe_sbom"
    ));

    let mut legacy = capability();
    legacy.recipe_sbom_task = Some("create_spdx".into());
    let _ = update_security(&mut state, SecurityAction::CapabilityLoaded(legacy));
    let transition = update_security(&mut state, SecurityAction::BeginSbomGeneration);
    assert!(matches!(
        transition.dialog,
        SecurityDialogUpdate::Open(SecurityDialog::Operation(SecurityOperationPreview {
            operation: SecurityOperation::SbomBuild(BuildRequest {
                task: Some(ref task),
                ..
            }),
            ..
        })) if task == "create_spdx"
    ));
}
