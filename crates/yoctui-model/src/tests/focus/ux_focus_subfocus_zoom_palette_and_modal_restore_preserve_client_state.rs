use super::*;

#[test]
fn ux_focus_subfocus_zoom_palette_and_modal_restore_preserve_client_state() {
    let mut app = App::new(32, 8_192);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    app.task_progress_scroll = 7;
    app.logs.scroll_offset = 11;
    let selections = (app.task_progress_scroll, app.logs.scroll_offset);

    let _ = update(&mut app, Action::CyclePaneSubfocus { backwards: false });
    assert_eq!(app.workspace_subfocus, WorkspaceSubfocus::Secondary);
    assert_eq!(app.pane_focus_label(), "Workspace/Secondary");
    let _ = update(&mut app, Action::TogglePaneZoom);
    assert_eq!(app.zoomed_pane, Some(FocusTarget::Workspace));

    let _ = update(&mut app, Action::OpenCommandPalette);
    assert_eq!(app.focus, FocusTarget::CommandPalette);
    assert_eq!(app.focus_return, Some(FocusTarget::Workspace));
    let _ = update(&mut app, Action::CloseCommandPalette);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.zoomed_pane, Some(FocusTarget::Workspace));
    assert_eq!(
        (app.task_progress_scroll, app.logs.scroll_offset),
        selections
    );

    let _ = update(&mut app, Action::OpenCommandPalette);
    app.command_palette_query = "focus workspace".into();
    app.command_palette_selection = 0;
    let _ = update(&mut app, Action::ActivateCommandPalette);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.zoomed_pane, Some(FocusTarget::Workspace));
    assert_eq!(
        crate::command_action(&app, CommandId::NextSubfocus),
        Action::CyclePaneSubfocus { backwards: false }
    );

    let _ = update(&mut app, Action::TogglePaneZoom);
    assert!(app.zoomed_pane.is_none());
    assert_eq!(
        (app.task_progress_scroll, app.logs.scroll_offset),
        selections
    );
}
