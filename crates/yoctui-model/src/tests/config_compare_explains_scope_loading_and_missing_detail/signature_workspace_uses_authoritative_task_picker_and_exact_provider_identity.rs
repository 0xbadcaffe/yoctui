use super::*;

#[test]
fn signature_workspace_uses_authoritative_task_picker_and_exact_provider_identity() {
    let provider = PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb");
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Inspector;
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some(provider.clone()),
        ..Recipe::default()
    });

    let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
    assert_eq!(
        app.notification.as_deref(),
        Some("Load authoritative recipe tasks with Enter before inspecting signatures.")
    );
    app.notification = None;
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec![
                "do_fetch".into(),
                "bad task".into(),
                "do_compile".into(),
                "do_compile".into(),
            ]),
            ..RecipeMetadata::default()
        },
    );

    let _ = update(&mut app, Action::BeginSelectedRecipeSignatures);
    let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog() else {
        panic!("signature task picker was not opened");
    };
    assert_eq!(picker.recipe.name, "busybox");
    assert_eq!(picker.recipe.file, provider);
    assert_eq!(picker.tasks, ["do_compile", "do_fetch"]);
    assert_eq!(app.focus, FocusTarget::Dialog);

    let _ = update(&mut app, Action::SelectSignatureTask { delta: 1 });
    assert_eq!(
        update(&mut app, Action::ConfirmSignatureTask),
        Some(Effect::GetSignatureDump(SignatureTarget {
            recipe: "busybox".into(),
            task: "do_fetch".into(),
        }))
    );
    assert_eq!(app.screen, Screen::Signatures);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(app.active_dialog().is_none());
    assert_eq!(
        update(&mut app, Action::LeaveSignatureWorkspace),
        Some(Effect::CancelSignatureOperation)
    );

    let target = SignatureTarget {
        recipe: "busybox".into(),
        task: "do_fetch".into(),
    };
    let record = signature_record(
        "busybox",
        "do_fetch",
        "aaa",
        "/build/tmp/stamps/busybox/do_fetch.sigdata.aaa",
    );
    let _ = update(
        &mut app,
        Action::SignatureDumpLoaded {
            target,
            records: vec![record],
        },
    );
    assert_eq!(
        update(&mut app, Action::OpenSignatureProvider),
        Some(Effect::OpenInEditor(provider))
    );
    let _ = update(&mut app, Action::LeaveSignatureWorkspace);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 0);
}
