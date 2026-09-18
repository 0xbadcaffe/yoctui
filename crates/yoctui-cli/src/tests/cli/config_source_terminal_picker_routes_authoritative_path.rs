use super::*;

#[test]
fn config_source_terminal_picker_routes_authoritative_path() {
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some("/build".into());
    app.dialogs.push_back(Dialog::ConfigSourcePicker(
        yoctui_model::ConfigSourcePicker {
            identity: VariableIdentity {
                name: "MACHINE".into(),
                recipe: None,
            },
            sources: vec![yoctui_model::ConfigSourceChoice {
                operation: "set".into(),
                path: "conf/local.conf".into(),
                line: Some(12),
            }],
            selection: 0,
        },
    ));
    let effect =
        config_source_picker_action(Input::Enter).and_then(|action| update(&mut app, action));
    assert_eq!(
        effect,
        Some(Effect::OpenInEditor("/build/conf/local.conf".into()))
    );
    assert!(app.active_dialog().is_none());
}
