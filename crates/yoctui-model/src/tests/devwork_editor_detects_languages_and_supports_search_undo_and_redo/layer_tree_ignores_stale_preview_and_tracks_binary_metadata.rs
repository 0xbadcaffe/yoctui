use super::*;

#[test]
fn layer_tree_ignores_stale_preview_and_tracks_binary_metadata() {
    let mut app = App::new(10, 1_000);
    let path = PathBuf::from("/layers/meta-demo/image.bin");
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "meta-demo".into(),
            root: "/layers/meta-demo".into(),
            directory: "/layers/meta-demo".into(),
            entries: vec![LayerBrowserEntry {
                path: path.clone(),
                size: Some(100_000),
                git: GitFileState::Untracked,
                ..LayerBrowserEntry::default()
            }],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserPreview {
            path: PathBuf::from("/layers/meta-demo/stale.bb"),
            content: "stale".into(),
            kind: PreviewKind::Text,
            truncated: false,
        },
    );
    assert!(app.layer_browser.as_ref().unwrap().preview.is_empty());
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserPreview {
            path,
            content: String::new(),
            kind: PreviewKind::Binary,
            truncated: true,
        },
    );
    let browser = app.layer_browser.as_ref().unwrap();
    assert_eq!(browser.preview_kind, PreviewKind::Binary);
    assert!(browser.preview_truncated);
}
