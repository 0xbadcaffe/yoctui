use super::*;

#[test]
fn junit_popup_editor_routes_selection_navigation_and_clipboard_actions() {
    let result = test_results_record("candidate", "candidate", &[]);
    let mut app = test_workflow_app();
    app.result_tool_capability = ResultToolCapability::Available("/workspace/resulttool".into());
    load_test_results(&mut app, vec![result], Vec::new());
    let _ = update(&mut app, Action::BeginTestJunitExport);
    let _ = update(&mut app, Action::SelectTestJunitDestination);
    let _ = update(&mut app, Action::AppendTestJunitTomlEditor('x'));
    assert!(matches!(
        update(&mut app, Action::CopyTestJunitTomlEditor),
        Some(Effect::CopyToClipboard(value)) if value.contains("destination")
    ));
    let _ = update(&mut app, Action::MoveTestJunitTomlEditorHome);
    let _ = update(&mut app, Action::MoveTestJunitTomlEditorEnd);
    let _ = update(&mut app, Action::PasteTestJunitTomlEditor);
    let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog() else {
        panic!("JUnit editor");
    };
    assert!(
        editor
            .text
            .contains("destination = \"x\"destination = \"x\"")
    );
}
