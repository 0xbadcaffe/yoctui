use super::*;

#[test]
fn selected_layer_opens_the_in_tui_workspace_editor() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers = vec![Layer {
        name: "meta-demo".into(),
        path: PathBuf::from("/layers/meta-demo"),
        priority: None,
    }];
    assert_eq!(
        update(&mut app, Action::BeginSelectedLayerWorkspaceEditor),
        Some(Effect::OpenWorkspaceEditor {
            label: "Layer: meta-demo".into(),
            root: PathBuf::from("/layers/meta-demo"),
        })
    );
}
