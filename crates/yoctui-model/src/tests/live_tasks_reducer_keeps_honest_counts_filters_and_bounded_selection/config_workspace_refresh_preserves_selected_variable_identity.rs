use super::*;

#[test]
fn config_workspace_refresh_preserves_selected_variable_identity() {
    let mut app = App::new(20, 4_000);
    app.workspace.variables.insert("A".into(), "one".into());
    app.workspace.variables.insert("B".into(), "two".into());
    app.config_selection = 1;
    let mut workspace = Workspace::default();
    workspace.variables.insert("B".into(), "updated".into());
    workspace.variables.insert("C".into(), "three".into());
    let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
    assert_eq!(app.config_selection, 0);
    assert_eq!(
        selected_config_identity(&app).map(|identity| identity.name),
        Some("B".into())
    );
}
