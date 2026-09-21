use super::*;

#[test]
fn wic_adapter_capability_crosses_app_boundary_without_parsing() {
    let capability = WicCapability::MissingKickstarts {
        executable: "/usr/bin/wic".into(),
    };
    assert_eq!(
        wic_capability_action(capability.clone()),
        Action::WicCapabilityLoaded(capability)
    );
}
