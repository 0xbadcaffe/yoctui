use super::*;

#[test]
fn client_runtime_qa_task_maps_capability_inspection_to_typed_daemon_input() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "busybox".into(),
        file: Some("/layers/busybox.bb".into()),
        ..Default::default()
    });
    let effect = Effect::Qa(yoctui_model::QaEffect::InspectCapability { scope: None });
    let Some(DaemonCommand::InspectQaCapability { request }) =
        daemon_command_for_effect(&app, &effect).unwrap()
    else {
        panic!("expected typed QA capability command");
    };
    assert_eq!(request.input.build_directory, "/build");
    assert_eq!(request.input.selected_recipe_name, "busybox");
    assert_eq!(request.input.recipe_names, vec!["busybox"]);
}
