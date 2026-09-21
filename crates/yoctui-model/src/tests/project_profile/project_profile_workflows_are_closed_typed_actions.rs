use super::*;

#[test]
fn project_profile_workflows_are_closed_typed_actions() {
    let steps = &profile().workflows[0].steps;
    assert!(matches!(steps[0], ProjectWorkflowStep::RefreshMetadata));
    assert!(!format!("{steps:?}").contains("command"));
}
