use super::*;
use yoctui_model::{
    DtcDecompileAction, DtcDecompileDialog, EnvironmentBrowser, EnvironmentDirectory,
    PlatformComponent, PlatformFile, PlatformFileKind, TextAreaState,
};

fn dialog() -> DtcDecompileDialog {
    DtcDecompileDialog::new(
        PlatformComponent::Kernel,
        &PlatformFile {
            path: "/workspace/board.dtb".into(),
            root: "/workspace".into(),
            kind: PlatformFileKind::Dtb,
            size_bytes: 4,
        },
        "/usr/bin/dtc".into(),
    )
}

#[test]
fn dtc_decompile_dialog_maps_form_editor_and_browser_keys() {
    let mut state = dialog();
    assert_eq!(
        dtc_decompile_dialog_action(&state, Input::Char('b')),
        Some(Action::DtcDecompile(DtcDecompileAction::Browse))
    );
    assert_eq!(
        dtc_decompile_dialog_action(&state, Input::Char(' ')),
        Some(Action::DtcDecompile(DtcDecompileAction::ToggleViewAfter))
    );
    state.editor = Some(TextAreaState::new(state.output.clone()));
    assert!(matches!(
        dtc_decompile_dialog_action(&state, Input::Char('x')),
        Some(Action::DtcDecompile(DtcDecompileAction::Insert(value))) if value == "x"
    ));
    state.editor = None;
    state.browser = Some(EnvironmentBrowser {
        request: 1,
        loading: false,
        directory: Some(EnvironmentDirectory {
            path: "/workspace".into(),
            children: vec!["/workspace/out".into()],
            init_script: None,
            notice: None,
        }),
        selection: 0,
    });
    assert_eq!(
        dtc_decompile_dialog_action(&state, Input::Char('s')),
        Some(Action::DtcDecompile(DtcDecompileAction::ChooseDirectory))
    );
}
