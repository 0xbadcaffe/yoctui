use super::*;

#[test]
fn devtool_workspace_renders_recipe_centered_workflow_and_scp_deployment() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Devtool;
    app.focus = FocusTarget::Workspace;
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "phosphor-state-manager".into(),
        version: Some("1.0".into()),
        layer: Some("meta-phosphor".into()),
        preferred_version: None,
        file: Some(
            "/work/openbmc/meta-phosphor/recipes-phosphor/state/phosphor-state-manager.bb".into(),
        ),
        append_count: Some(0),
    });

    let output = rendered_text(&app, 160, 50);
    for expected in [
        "Devtool Workspace",
        "phosphor-state-manager",
        "Start/refresh workspace",
        "Edit source",
        "Build workspace recipe",
        "Deploy build with SSH/SCP",
        "Create/update patches",
        "Finish into configured layer",
    ] {
        assert!(output.contains(expected), "missing {expected:?}");
    }
}

#[test]
fn devtool_workspace_renders_safely_at_the_supported_minimum() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Devtool;
    let _ = rendered_text(&app, 79, 23);
}

#[test]
fn devtool_workspace_deploy_dialog_names_the_ssh_scp_transport() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Devtool;
    app.dialogs.push_back(Dialog::DevtoolDeployConfirmation(
        yoctui_model::DevtoolDeployPlan {
            identity: yoctui_model::RecipeIdentity {
                name: "phosphor-state-manager".into(),
                file: "/work/meta/recipes/phosphor-state-manager.bb".into(),
            },
            target: "root@bmc".into(),
        },
    ));
    app.focus = FocusTarget::Dialog;

    let output = rendered_text(&app, 120, 35);
    assert!(output.contains("Confirm SSH/SCP deployment"));
    assert!(output.contains("built install tree"));
    assert!(output.contains("devtool deploy-target phosphor-state-manager root@bmc"));
}
