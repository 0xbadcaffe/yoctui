use super::*;

#[test]
fn layer_selection_stays_in_workspace_bounds() {
    let mut app = App::new(10, 1_000);
    app.workspace.layers = vec![
        Layer {
            name: "alpha".into(),
            path: PathBuf::from("/layers/alpha"),
            priority: Some(1),
        },
        Layer {
            name: "beta".into(),
            path: PathBuf::from("/layers/beta"),
            priority: None,
        },
    ];
    let _ = update(&mut app, Action::SelectLayer { delta: 8 });
    assert_eq!(app.layer_selection, 1);
    let _ = update(&mut app, Action::SelectLayer { delta: -8 });
    assert_eq!(app.layer_selection, 0);
}
