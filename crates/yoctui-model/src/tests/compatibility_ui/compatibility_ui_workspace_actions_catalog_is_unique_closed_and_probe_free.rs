use super::*;

#[test]
fn compatibility_ui_workspace_actions_catalog_is_unique_closed_and_probe_free() {
    let mut ids = std::collections::BTreeSet::new();
    for destination in crate::WorkspaceDestination::ALL {
        for action in compatibility_ui_workspace_action_definitions(destination) {
            assert!(ids.insert(action.id), "duplicate action ID {}", action.id);
            assert!(!action.label.is_empty());
            assert!(!action.shortcut.is_empty());
            assert!(!matches!(
                action.requirement,
                WorkspaceEffectRequirement::DaemonProbe { .. }
            ));
        }
    }
    for expected in [
        "dashboard.build",
        "recipes.devtool_modify",
        "recipes.devtool_gitui",
        "layers.remove",
        "configuration.getvar",
        "tasks.cancel",
        "dependencies.refresh",
        "signatures.compare",
        "packages.detail",
        "images.qemu",
        "sdk.extensible",
        "testing.oe_selftest",
        "security.spdx",
        "qa.layer",
        "devtool.upgrade",
        "devtool.gitui",
        "devtool.shell",
        "devtool.build",
        "qemu_wic.wic",
        "maintenance.cleanup",
        "terminal.menuconfig",
    ] {
        assert!(
            ids.contains(expected),
            "missing contextual action {expected}"
        );
    }
}
