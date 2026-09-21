use super::*;

#[test]
fn view_filters_and_selection_are_independent() {
    let mut state = PlatformWorkbench {
        inventory: PlatformInventoryState::Available(PlatformInventory {
            component: PlatformComponent::Kernel,
            target: "virtual/kernel".into(),
            provider: None,
            tasks: vec![],
            roots: vec!["/work".into()],
            files: vec![
                PlatformFile {
                    path: "/work/.config".into(),
                    root: "/work".into(),
                    kind: PlatformFileKind::DotConfig,
                    size_bytes: 1,
                },
                PlatformFile {
                    path: "/work/a.dts".into(),
                    root: "/work".into(),
                    kind: PlatformFileKind::Dts,
                    size_bytes: 2,
                },
                PlatformFile {
                    path: "/work/b.dtb".into(),
                    root: "/work".into(),
                    kind: PlatformFileKind::Dtb,
                    size_bytes: 3,
                },
            ],
            dtc: None,
            limitations: vec![],
        }),
        ..PlatformWorkbench::default()
    };
    assert_eq!(state.visible_files().count(), 1);
    state.cycle_view();
    state.select(1);
    assert_eq!(
        state.selected_file().map(|file| file.kind),
        Some(PlatformFileKind::Dtb)
    );
    state.cycle_view();
    assert_eq!(
        state.selected_file().map(|file| file.kind),
        Some(PlatformFileKind::DotConfig)
    );
}
