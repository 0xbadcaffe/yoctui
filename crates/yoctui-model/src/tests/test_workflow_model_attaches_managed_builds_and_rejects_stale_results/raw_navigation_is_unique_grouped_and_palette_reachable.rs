use super::*;

#[test]
fn raw_navigation_is_unique_grouped_and_palette_reachable() {
    let raw_destinations = NAVIGATOR_SCREENS
        .iter()
        .enumerate()
        .filter_map(|(index, screen)| (*screen == Screen::RawMode).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(raw_destinations, [17]);
    assert_eq!(
        NAVIGATOR_COMPATIBILITY_DESTINATIONS[17],
        WorkspaceDestination::RawMode
    );
    assert_eq!(NAVIGATOR_GROUPS[4].label, "TOOLS");
    assert!((NAVIGATOR_GROUPS[4].start..NAVIGATOR_GROUPS[4].end).contains(&17));

    let mut app = App::new(16, 4096);
    let raw_commands = app
        .command_palette_commands()
        .into_iter()
        .filter(|command| command.id == CommandId::OpenRawMode)
        .collect::<Vec<_>>();
    assert_eq!(raw_commands.len(), 1);
    assert!(raw_commands[0].enabled());
    assert_eq!(
        command_action(&app, CommandId::OpenRawMode),
        Action::Open(Screen::RawMode)
    );

    assert_eq!(update(&mut app, Action::Open(Screen::RawMode)), None);
    assert_eq!(app.navigator_selection, 17);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert_eq!(app.inspector_mode(), InspectorMode::Navigator);
    assert_eq!(
        update(&mut app, Action::CycleFocus { backwards: false }),
        None
    );
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.inspector_mode(), InspectorMode::RawCommand);
}
