use super::*;
use crate::{Action, App, update};

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

#[test]
fn numbered_platform_view_actions_select_exact_tabs() {
    let mut app = App::new(8, 1_000);
    let _ = update(&mut app, Action::SetKernelView(PlatformView::DeviceTrees));
    let _ = update(&mut app, Action::SetFirmwareView(PlatformView::DeviceTrees));
    assert_eq!(app.kernel.view, PlatformView::DeviceTrees);
    assert_eq!(app.firmware.view, PlatformView::DeviceTrees);

    let _ = update(&mut app, Action::SetKernelView(PlatformView::Configuration));
    assert_eq!(app.kernel.view, PlatformView::Configuration);
}
