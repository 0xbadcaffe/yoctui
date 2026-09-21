use super::*;

#[test]
fn configuration_selection_stays_in_workspace_bounds() {
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    let _ = update(&mut app, Action::SelectConfigVariable { delta: 8 });
    assert_eq!(app.config_selection, 1);
    let _ = update(&mut app, Action::SelectConfigVariable { delta: -8 });
    assert_eq!(app.config_selection, 0);
}
