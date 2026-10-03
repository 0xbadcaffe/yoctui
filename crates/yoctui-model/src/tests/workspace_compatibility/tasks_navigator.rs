use super::*;
use crate::WorkspaceDestination;

#[test]
fn tasks_navigator_typed_lookup_roundtrips_existing_destinations_only() {
    let mut app = App::new(10, 1024);
    for destination in WorkspaceDestination::ALL {
        match App::navigator_selection_for_destination(destination) {
            Some(index) => {
                app.navigator_selection = index;
                assert_eq!(app.navigator_compatibility_destination(), destination);
            }
            None => assert!(matches!(
                destination,
                WorkspaceDestination::BuildHistory
                    | WorkspaceDestination::Signatures
                    | WorkspaceDestination::ProjectProfiles
                    | WorkspaceDestination::Help
            )),
        }
    }
    assert_ne!(
        App::navigator_selection_for_destination(WorkspaceDestination::Images),
        App::navigator_selection_for_destination(WorkspaceDestination::QemuWic)
    );
}
