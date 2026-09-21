use super::*;

#[test]
fn pty_menuconfig_previews_exact_recipe_kernel_uboot_and_devshell_argv() {
    let (root, router, kernel, uboot) = fixture();
    let cases = [
        (
            PtyMenuconfigAction::KernelMenuconfig,
            kernel.clone(),
            "menuconfig",
            PtySessionKind::Menuconfig,
        ),
        (
            PtyMenuconfigAction::UBootMenuconfig,
            uboot,
            "menuconfig",
            PtySessionKind::Menuconfig,
        ),
        (
            PtyMenuconfigAction::RecipeTask {
                recipe: kernel.clone(),
                task: PtyBitBakeInteractiveTask::Devshell,
            },
            kernel.clone(),
            "devshell",
            PtySessionKind::Devshell,
        ),
        (
            PtyMenuconfigAction::RecipeTask {
                recipe: kernel.clone(),
                task: PtyBitBakeInteractiveTask::Nconfig,
            },
            kernel,
            "nconfig",
            PtySessionKind::Menuconfig,
        ),
    ];
    for (action, recipe, task, kind) in cases {
        let preview = router.preview(action).unwrap();
        assert_eq!(preview.command.arguments, vec!["-c", task, &recipe.name]);
        assert_eq!(preview.kind, kind);
        assert_eq!(preview.cwd, fs::canonicalize(root.join("build")).unwrap());
    }
    fs::remove_dir_all(root).unwrap();
}
