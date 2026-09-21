use super::*;

#[test]
fn ux_terminal_runtime_preserves_explicit_local_build_authority() {
    let local = PathBuf::from("/configured/build");
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = None;
    restore_local_build_dir(&mut app, Some(&local));
    assert_eq!(app.workspace.build_dir.as_deref(), Some(local.as_path()));

    app.workspace.build_dir = Some("/daemon/build".into());
    restore_local_build_dir(&mut app, Some(&local));
    assert_eq!(
        app.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/daemon/build"))
    );
}
