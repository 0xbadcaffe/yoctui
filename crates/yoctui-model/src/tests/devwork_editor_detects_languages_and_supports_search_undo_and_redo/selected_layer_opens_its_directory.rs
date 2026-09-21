use super::*;

#[test]
fn selected_layer_opens_its_directory() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers = vec![Layer {
        name: "meta-demo".into(),
        path: PathBuf::from("/layers/meta-demo"),
        priority: None,
    }];
    assert_eq!(
        update(&mut app, Action::OpenSelectedLayer),
        Some(Effect::OpenInEditor(PathBuf::from("/layers/meta-demo")))
    );
}
