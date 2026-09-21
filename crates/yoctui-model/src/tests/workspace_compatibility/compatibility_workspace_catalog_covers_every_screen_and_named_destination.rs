use super::*;

#[test]
fn compatibility_workspace_catalog_covers_every_screen_and_named_destination() {
    assert_eq!(WorkspaceDestination::ALL.len(), 28);
    for screen in [
        Screen::Dashboard,
        Screen::Insights,
        Screen::Tasks,
        Screen::BuildHistory,
        Screen::Dependencies,
        Screen::Signatures,
        Screen::LayerRelationships,
        Screen::Recipes,
        Screen::Packages,
        Screen::Images,
        Screen::Kernel,
        Screen::Firmware,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Qa,
        Screen::RawMode,
        Screen::Layers,
        Screen::Configuration,
        Screen::Bbmask,
        Screen::Maintenance,
        Screen::Logs,
        Screen::Errors,
        Screen::Help,
        Screen::BuildEnvironment,
        Screen::Compatibility,
        Screen::Settings,
    ] {
        let _ = workspace_screen_destination(screen);
    }
    assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::Devtool));
    assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::QemuWic));
    assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::ProjectProfiles));
    assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::TerminalSessions));
    assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::Compatibility));
    assert!(WorkspaceDestination::ALL.contains(&WorkspaceDestination::RawMode));
    for destination in WorkspaceDestination::ALL {
        let _ = workspace_destination_requirement(destination);
    }
}
