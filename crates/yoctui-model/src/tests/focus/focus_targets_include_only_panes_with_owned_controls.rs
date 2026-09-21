use super::*;

#[test]
fn focus_targets_include_only_panes_with_owned_controls() {
    let mut app = App::new(32, 8_192);
    for passive in [Screen::Dashboard, Screen::LayerRelationships, Screen::Help] {
        app.screen = passive;
        assert_eq!(
            pane_focus_targets(&app).collect::<Vec<_>>(),
            [FocusTarget::Navigator]
        );
        let commands = app.command_palette_commands();
        assert_eq!(
            commands
                .iter()
                .find(|command| command.id == crate::CommandId::FocusWorkspace)
                .and_then(|command| command.disabled_reason.as_deref()),
            Some("The current Workspace is read-only")
        );
    }

    for interactive in [Screen::Tasks, Screen::Layers, Screen::TerminalSessions] {
        app.screen = interactive;
        assert_eq!(
            pane_focus_targets(&app).collect::<Vec<_>>(),
            [FocusTarget::Navigator, FocusTarget::Workspace]
        );
        assert!(
            app.command_palette_commands()
                .iter()
                .find(|command| command.id == crate::CommandId::FocusWorkspace)
                .is_some_and(|command| command.enabled())
        );
    }
    assert_eq!(
        app.command_palette_commands()
            .iter()
            .find(|command| command.id == crate::CommandId::FocusInspector)
            .and_then(|command| command.disabled_reason.as_deref()),
        Some("The Inspector is read-only")
    );
}
