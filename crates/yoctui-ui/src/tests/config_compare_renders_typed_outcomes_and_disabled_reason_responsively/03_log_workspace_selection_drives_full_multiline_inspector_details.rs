#[test]
fn log_workspace_selection_drives_full_multiline_inspector_details() {
    let mut app = App::new(20, 4_000);
    app.screen = Screen::Logs;
    app.focus = FocusTarget::Workspace;
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Error,
        message: "compile failed\ncompiler context line".into(),
        recipe: Some("busybox".into()),
        task: Some("do_compile".into()),
        path: Some("/tmp/log.do_compile".into()),
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: true,
        diagnostic: None,
    });
    app.logs.insert(yoctui_model::LogEntry {
        id: 0,
        severity: Severity::Info,
        message: "later output".into(),
        recipe: None,
        task: None,
        path: None,
        timestamp: SystemTime::UNIX_EPOCH,
        build: Some("core-image-minimal".into()),
        protected: false,
        diagnostic: None,
    });
    app.logs.follow = false;
    app.logs.selection = 0;
    let output = rendered_text(&app, 180, 34);
    assert!(output.contains("do_compile"), "{output}");
    assert!(output.contains("Source: /tmp/log.do_compile"), "{output}");
    assert!(output.contains("compiler context line"), "{output}");
    assert!(output.contains("Build: core-image-minimal"), "{output}");
}
