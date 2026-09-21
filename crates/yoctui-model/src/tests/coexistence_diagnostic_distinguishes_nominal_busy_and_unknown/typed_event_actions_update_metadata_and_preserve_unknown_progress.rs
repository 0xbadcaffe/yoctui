use super::*;

#[test]
fn typed_event_actions_update_metadata_and_preserve_unknown_progress() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "zlib".into(),
                version: None,
                layer: Some("core".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "base-files".into(),
                version: None,
                layer: Some("core".into()),
                ..Recipe::default()
            },
        ]),
    );
    let _ = update(
        &mut app,
        Action::LayersLoaded(vec![Layer {
            name: "core".into(),
            path: "/poky/meta".into(),
            priority: Some(5),
        }]),
    );
    let _ = update(
        &mut app,
        Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            },
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: Some("/build/conf/local.conf:1".into()),
            operations: vec![],
            active_overrides: vec![],
        }),
    );
    let _ = update(
        &mut app,
        Action::RecipeSourcesLoaded {
            recipe: "base-files".into(),
            paths: vec!["/poky/meta/recipes-core/base-files/base-files.bb".into()],
        },
    );
    assert_eq!(app.workspace.recipes[0].name, "base-files");
    assert_eq!(app.workspace.layers[0].path, PathBuf::from("/poky/meta"));
    assert_eq!(app.workspace.variables["MACHINE"], "qemux86-64");
    assert_eq!(
        app.recipe_sources["base-files"][0],
        PathBuf::from("/poky/meta/recipes-core/base-files/base-files.bb")
    );

    let id = TaskId("base-files:do_install".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "base-files".into(),
            task: "do_install".into(),
            progress: None,
            ..TaskInfo::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id: id.clone(),
            progress: None,
        },
    );
    assert_eq!(app.tasks[&id].progress, None);
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id: id.clone(),
            progress: Some(250),
        },
    );
    assert_eq!(app.tasks[&id].progress, Some(100));
}
