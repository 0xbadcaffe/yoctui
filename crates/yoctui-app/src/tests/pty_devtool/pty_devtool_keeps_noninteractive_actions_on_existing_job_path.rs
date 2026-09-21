use super::*;

#[test]
fn pty_devtool_keeps_noninteractive_actions_on_existing_job_path() {
    let (root, router, status) = fixture();
    for action in [
        PtyDevtoolAction::Modify,
        PtyDevtoolAction::UpdateRecipe,
        PtyDevtoolAction::Finish,
        PtyDevtoolAction::Deploy,
        PtyDevtoolAction::Reset,
    ] {
        assert_eq!(
            router.preview(&status, action),
            Err(PtyDevtoolError::UseBackgroundJob)
        );
    }
    fs::remove_dir_all(root).unwrap();
}
