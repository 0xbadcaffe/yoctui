use super::*;

#[test]
fn pty_menuconfig_rejects_stale_recipe_and_unavailable_task() {
    let (root, router, kernel, _) = fixture();
    let stale = RecipeIdentity {
        name: kernel.name.clone(),
        file: root.join("source/other.bb"),
    };
    assert!(matches!(
        router.preview(PtyMenuconfigAction::RecipeTask {
            recipe: stale,
            task: PtyBitBakeInteractiveTask::Menuconfig
        }),
        Err(PtyMenuconfigError::StaleRecipe(_))
    ));
    assert_eq!(
        router.preview(PtyMenuconfigAction::RecipeTask {
            recipe: kernel,
            task: PtyBitBakeInteractiveTask::Xconfig
        }),
        Err(PtyMenuconfigError::TaskUnavailable {
            recipe: "virtual/kernel".into(),
            task: PtyBitBakeInteractiveTask::Xconfig
        })
    );
    fs::remove_dir_all(root).unwrap();
}
