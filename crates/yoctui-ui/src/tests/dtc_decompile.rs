use super::*;

#[test]
fn dtc_decompile_dialog_renders_destination_checkbox_and_browser() {
    let mut app = App::new(10, 1_000);
    let file = yoctui_model::PlatformFile {
        path: "/workspace/kernel/board.dtb".into(),
        root: "/workspace/kernel".into(),
        kind: yoctui_model::PlatformFileKind::Dtb,
        size_bytes: 4,
    };
    app.dialogs
        .push_back(Dialog::DtcDecompile(yoctui_model::DtcDecompileDialog::new(
            yoctui_model::PlatformComponent::Kernel,
            &file,
            "/usr/bin/dtc".into(),
        )));
    let output = rendered_text(&app, 120, 32);
    for expected in [
        "Decompile device tree",
        "board.yoctui.dts",
        "[x] View file after decompilation",
        "b browse folders",
    ] {
        assert!(output.contains(expected), "missing {expected}: {output}");
    }

    let Some(Dialog::DtcDecompile(dialog)) = app.dialogs.front_mut() else {
        unreachable!();
    };
    dialog.browser = Some(yoctui_model::EnvironmentBrowser {
        request: 1,
        loading: false,
        directory: Some(yoctui_model::EnvironmentDirectory {
            path: "/workspace/output".into(),
            children: vec!["/workspace/output/nested".into()],
            init_script: None,
            notice: None,
        }),
        selection: 0,
    });
    let output = rendered_text(&app, 100, 28);
    assert!(output.contains("Choose the folder"));
    assert!(output.contains("nested/"));
    assert!(output.contains("s use folder"));
}
