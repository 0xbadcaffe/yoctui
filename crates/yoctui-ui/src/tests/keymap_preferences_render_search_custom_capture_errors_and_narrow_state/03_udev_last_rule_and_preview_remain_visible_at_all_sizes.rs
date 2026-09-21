#[test]
fn udev_last_rule_and_preview_remain_visible_at_all_sizes() {
    let mut app = ux_rootfs_ui_app();
    let RootfsCompositionState::Partial { composition, .. } = &mut app.rootfs_composition else {
        unreachable!()
    };
    composition.system_inventory =
        yoctui_model::RootfsAuthority::Available(yoctui_model::RootfsSystemInventory {
            udev_rules: (0..100)
                .map(|index| yoctui_model::RootfsUdevRule {
                    name: format!("{index:03}-test.rules"),
                    logical_path: yoctui_model::RootfsPathIdentity(
                        format!("/etc/udev/rules.d/{index:03}-test.rules").into(),
                    ),
                    masked: false,
                    overridden_by: None,
                    limitation: None,
                    preview: "# first\nSUBSYSTEM==\"tty\"\n".into(),
                    preview_truncated: false,
                })
                .collect(),
            ..Default::default()
        });
    app.images_view = ImagesView::UdevRules;
    yoctui_model::update(&mut app, Action::SelectRootfsUdevRule { delta: isize::MAX });
    assert_eq!(app.rootfs_udev_selection, 99);
    yoctui_model::update(&mut app, Action::ScrollRootfsUdevPreview { delta: 1 });
    for (width, height) in [(160, 50), (100, 30), (80, 24)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("099-test.rules"), "{text}");
        assert!(text.contains("SUBSYSTEM"), "{text}");
        assert!(text.contains("6 udev"), "{text}");
    }
}
