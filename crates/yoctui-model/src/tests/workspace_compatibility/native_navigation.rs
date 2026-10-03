use super::*;
use crate::{Action, ClientReplicaStatus, DaemonModelInstanceId, FocusTarget, Screen, update};

const DESTINATIONS: [Screen; 4] = [Screen::Sdk, Screen::Testing, Screen::Security, Screen::Qa];

fn native_app() -> App {
    let mut app = App::new(16, 4096);
    app.daemon.instance_id = Some(DaemonModelInstanceId([42; 16]));
    app.daemon.status = ClientReplicaStatus::Current;
    app
}

fn assert_not_probed(app: &App) {
    assert_eq!(
        app.sdk_tool_capability,
        crate::SdkToolCapability::NotInspected
    );
    assert_eq!(
        app.test_capability.oe_selftest,
        crate::TestExecutableCapability::NotInspected
    );
    assert_eq!(
        app.result_tool_capability,
        crate::ResultToolCapability::NotInspected
    );
    assert_eq!(
        app.security.capability,
        crate::SecurityCapability::NotInspected
    );
    assert_eq!(app.qa.capability, crate::QaCapability::NotInspected);
    assert_eq!(
        app.maintenance.capability,
        crate::MaintenanceCapability::NotInspected
    );
    assert!(app.daemon.jobs.is_empty());
    assert!(app.daemon.pty_sessions.is_empty());
}

#[test]
fn native_navigation_keeps_screen_changes_before_and_after_capability_discovery() {
    for status in [
        ClientReplicaStatus::Current,
        ClientReplicaStatus::Synchronizing,
        ClientReplicaStatus::Stale,
        ClientReplicaStatus::Disconnected,
    ] {
        for snapshot in [
            None,
            Some(authority(1, vec![])),
            Some(authority(
                1,
                vec![(
                    CapabilityId::OeSelftest,
                    CapabilityState::Available,
                    Some("fixture"),
                )],
            )),
            Some(authority(
                1,
                vec![(
                    CapabilityId::OeSelftest,
                    CapabilityState::Unknown {
                        reason: reason("unknown"),
                    },
                    None,
                )],
            )),
            Some(authority(
                1,
                vec![(
                    CapabilityId::OeSelftest,
                    CapabilityState::Unavailable {
                        reason: reason("missing"),
                    },
                    None,
                )],
            )),
        ] {
            for destination in DESTINATIONS {
                let mut app = native_app();
                app.daemon.status = status;
                if let Some(snapshot) = snapshot.clone() {
                    install_workspace_compatibility(&mut app, snapshot).unwrap();
                }
                assert!(
                    update_with_workspace_authority(&mut app, Action::Open(destination)).is_none()
                );
                assert_eq!(app.screen, destination, "{status:?}: {destination:?}");
                assert_eq!(app.focus, FocusTarget::Navigator);
                assert!(app.notification.is_none(), "{:?}", app.notification);
                assert_not_probed(&app);
            }
        }
    }
}

#[test]
fn native_navigation_navigator_and_palette_close_without_probe_or_rollback() {
    for (destination, query) in [
        (Screen::Sdk, "Open SDK"),
        (Screen::Testing, "Open Testing"),
        (Screen::Security, "Open Security"),
        (Screen::Qa, "Open QA"),
    ] {
        let mut app = native_app();
        let _ = update(&mut app, Action::Open(destination));
        let selection = app.navigator_selection;
        let _ = update(&mut app, Action::Open(Screen::Dashboard));
        let _ = update(&mut app, Action::SelectNavigatorAt { index: selection });
        assert!(update_with_workspace_authority(&mut app, Action::ActivateNavigator).is_none());
        assert_eq!(app.screen, destination);
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert_not_probed(&app);

        let mut app = native_app();
        let _ = update(&mut app, Action::OpenCommandPalette);
        for character in query.chars() {
            let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
        }
        assert!(
            update_with_workspace_authority(&mut app, Action::ActivateCommandPalette).is_none()
        );
        assert_eq!(app.screen, destination);
        assert!(!app.command_palette_open);
        assert_not_probed(&app);
    }
}

#[test]
fn native_navigation_still_denies_explicit_probes_without_mutating_state() {
    for action in [
        Action::InspectTestCapability,
        Action::Security(crate::SecurityAction::InspectCapability),
        Action::Qa(crate::QaAction::InspectCapability),
    ] {
        let mut app = native_app();
        assert!(
            update_with_workspace_authority(&mut app, action.clone()).is_none(),
            "{action:?}"
        );
        assert_eq!(app.screen, Screen::Dashboard);
        assert_not_probed(&app);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("Environment probing is daemon-owned")
        );
    }
}

#[test]
fn native_navigation_preserves_non_daemon_automatic_probe_effects() {
    for destination in DESTINATIONS {
        let mut app = App::new(16, 4096);
        let effect = update(&mut app, Action::Open(destination)).unwrap();
        assert!(
            matches!(
                workspace_effect_requirement(&effect),
                WorkspaceEffectRequirement::DaemonProbe { .. }
            ),
            "{destination:?}: {effect:?}"
        );
        assert_eq!(app.screen, destination);
    }
}

#[test]
fn native_navigation_preserves_client_local_maintenance_inspection() {
    let mut legacy = App::new(16, 4096);
    let expected = update(&mut legacy, Action::Open(Screen::Maintenance));
    let mut native = native_app();
    let actual = update_with_workspace_authority(&mut native, Action::Open(Screen::Maintenance));
    assert_eq!(actual, expected);
    assert!(matches!(
        actual,
        Some(Effect::Maintenance(
            crate::MaintenanceEffect::InspectCapability { .. }
        ))
    ));
    assert_eq!(native.maintenance.capability, legacy.maintenance.capability);
    assert_eq!(native.screen, Screen::Maintenance);
}

#[test]
fn native_navigation_menu_commands_keep_the_navigation_transition() {
    for (destination, command) in [
        (Screen::Sdk, crate::CommandId::OpenSdk),
        (Screen::Testing, crate::CommandId::OpenTesting),
        (Screen::Security, crate::CommandId::OpenSecurity),
        (Screen::Qa, crate::CommandId::OpenQa),
    ] {
        let mut app = native_app();
        let _ = update(&mut app, Action::OpenApplicationMenu);
        let item = app
            .application_menu_items(crate::ApplicationMenuGroup::for_command(command))
            .into_iter()
            .find(|item| item.target == crate::OperatorActionTarget::Command(command))
            .unwrap();
        assert!(item.enabled());
        let _ = update(&mut app, Action::CloseMenu);
        let action = crate::command_action(&app, command);
        assert!(update_with_workspace_authority(&mut app, action).is_none());
        assert_eq!(app.screen, destination);
        assert!(!app.menu.is_open());
        assert_not_probed(&app);
    }
}

#[test]
fn native_navigation_authority_only_context_also_avoids_client_probes() {
    for destination in DESTINATIONS {
        let mut app = App::new(16, 4096);
        install_workspace_compatibility(&mut app, authority(1, vec![])).unwrap();
        assert!(app.daemon.instance_id.is_none());
        assert!(update_with_workspace_authority(&mut app, Action::Open(destination)).is_none());
        assert_eq!(app.screen, destination);
        assert_not_probed(&app);
    }
}
