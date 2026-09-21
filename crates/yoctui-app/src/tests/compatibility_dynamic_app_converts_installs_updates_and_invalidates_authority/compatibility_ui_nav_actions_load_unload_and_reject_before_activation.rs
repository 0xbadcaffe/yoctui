use super::*;

#[test]
fn compatibility_ui_nav_actions_load_unload_and_reject_before_activation() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.workspace.build_dir = Some("/work/poky/build".into());

    let commands = app.command_palette_commands();
    let build = commands
        .iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    assert!(!build.enabled());
    assert_eq!(
        build.compatibility_state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );
    assert!(
        build
            .disabled_reason
            .as_deref()
            .unwrap()
            .contains("current environment capability snapshot")
    );
    let layers = commands
        .iter()
        .find(|command| command.id == yoctui_model::CommandId::OpenLayers)
        .unwrap();
    assert!(layers.enabled(), "navigation must remain discoverable");
    assert_eq!(
        layers.compatibility_state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );

    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(1))
        .unwrap();
    let build = app
        .command_palette_commands()
        .into_iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    assert!(build.enabled());
    assert_eq!(
        build.compatibility_state,
        yoctui_model::WorkspaceAvailabilityState::Available
    );
    assert_eq!(
        build.implementations,
        [(
            yoctui_model::CapabilityId::BitBakeBuild,
            "bitbake.build.command".into()
        )]
    );

    app.command_palette_open = true;
    app.focus = yoctui_model::FocusTarget::CommandPalette;
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ActivateCommandPalette,),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::BuildOptions)
    ));
    let _ = compatibility_workspace_action(&mut app, yoctui_model::Action::CloseBuildOptions);

    yoctui_model::invalidate_workspace_compatibility(&mut app);
    app.command_palette_open = true;
    app.focus = yoctui_model::FocusTarget::CommandPalette;
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ActivateCommandPalette,),
        None
    );
    assert!(app.active_dialog().is_none());
    assert!(app.command_palette_open);
}
