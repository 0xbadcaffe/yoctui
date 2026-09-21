use super::*;

#[test]
fn wic_workspace_dialog_is_bounded_modal_and_stale_safe() {
    let mut app = qemu_model_app();
    app.wic_capability = wic_model_capability();
    if let WicCapability::Available { kickstarts, .. } = &mut app.wic_capability {
        kickstarts.push(WicKickstart {
            identity: WicKickstartIdentity {
                name: "configured".into(),
                path: Some("/layers/custom/configured.wks".into()),
            },
            source: "part /boot --source=bootimg-partition".into(),
            partitions: Vec::new(),
            limitations: Vec::new(),
        });
    }
    app.workspace
        .variables
        .insert("WKS_FILE".into(), "/layers/custom/configured.wks".into());
    let _ = update(&mut app, Action::BeginSelectedWicCreate);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCreateTomlEditor { editor, .. })
            if !editor.editing
                && editor.text.contains("kickstart = \"configured\"")
                && editor.selected_text() == Some("/build/tmp/deploy/images/qemux86-64")
    ));
    if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "machine = \"qemux86-64\"\nimage = \"core-image-minimal\"\nkickstart = \"configured\"\noutput_directory = \"relative/output\"\ngenerate_bmap = true\ncompression = \"none\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewWicCreate);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCreateTomlEditor {
            validation_error: Some(_),
            ..
        })
    ));
    let _ = update(&mut app, Action::CancelWicCreate);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Navigator);

    let _ = update(&mut app, Action::BeginSelectedWicCreate);
    let _ = update(&mut app, Action::PreviewWicCreate);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::WicCreateConfirmation(_))
    ));
    app.wic_capability = WicCapability::MissingTool;
    assert!(update(&mut app, Action::ConfirmWicCreate).is_none());
    assert!(app.active_wic_session().is_none());
}
