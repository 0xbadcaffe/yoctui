use super::*;

#[test]
fn pty_devtool_rejects_stale_status_workspace_and_recipe() {
    let (root, router, mut status) = fixture();
    status.error = Some(DevtoolStatusError::MalformedOutput { line: "bad".into() });
    assert_eq!(
        router.preview(&status, PtyDevtoolAction::EditRecipe),
        Err(PtyDevtoolError::StaleStatus)
    );
    status.error = None;
    status.identity.name = "bad recipe;touch".into();
    assert_eq!(
        router.preview(&status, PtyDevtoolAction::EditRecipe),
        Err(PtyDevtoolError::InvalidRecipe)
    );
    status.identity.name = "busybox".into();
    status.workspace = DevtoolWorkspace::Present {
        source_path: root.join("source"),
        recipe_file: None,
    };
    assert_eq!(
        router.preview(
            &status,
            PtyDevtoolAction::WorkspaceShell {
                workspace_identity: "busybox-workspace".into()
            }
        ),
        Err(PtyDevtoolError::StaleWorkspace)
    );
    fs::remove_dir_all(root).unwrap();
}
