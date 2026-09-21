use super::*;

#[test]
fn compatibility_devtool_previews_workspace_shell_and_authorized_exact_edit_recipe() {
    let (root, router, status) = fixture();
    let workspace = router
        .preview(
            &status,
            PtyDevtoolAction::WorkspaceShell {
                workspace_identity: "busybox-workspace".into(),
            },
        )
        .unwrap();
    assert_eq!(workspace.kind, PtySessionKind::DevtoolShell);
    assert_eq!(
        workspace.cwd,
        fs::canonicalize(root.join("workspace/busybox")).unwrap()
    );
    assert_eq!(workspace.command.arguments, vec!["-i"]);
    let edit = router
        .preview(&status, PtyDevtoolAction::EditRecipe)
        .unwrap();
    assert_eq!(edit.kind, PtySessionKind::InteractiveTool);
    assert_eq!(edit.command.arguments, vec!["edit-recipe", "busybox"]);
    assert_eq!(edit.cwd, fs::canonicalize(root.join("build")).unwrap());
    fs::remove_dir_all(root).unwrap();
}
