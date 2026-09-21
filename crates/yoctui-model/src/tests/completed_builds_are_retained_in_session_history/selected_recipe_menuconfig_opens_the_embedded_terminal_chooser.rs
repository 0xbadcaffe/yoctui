use super::*;

#[test]
fn selected_recipe_menuconfig_opens_the_embedded_terminal_chooser() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_menuconfig".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TerminalLaunch(TerminalLaunchDialog {
            request: TerminalLaunchRequest {
                name,
                kind: TerminalCreationKind::Menuconfig,
                ..
            },
            destination: TerminalLaunchDestination::Embedded,
            ..
        })) if name == "menuconfig:busybox"
    ));
}
