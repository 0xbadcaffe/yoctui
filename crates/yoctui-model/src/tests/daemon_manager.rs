use crate::*;

#[test]
fn daemon_manager_navigation_menu_and_scrolling_stay_local_and_bounded() {
    let mut app = App::new(100, 100_000);
    assert_eq!(CommandId::OpenDaemons.screen(), Some(Screen::Daemons));
    assert_eq!(
        ApplicationMenuGroup::for_command(CommandId::OpenDaemons),
        ApplicationMenuGroup::Tools
    );
    assert!(matches!(
        workspace_destination_requirement(WorkspaceDestination::Daemons),
        WorkspaceEffectRequirement::ClientLocal
    ));
    app.screen = Screen::Daemons;
    app.daemon_manager.logs_visible = true;
    app.daemon_manager.visible_rows = 10;
    app.daemon_manager.logs = (0..80).map(|index| format!("log {index}")).collect();
    update(&mut app, Action::ScrollDaemonManager { delta: isize::MAX });
    assert_eq!(app.daemon_manager.scroll, 70);
    update(&mut app, Action::ScrollDaemonManager { delta: -1 });
    assert_eq!(app.daemon_manager.scroll, 69);
    update(&mut app, Action::ScrollDaemonManager { delta: isize::MIN });
    assert_eq!(app.daemon_manager.scroll, 0);
}
