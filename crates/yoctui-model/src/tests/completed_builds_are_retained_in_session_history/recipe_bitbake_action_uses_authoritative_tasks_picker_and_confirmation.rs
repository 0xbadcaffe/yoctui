use super::*;

#[test]
fn recipe_bitbake_action_uses_authoritative_tasks_picker_and_confirmation() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(
                [
                    "do_clean",
                    "do_cleansstate",
                    "do_devshell",
                    "do_diffconfig",
                    "do_diffsigs",
                    "do_menuconfig",
                ]
                .into_iter()
                .map(str::to_owned)
                .collect(),
            ),
            ..RecipeMetadata::default()
        },
    );

    let actions = [
        (Action::BeginSelectedRecipeClean, "clean"),
        (Action::BeginSelectedRecipeCleanState, "cleansstate"),
        (Action::BeginSelectedRecipeDiffconfig, "diffconfig"),
        (Action::BeginSelectedRecipeDiffsigs, "diffsigs"),
    ];
    for (action, expected) in actions {
        app.dialogs.clear();
        let _ = update(&mut app, action);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(BuildRequest {
                targets,
                task: Some(task),
                force: false,
            })) if targets == &vec!["busybox".to_owned()] && task == expected
        ));
    }
    for (action, expected) in [
        (Action::BeginSelectedRecipeMenuConfig, "menuconfig"),
        (Action::BeginSelectedRecipeDevshell, "devshell"),
    ] {
        app.dialogs.clear();
        assert_eq!(update(&mut app, action), None);
        assert!(matches!(
            app.active_dialog(),
            Some(Dialog::TerminalLaunch(TerminalLaunchDialog { request, destination: TerminalLaunchDestination::Embedded, .. }))
                if request.name == format!("{expected}:busybox")
        ));
    }

    app.dialogs.clear();
    let _ = update(&mut app, Action::BeginSelectedRecipeForceTask);
    let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog() else {
        panic!("authoritative task picker did not open");
    };
    assert!(picker.force);
    assert_eq!(picker.tasks[0], "clean");
    let _ = update(&mut app, Action::SelectRecipeTask { delta: 3 });
    let _ = update(&mut app, Action::PreviewSelectedRecipeTask);
    let Some(Dialog::RecipeTaskConfirmation(request)) = app.active_dialog() else {
        panic!("forced task was not previewed");
    };
    assert!(request.force);
    assert_eq!(request.targets, vec!["busybox"]);
    let request = request.clone();
    assert_eq!(
        update(&mut app, Action::ConfirmRecipeTask),
        Some(Effect::Start(request))
    );
}
