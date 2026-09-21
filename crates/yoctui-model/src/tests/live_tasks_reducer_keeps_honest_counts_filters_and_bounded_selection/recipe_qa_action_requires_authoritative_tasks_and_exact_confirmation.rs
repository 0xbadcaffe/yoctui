use super::*;

#[test]
fn recipe_qa_action_requires_authoritative_tasks_and_exact_confirmation() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Load selected recipe metadata")
    );
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_cve_check".into(), "do_create_spdx".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeCveCheck);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets,
            task: Some(task),
            force: false,
        })) if targets == &["busybox"] && task == "cve_check"
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmRecipeTask),
        Some(Effect::Start(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cve_check".into()),
            force: false,
        }))
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest {
            task: Some(task),
            ..
        })) if task == "create_spdx"
    ));
    let _ = update(&mut app, Action::CancelRecipeTask);
    app.recipe_metadata.get_mut("busybox").unwrap().tasks = Some(vec![]);
    let _ = update(&mut app, Action::BeginSelectedRecipeSpdx);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Task create_spdx is not reported")
    );
}
