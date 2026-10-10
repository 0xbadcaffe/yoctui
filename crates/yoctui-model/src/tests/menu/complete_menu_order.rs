use super::*;

#[test]
fn menus_cover_every_screen_once_and_keep_common_actions_first() {
    let app = App::new(10, 1024);
    let workspace = app.application_menu_items(ApplicationMenuGroup::Workspace);
    assert_eq!(
        workspace.iter().map(|item| item.label).collect::<Vec<_>>(),
        [
            "Open Build Environment",
            "Open Compatibility",
            "Open Terminal Sessions",
            "Command palette",
            "Quit"
        ]
    );
    assert!(workspace.iter().all(MenuItem::enabled));
    assert_eq!(
        app.application_menu_items(ApplicationMenuGroup::View)[0].label,
        "Preferences"
    );
    let navigation = app.application_menu_items(ApplicationMenuGroup::Navigate);
    let screens = navigation
        .iter()
        .filter_map(|item| match item.target {
            OperatorActionTarget::Command(command) => match crate::command_action(&app, command) {
                Action::Open(screen) => Some(screen),
                _ => None,
            },
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut expected = MENU_SCREENS.to_vec();
    expected.push(Screen::Daemons);
    assert_eq!(screens, expected);
    assert!(
        app.application_menu_items(ApplicationMenuGroup::Tools)
            .iter()
            .any(|item| item.target == OperatorActionTarget::Command(CommandId::OpenDaemons))
    );
    for definition in crate::global_operator_action_definitions() {
        let OperatorActionTarget::Command(command) = definition.target else {
            unreachable!()
        };
        let group = ApplicationMenuGroup::for_command(command);
        assert_eq!(definition.menu_path[0], group.label());
        if let Action::Open(screen) = crate::command_action(&app, command) {
            assert_eq!(command.screen(), Some(screen));
        } else {
            assert_eq!(command.screen(), None);
        }
        assert!(
            app.application_menu_items(group)
                .iter()
                .any(|item| item.target == definition.target),
            "missing {:?}",
            command
        );
    }
    let config = app.application_menu_items(ApplicationMenuGroup::Configuration);
    assert_eq!(config[0].label, "Open Configuration");
    assert_eq!(config[1].label, "Open BBMASK");
    assert_eq!(config[2].label, "Edit BBMASK");
    assert!(!config[2].enabled(), "editing still needs a workspace");
}

#[test]
fn menu_quit_keeps_confirmation_and_missing_destinations_remain_inspectable_offline() {
    let mut app = App::new(10, 1024);
    let quit = crate::command_action(&app, crate::CommandId::Quit);
    crate::update(&mut app, quit);
    assert!(matches!(
        app.active_dialog(),
        Some(crate::Dialog::QuitConfirmation)
    ));
    assert!(!app.should_quit);
    crate::update(&mut app, Action::CancelQuit);
    for command in [
        crate::CommandId::OpenInsights,
        crate::CommandId::OpenBuildHistory,
        crate::CommandId::OpenKernel,
        crate::CommandId::OpenFirmware,
        crate::CommandId::OpenSignatures,
        crate::CommandId::OpenLayerRelationships,
        crate::CommandId::OpenDevtoolWorkspace,
        crate::CommandId::OpenBbmask,
    ] {
        let action = crate::command_action(&app, command);
        let Action::Open(screen) = action else {
            panic!("missing navigation route")
        };
        crate::update_with_workspace_authority(&mut app, action);
        assert_eq!(app.screen, screen);
        assert!(!app.should_quit);
    }
}
