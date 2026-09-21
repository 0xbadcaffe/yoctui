use super::*;

#[test]
fn layer_file_right_focuses_preview_and_arrows_scroll_only_the_preview() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserDirectory {
            layer: "rootfs".into(),
            root: "/build/rootfs".into(),
            directory: "/build/rootfs".into(),
            entries: vec![LayerBrowserEntry {
                path: "/build/rootfs/etc/os-release".into(),
                ..LayerBrowserEntry::default()
            }],
        },
    );
    let _ = update(
        &mut app,
        Action::LoadLayerBrowserPreview {
            path: "/build/rootfs/etc/os-release".into(),
            content: "one\ntwo\nthree".into(),
            kind: PreviewKind::Text,
            truncated: false,
        },
    );
    assert_eq!(update(&mut app, Action::LayerBrowserExpand), None);
    assert!(app.layer_browser.as_ref().unwrap().preview_focused);
    let _ = update(&mut app, Action::ScrollLayerBrowserPreview { delta: 1 });
    assert_eq!(app.layer_browser.as_ref().unwrap().preview_scroll, 1);
    let _ = update(&mut app, Action::FocusLayerBrowserTree);
    assert!(!app.layer_browser.as_ref().unwrap().preview_focused);
}
