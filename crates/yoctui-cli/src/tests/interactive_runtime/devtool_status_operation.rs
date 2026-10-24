use super::*;

#[test]
fn workbench_devtool_modify_handoff_stays_with_its_bound_recipe_and_build() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    update(
        &mut app,
        Action::Workbench(yoctui_model::WorkbenchAction::OpenRecipe {
            recipe: "demo".into(),
            root: "/source".into(),
        }),
    );
    let identity = RecipeIdentity {
        name: "demo".into(),
        file: "/layer/demo.bb".into(),
    };
    assert!(should_open_modified_workspace(&app, &identity));
    app.workbench.menu = Some(0);
    assert!(!should_open_modified_workspace(&app, &identity));
    app.workbench.menu = None;
    app.workbench.current_mut().unwrap().recipe = Some("other".into());
    assert!(!should_open_modified_workspace(&app, &identity));
    app.workbench.current_mut().unwrap().recipe = Some("demo".into());
    app.workspace.build_dir = Some("/another-build".into());
    assert!(!should_open_modified_workspace(&app, &identity));
    app.screen = Screen::Recipes;
    assert!(should_open_modified_workspace(&app, &identity));
}

#[tokio::test]
async fn devtool_status_deadline_bounds_busy_bitbake_without_fabricating_nonmembership() {
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        status_with_deadline(std::future::pending(), Duration::from_millis(10)),
    )
    .await
    .unwrap();
    assert!(
        result
            .unwrap_err()
            .contains("Existing workspace status was retained")
    );
}
