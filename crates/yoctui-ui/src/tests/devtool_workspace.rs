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

#[test]
fn devtool_patch_picker_and_confirmation_render_exact_configured_layer_argv() {
    let identity = yoctui_model::RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta-core/recipes-core/busybox/busybox.bb".into(),
    };
    let layer = yoctui_model::Layer {
        name: "meta-custom".into(),
        path: "/layers/meta-custom".into(),
        priority: Some(8),
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Devtool;
    app.dialogs.push_back(Dialog::DevtoolPatchPicker(
        yoctui_model::DevtoolPatchPicker {
            identity: identity.clone(),
            layers: vec![layer.clone()],
            selection: 0,
        },
    ));
    let picker = rendered_text(&app, 120, 35);
    assert!(picker.contains("Create/update patches for busybox"));
    assert!(picker.contains("meta-custom"));

    app.dialogs.clear();
    app.dialogs.push_back(Dialog::DevtoolPatchConfirmation(
        yoctui_model::DevtoolPatchPlan { identity, layer },
    ));
    let confirmation = rendered_text(&app, 120, 35);
    assert!(confirmation.contains("Confirm patch installation"));
    assert!(
        confirmation
            .contains("devtool update-recipe --mode patch --append /layers/meta-custom busybox")
    );
}
